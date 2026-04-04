use anyhow::{anyhow, Result};
use shared_types::{HitId, SearchField, SearchHit, SearchRequest, SearchTarget, SearchTerm};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tantivy::collector::{Count, TopDocs};
use tantivy::query::{AllQuery, BooleanQuery, Occur, Query, QueryParser, RegexQuery, TermQuery};
use tantivy::schema::{
    Field, IndexRecordOption, NumericOptions, Schema, TantivyDocument, TextFieldIndexing,
    TextOptions, Value, FAST, INDEXED, STORED,
};
use tantivy::{doc, Index, IndexReader, IndexWriter, ReloadPolicy, Term};

#[derive(Clone)]
pub struct TantivySearchIndex {
    pub index: Index,
    reader: IndexReader,
    writer: Arc<Mutex<IndexWriter>>,
    fields: TantivyFields,
}

#[derive(Clone)]
pub struct TantivyFields {
    // ID fields (mutually exclusive)
    pub email_id: Field,
    pub file_id: Field,
    // Universal fields
    pub body_text: Field,
    pub filename: Field,
    pub date_received: Field,
    // Email-specific fields
    pub subject: Field,
    pub from_address: Field,
    pub to_addresses: Field,
    // Filtering
    pub credential_id: Field,
}

/// Text fields extracted for indexing
#[derive(Debug, Clone)]
pub struct IndexedTextFields {
    pub body_text: Option<String>,
    pub filename: Option<String>,
    pub subject: Option<String>,
    pub from_address: Option<String>,
    /// Space-separated lowercase email addresses from to + cc fields.
    pub to_addresses: Option<String>,
    pub credential_id: Option<i64>,
}

impl IndexedTextFields {
    /// Create from email data.
    /// `to_addresses` should be a space-separated string of all recipient addresses.
    pub fn from_email(
        body_text: Option<String>,
        subject: Option<String>,
        from_address: String,
        to_addresses: Option<String>,
        credential_id: i64,
    ) -> Self {
        Self {
            body_text,
            filename: None,
            subject,
            from_address: Some(from_address),
            to_addresses,
            credential_id: Some(credential_id),
        }
    }
}

/// Result of a search operation
#[derive(Debug, Clone)]
pub struct TantivySearchResult {
    pub hits: Vec<SearchHit>,
    pub total_hits: usize,
}

pub fn build_schema() -> Schema {
    let mut builder = Schema::builder();

    let exact_text = TextOptions::default().set_indexing_options(
        TextFieldIndexing::default()
            .set_tokenizer("raw")
            .set_index_option(IndexRecordOption::WithFreqsAndPositions),
    );

    let free_text = TextOptions::default().set_indexing_options(
        TextFieldIndexing::default()
            .set_tokenizer("en_stem")
            .set_index_option(IndexRecordOption::WithFreqsAndPositions),
    );

    // ID fields (0 means not set for that type)
    builder.add_u64_field("email_id", INDEXED | STORED | FAST);
    builder.add_u64_field("file_id", INDEXED | STORED | FAST);

    // Universal fields
    builder.add_text_field("body_text", free_text.clone());
    builder.add_text_field("filename", free_text.clone());

    // Email-specific fields
    builder.add_text_field("subject", free_text.clone());
    builder.add_text_field("from_address", exact_text.clone());
    builder.add_text_field("to_addresses", exact_text.clone());

    // Filtering
    builder.add_u64_field("credential_id", INDEXED | FAST);

    // Date field
    let i64_fast = NumericOptions::default().set_indexed().set_fast();
    builder.add_i64_field("date_received", i64_fast);

    builder.build()
}

fn fields_from_schema(schema: &Schema) -> Result<TantivyFields> {
    let get = |name: &str| -> Result<Field> {
        schema
            .get_field(name)
            .map_err(|_| anyhow!("Missing tantivy field: {name}"))
    };

    Ok(TantivyFields {
        email_id: get("email_id")?,
        file_id: get("file_id")?,
        body_text: get("body_text")?,
        filename: get("filename")?,
        subject: get("subject")?,
        from_address: get("from_address")?,
        to_addresses: get("to_addresses")?,
        credential_id: get("credential_id")?,
        date_received: get("date_received")?,
    })
}

pub fn open_or_create_index(path: &Path) -> Result<TantivySearchIndex> {
    let schema = build_schema();
    if path.exists() {
        std::fs::remove_dir_all(path)?;
    }
    std::fs::create_dir_all(path)?;
    let index = Index::create_in_dir(path, schema.clone())?;

    let fields = fields_from_schema(&index.schema())?;
    let reader = index
        .reader_builder()
        .reload_policy(ReloadPolicy::OnCommitWithDelay)
        .try_into()?;
    let writer = index.writer(50_000_000)?;

    Ok(TantivySearchIndex {
        index,
        reader,
        writer: Arc::new(Mutex::new(writer)),
        fields,
    })
}

/// Open an existing index if present; create a new one otherwise.
/// Unlike `open_or_create_index`, this function does not wipe existing index contents.
pub fn open_or_create_index_preserving(path: &Path) -> Result<TantivySearchIndex> {
    let schema = build_schema();
    let index = if path.exists() {
        Index::open_in_dir(path)?
    } else {
        std::fs::create_dir_all(path)?;
        Index::create_in_dir(path, schema)?
    };

    let fields = fields_from_schema(&index.schema())?;
    let reader = index
        .reader_builder()
        .reload_policy(ReloadPolicy::OnCommitWithDelay)
        .try_into()?;
    let writer = index.writer(50_000_000)?;

    Ok(TantivySearchIndex {
        index,
        reader,
        writer: Arc::new(Mutex::new(writer)),
        fields,
    })
}

impl TantivySearchIndex {
    fn escape_query_value(value: &str) -> String {
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace(':', "\\:")
    }

    /// Index an email
    pub fn index_email(&self, email_id: i64, extracted: &IndexedTextFields) -> Result<()> {
        let mut writer = self.writer.lock().map_err(|_| anyhow!("poisoned writer"))?;
        self.index_email_with_writer(&mut writer, email_id, extracted)?;
        writer.commit()?;
        self.reader.reload()?;
        Ok(())
    }

    fn index_email_with_writer(
        &self,
        writer: &mut IndexWriter,
        email_id: i64,
        extracted: &IndexedTextFields,
    ) -> Result<()> {
        let from_address = extracted
            .from_address
            .clone()
            .unwrap_or_default()
            .to_lowercase();

        let to_addresses = extracted
            .to_addresses
            .clone()
            .unwrap_or_default()
            .to_lowercase();

        // Delete any existing document with this email_id
        writer.delete_term(Term::from_field_u64(
            self.fields.email_id,
            email_id.max(0) as u64,
        ));

        writer.add_document(doc!(
            self.fields.email_id => email_id.max(0) as u64,
            self.fields.file_id => 0u64,  // Not a file
            self.fields.body_text => extracted.body_text.clone().unwrap_or_default(),
            self.fields.filename => extracted.filename.clone().unwrap_or_default(),
            self.fields.subject => extracted.subject.clone().unwrap_or_default(),
            self.fields.from_address => from_address,
            self.fields.to_addresses => to_addresses,
            self.fields.credential_id => extracted.credential_id.unwrap_or(0).max(0) as u64,
            self.fields.date_received => 0i64,
        ))?;
        Ok(())
    }

    /// Index multiple emails in a batch
    pub fn index_emails(&self, emails: &[(i64, IndexedTextFields)]) -> Result<()> {
        let mut writer = self.writer.lock().map_err(|_| anyhow!("poisoned writer"))?;
        for (email_id, extracted) in emails {
            self.index_email_with_writer(&mut writer, *email_id, extracted)?;
        }
        writer.commit()?;
        self.reader.reload()?;
        Ok(())
    }

    /// Delete an email from the index
    pub fn delete_email(&self, email_id: i64) -> Result<()> {
        let mut writer = self.writer.lock().map_err(|_| anyhow!("poisoned writer"))?;
        writer.delete_term(Term::from_field_u64(
            self.fields.email_id,
            email_id.max(0) as u64,
        ));
        writer.commit()?;
        self.reader.reload()?;
        Ok(())
    }

    fn build_term_query(&self, term: &SearchTerm) -> Result<Box<dyn Query>> {
        if matches!(term.field, SearchField::FromAddress) {
            let lowered = term.value.trim().to_lowercase();
            let pattern = format!(".*{}.*", regex::escape(&lowered));
            return Ok(Box::new(RegexQuery::from_pattern(
                &pattern,
                self.fields.from_address,
            )?));
        }

        if matches!(term.field, SearchField::ToAddresses) {
            let lowered = term.value.trim().to_lowercase();
            let pattern = format!(".*{}.*", regex::escape(&lowered));
            return Ok(Box::new(RegexQuery::from_pattern(
                &pattern,
                self.fields.to_addresses,
            )?));
        }

        if matches!(term.field, SearchField::Any) {
            let parser = QueryParser::for_index(
                &self.index,
                vec![
                    self.fields.subject,
                    self.fields.body_text,
                    self.fields.filename,
                ],
            );
            let escaped = Self::escape_query_value(&term.value);
            let query_str = if term.is_phrase {
                format!("\"{escaped}\"")
            } else {
                escaped
            };
            let text_query = parser.parse_query(&query_str)?;

            let lowered = term.value.trim().to_lowercase();
            let pattern = format!(".*{}.*", regex::escape(&lowered));
            let from_query = RegexQuery::from_pattern(&pattern, self.fields.from_address)?;
            let to_query = RegexQuery::from_pattern(&pattern, self.fields.to_addresses)?;

            return Ok(Box::new(BooleanQuery::new(vec![
                (Occur::Should, text_query),
                (Occur::Should, Box::new(from_query)),
                (Occur::Should, Box::new(to_query)),
            ])));
        }

        let target_fields = match term.field {
            SearchField::Any => vec![
                self.fields.subject,
                self.fields.body_text,
                self.fields.filename,
            ],
            SearchField::Subject => vec![self.fields.subject],
            SearchField::FromAddress => vec![self.fields.from_address],
            SearchField::ToAddresses => vec![self.fields.to_addresses],
            SearchField::BodyText => vec![self.fields.body_text],
            SearchField::Filename => vec![self.fields.filename],
        };

        let parser = QueryParser::for_index(&self.index, target_fields);
        let escaped = Self::escape_query_value(&term.value);
        let query_str = if term.is_phrase {
            format!("\"{escaped}\"")
        } else {
            escaped
        };

        Ok(parser.parse_query(&query_str)?)
    }

    pub fn search(&self, request: &SearchRequest) -> Result<TantivySearchResult> {
        if request.terms.is_empty() {
            return Err(anyhow!("terms must not be empty"));
        }

        let limit = request.limit.unwrap_or(25);
        if limit > 100 {
            return Err(anyhow!("limit must be <= 100"));
        }

        let offset = request.offset.unwrap_or(0);
        let searcher = self.reader.searcher();

        let mut must_clauses: Vec<(Occur, Box<dyn Query>)> = request
            .terms
            .iter()
            .map(|t| self.build_term_query(t).map(|q| (Occur::Must, q)))
            .collect::<Result<Vec<_>>>()?;

        // Filter by target type
        match request.target {
            SearchTarget::Email => {
                must_clauses.push((
                    Occur::Must,
                    Box::new(TermQuery::new(
                        Term::from_field_u64(self.fields.file_id, 0u64),
                        IndexRecordOption::Basic,
                    )),
                ));
            }
            SearchTarget::File => {
                must_clauses.push((
                    Occur::Must,
                    Box::new(TermQuery::new(
                        Term::from_field_u64(self.fields.email_id, 0u64),
                        IndexRecordOption::Basic,
                    )),
                ));
            }
            SearchTarget::All => {
                // No filter - include both emails and files
            }
        }

        if let Some(credential_id) = request.credential_id {
            must_clauses.push((
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_u64(self.fields.credential_id, credential_id.max(0) as u64),
                    IndexRecordOption::Basic,
                )),
            ));
        }

        let query: Box<dyn Query> = if must_clauses.is_empty() {
            Box::new(AllQuery)
        } else {
            Box::new(BooleanQuery::new(must_clauses))
        };

        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit).and_offset(offset))?;
        let total_hits = searcher.search(&query, &Count)?;

        let mut hits = Vec::with_capacity(top_docs.len());
        for (score, addr) in top_docs {
            let retrieved: TantivyDocument = searcher.doc(addr)?;

            // Determine if this is an email or file hit
            let email_id = retrieved
                .get_first(self.fields.email_id)
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let file_id = retrieved
                .get_first(self.fields.file_id)
                .and_then(|v| v.as_u64())
                .unwrap_or(0);

            let hit_id = if email_id > 0 {
                HitId::Email(email_id as i64)
            } else if file_id > 0 {
                HitId::File(file_id as i64)
            } else {
                // Skip invalid hits
                continue;
            };

            let snippet = retrieved
                .get_first(self.fields.body_text)
                .and_then(|v| v.as_str())
                .map(|s| s.chars().take(160).collect::<String>())
                .filter(|s| !s.is_empty());

            let subject = retrieved
                .get_first(self.fields.subject)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .filter(|s| !s.is_empty());

            let filename = retrieved
                .get_first(self.fields.filename)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .filter(|s| !s.is_empty());

            let from_address = retrieved
                .get_first(self.fields.from_address)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .filter(|s| !s.is_empty());

            let date_received = retrieved
                .get_first(self.fields.date_received)
                .and_then(|v| v.as_i64());

            hits.push(SearchHit {
                hit_id,
                score,
                snippet,
                subject,
                filename,
                from_address,
                date_received,
            });
        }

        Ok(TantivySearchResult { hits, total_hits })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn any_term(value: &str) -> SearchTerm {
        SearchTerm {
            field: SearchField::Any,
            value: value.to_string(),
            is_phrase: false,
        }
    }

    #[test]
    fn search_target_email_returns_indexed_email_hits() {
        let dir = tempdir().expect("temp dir");
        let index = open_or_create_index(dir.path()).expect("index");

        let extracted = IndexedTextFields::from_email(
            Some("invoice from acme".to_string()),
            Some("Acme invoice".to_string()),
            "billing@acme.com".to_string(),
            Some("user@example.com".to_string()),
            42,
        );
        index.index_email(101, &extracted).expect("index email");

        let request = SearchRequest {
            target: SearchTarget::Email,
            terms: vec![any_term("invoice")],
            credential_id: Some(42),
            limit: Some(10),
            offset: Some(0),
        };

        let result = index.search(&request).expect("search");
        assert_eq!(result.total_hits, 1);
        assert!(matches!(result.hits[0].hit_id, HitId::Email(101)));
    }

    #[test]
    fn search_target_file_excludes_indexed_email_hits() {
        let dir = tempdir().expect("temp dir");
        let index = open_or_create_index(dir.path()).expect("index");

        let extracted = IndexedTextFields::from_email(
            Some("invoice from acme".to_string()),
            Some("Acme invoice".to_string()),
            "billing@acme.com".to_string(),
            Some("user@example.com".to_string()),
            42,
        );
        index.index_email(101, &extracted).expect("index email");

        let request = SearchRequest {
            target: SearchTarget::File,
            terms: vec![any_term("invoice")],
            credential_id: Some(42),
            limit: Some(10),
            offset: Some(0),
        };

        let result = index.search(&request).expect("search");
        assert_eq!(result.total_hits, 0);
        assert!(result.hits.is_empty());
    }
}
