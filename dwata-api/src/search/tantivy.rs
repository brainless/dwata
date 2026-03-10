use anyhow::{anyhow, Result};
use shared_types::{
    Document, DocumentKind, SearchDocumentsRequest, SearchField, SearchHit, SearchTerm,
};
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
    pub document_id: Field,
    pub kind: Field,
    pub source_id: Field,
    pub credential_id: Field,
    pub title: Field,
    pub from_address: Field,
    pub body_text: Field,
    pub attachment_text: Field,
    pub date_received: Field,
    pub date_modified: Field,
    pub indexed_at: Field,
}

#[derive(Debug, Clone)]
pub struct IndexedTextFields {
    pub title: Option<String>,
    pub from_address: Option<String>,
    pub body_text: Option<String>,
    pub attachment_text: Option<String>,
    pub credential_id: Option<i64>,
}

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

    builder.add_u64_field("document_id", INDEXED | STORED | FAST);
    builder.add_text_field("kind", exact_text.clone());
    builder.add_u64_field("source_id", INDEXED | FAST);
    builder.add_u64_field("credential_id", INDEXED | FAST);
    builder.add_text_field("title", free_text.clone());
    builder.add_text_field("from_address", exact_text.clone());
    builder.add_text_field("body_text", free_text.clone());
    builder.add_text_field("attachment_text", free_text);

    let i64_fast = NumericOptions::default().set_indexed().set_fast();
    builder.add_i64_field("date_received", i64_fast.clone());
    builder.add_i64_field("date_modified", i64_fast.clone());
    builder.add_i64_field("indexed_at", i64_fast);

    builder.build()
}

fn kind_to_str(kind: &DocumentKind) -> &'static str {
    match kind {
        DocumentKind::Email => "email",
        DocumentKind::Attachment => "attachment",
        DocumentKind::File => "file",
    }
}

fn fields_from_schema(schema: &Schema) -> Result<TantivyFields> {
    let get = |name: &str| -> Result<Field> {
        schema
            .get_field(name)
            .map_err(|_| anyhow!("Missing tantivy field: {name}"))
    };

    Ok(TantivyFields {
        document_id: get("document_id")?,
        kind: get("kind")?,
        source_id: get("source_id")?,
        credential_id: get("credential_id")?,
        title: get("title")?,
        from_address: get("from_address")?,
        body_text: get("body_text")?,
        attachment_text: get("attachment_text")?,
        date_received: get("date_received")?,
        date_modified: get("date_modified")?,
        indexed_at: get("indexed_at")?,
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

impl TantivySearchIndex {
    fn escape_query_value(value: &str) -> String {
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace(':', "\\:")
    }

    pub fn index_document(&self, doc_row: &Document, extracted: &IndexedTextFields) -> Result<()> {
        let mut writer = self.writer.lock().map_err(|_| anyhow!("poisoned writer"))?;
        self.index_document_with_writer(&mut writer, doc_row, extracted)?;
        writer.commit()?;
        self.reader.reload()?;
        Ok(())
    }

    fn index_document_with_writer(
        &self,
        writer: &mut IndexWriter,
        doc_row: &Document,
        extracted: &IndexedTextFields,
    ) -> Result<()> {
        let from_address = extracted
            .from_address
            .clone()
            .unwrap_or_default()
            .to_lowercase();
        writer.delete_term(Term::from_field_u64(
            self.fields.document_id,
            doc_row.id.max(0) as u64,
        ));
        writer.add_document(doc!(
            self.fields.document_id => doc_row.id.max(0) as u64,
            self.fields.kind => kind_to_str(&doc_row.kind),
            self.fields.source_id => doc_row.source_id.max(0) as u64,
            self.fields.credential_id => extracted.credential_id.unwrap_or(0).max(0) as u64,
            self.fields.title => extracted.title.clone().or_else(|| doc_row.title.clone()).unwrap_or_default(),
            self.fields.from_address => from_address,
            self.fields.body_text => extracted.body_text.clone().unwrap_or_default(),
            self.fields.attachment_text => extracted.attachment_text.clone().unwrap_or_default(),
            self.fields.date_received => doc_row.date_received.unwrap_or(0),
            self.fields.date_modified => doc_row.date_modified.unwrap_or(0),
            self.fields.indexed_at => chrono::Utc::now().timestamp_millis(),
        ))?;
        Ok(())
    }

    pub fn index_documents_page(&self, rows: &[(Document, IndexedTextFields)]) -> Result<()> {
        let mut writer = self.writer.lock().map_err(|_| anyhow!("poisoned writer"))?;
        for (doc_row, extracted) in rows {
            self.index_document_with_writer(&mut writer, doc_row, extracted)?;
        }
        writer.commit()?;
        self.reader.reload()?;
        Ok(())
    }

    pub fn delete_document(&self, document_id: i64) -> Result<()> {
        let mut writer = self.writer.lock().map_err(|_| anyhow!("poisoned writer"))?;
        writer.delete_term(Term::from_field_u64(
            self.fields.document_id,
            document_id.max(0) as u64,
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

        if matches!(term.field, SearchField::Any) {
            let parser = QueryParser::for_index(
                &self.index,
                vec![
                    self.fields.title,
                    self.fields.body_text,
                    self.fields.attachment_text,
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

            return Ok(Box::new(BooleanQuery::new(vec![
                (Occur::Should, text_query),
                (Occur::Should, Box::new(from_query)),
            ])));
        }

        let target_fields = match term.field {
            SearchField::Any => vec![
                self.fields.title,
                self.fields.body_text,
                self.fields.attachment_text,
            ],
            SearchField::Title => vec![self.fields.title],
            SearchField::FromAddress => vec![self.fields.from_address],
            SearchField::BodyText => vec![self.fields.body_text],
            SearchField::AttachmentText => vec![self.fields.attachment_text],
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

    pub fn search(&self, request: &SearchDocumentsRequest) -> Result<TantivySearchResult> {
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

        if let Some(kind) = &request.kind {
            must_clauses.push((
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(self.fields.kind, kind_to_str(kind)),
                    IndexRecordOption::Basic,
                )),
            ));
        }

        if let Some(source_id) = request.source_id {
            must_clauses.push((
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_u64(self.fields.source_id, source_id.max(0) as u64),
                    IndexRecordOption::Basic,
                )),
            ));
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
            let document_id = retrieved
                .get_first(self.fields.document_id)
                .and_then(|v| v.as_u64())
                .ok_or_else(|| anyhow!("document_id missing in index doc"))?
                as i64;

            let snippet = retrieved
                .get_first(self.fields.body_text)
                .and_then(|v| v.as_str())
                .map(|s| s.chars().take(160).collect::<String>())
                .filter(|s| !s.is_empty());

            hits.push(SearchHit {
                document_id,
                score,
                snippet,
            });
        }

        Ok(TantivySearchResult { hits, total_hits })
    }
}
