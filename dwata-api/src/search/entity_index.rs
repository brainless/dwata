use anyhow::{anyhow, Result};
use async_trait::async_trait;
use dwata_agents::entity_search::{
    EntitySearchProvider, EntitySearchResult, NamedEntityKind, SearchEntitiesParams,
};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tantivy::collector::TopDocs;
use tantivy::query::{BooleanQuery, Occur, Query, QueryParser, TermQuery};
use tantivy::schema::{
    Field, IndexRecordOption, Schema, TantivyDocument, TextFieldIndexing, TextOptions, Value, FAST,
    INDEXED, STORED,
};
use tantivy::{doc, Index, IndexReader, IndexWriter, ReloadPolicy, Term};

#[derive(Clone)]
pub struct EntitySearchIndex {
    pub index: Index,
    reader: IndexReader,
    writer: Arc<Mutex<IndexWriter>>,
    fields: EntityIndexFields,
}

#[derive(Clone)]
struct EntityIndexFields {
    pub entity_type: Field,
    pub entity_id: Field,
    pub name: Field,
    pub search_summary: Field,
}

impl EntityIndexFields {
    fn entity_type_value(kind: NamedEntityKind) -> &'static str {
        match kind {
            NamedEntityKind::Location => "location",
            NamedEntityKind::Organisation => "organisation",
            NamedEntityKind::Person => "person",
            NamedEntityKind::Bill => "bill",
            NamedEntityKind::Transaction => "transaction",
            NamedEntityKind::Subscription => "subscription",
            NamedEntityKind::Order => "order",
            NamedEntityKind::Event => "event",
        }
    }
}

fn build_schema() -> Schema {
    let mut builder = Schema::builder();

    let free_text = TextOptions::default().set_indexing_options(
        TextFieldIndexing::default()
            .set_tokenizer("en_stem")
            .set_index_option(IndexRecordOption::WithFreqsAndPositions),
    );

    let exact_text = TextOptions::default().set_stored().set_indexing_options(
        TextFieldIndexing::default()
            .set_tokenizer("raw")
            .set_index_option(IndexRecordOption::WithFreqsAndPositions),
    );

    builder.add_text_field("entity_type", exact_text.clone());
    builder.add_u64_field("entity_id", INDEXED | STORED | FAST);
    builder.add_text_field("name", free_text.clone());
    builder.add_text_field("search_summary", free_text.clone());

    builder.build()
}

fn fields_from_schema(schema: &Schema) -> Result<EntityIndexFields> {
    let get = |name: &str| -> Result<Field> {
        schema
            .get_field(name)
            .map_err(|_| anyhow!("Missing tantivy field: {name}"))
    };

    Ok(EntityIndexFields {
        entity_type: get("entity_type")?,
        entity_id: get("entity_id")?,
        name: get("name")?,
        search_summary: get("search_summary")?,
    })
}

pub fn open_or_create_index(path: &Path) -> Result<EntitySearchIndex> {
    if path.exists() {
        std::fs::remove_dir_all(path)?;
    }
    std::fs::create_dir_all(path)?;

    let schema = build_schema();
    let index = Index::create_in_dir(path, schema.clone())?;

    let fields = fields_from_schema(&index.schema())?;
    let reader = index
        .reader_builder()
        .reload_policy(ReloadPolicy::OnCommitWithDelay)
        .try_into()?;
    let writer = index.writer(50_000_000)?;

    Ok(EntitySearchIndex {
        index,
        reader,
        writer: Arc::new(Mutex::new(writer)),
        fields,
    })
}

impl EntitySearchIndex {
    fn escape_query_value(value: &str) -> String {
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace(':', "\\:")
    }

    pub fn index_entity(
        &self,
        entity_type: NamedEntityKind,
        entity_id: i64,
        name: &str,
        search_summary: Option<&str>,
    ) -> Result<()> {
        let mut writer = self.writer.lock().map_err(|_| anyhow!("poisoned writer"))?;
        self.index_entity_with_writer(&mut writer, entity_type, entity_id, name, search_summary)?;
        writer.commit()?;
        self.reader.reload()?;
        Ok(())
    }

    fn index_entity_with_writer(
        &self,
        writer: &mut IndexWriter,
        entity_type: NamedEntityKind,
        entity_id: i64,
        name: &str,
        search_summary: Option<&str>,
    ) -> Result<()> {
        let type_str = EntityIndexFields::entity_type_value(entity_type);

        writer.delete_term(Term::from_field_u64(
            self.fields.entity_id,
            entity_id as u64,
        ));

        writer.add_document(doc!(
            self.fields.entity_type => type_str,
            self.fields.entity_id => entity_id as u64,
            self.fields.name => name,
            self.fields.search_summary => search_summary.unwrap_or(""),
        ))?;
        Ok(())
    }

    pub fn delete_entity(&self, entity_id: i64) -> Result<()> {
        let mut writer = self.writer.lock().map_err(|_| anyhow!("poisoned writer"))?;
        writer.delete_term(Term::from_field_u64(
            self.fields.entity_id,
            entity_id as u64,
        ));
        writer.commit()?;
        self.reader.reload()?;
        Ok(())
    }

    pub fn search(&self, params: &SearchEntitiesParams) -> Result<Vec<EntitySearchResult>> {
        let limit = params.limit.unwrap_or(5).min(20) as usize;
        let searcher = self.reader.searcher();

        let entity_type_terms: Vec<_> = params
            .entity_types
            .iter()
            .map(|k| EntityIndexFields::entity_type_value(*k))
            .collect();

        let search_text = Self::escape_query_value(&params.keywords);

        let name_query = {
            let parser = QueryParser::for_index(
                &self.index,
                vec![self.fields.name, self.fields.search_summary],
            );
            parser.parse_query(&search_text).ok()
        };

        let results = if entity_type_terms.len() == 1 {
            let type_term = Term::from_field_text(self.fields.entity_type, &entity_type_terms[0]);
            let type_query =
                Box::new(TermQuery::new(type_term, IndexRecordOption::Basic)) as Box<dyn Query>;

            let query: Box<dyn Query> = if let Some(text_query) = name_query {
                Box::new(BooleanQuery::new(vec![
                    (Occur::Must, type_query),
                    (Occur::Should, text_query),
                ]))
            } else {
                type_query
            };

            let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;

            top_docs
                .into_iter()
                .map(|(score, addr)| {
                    let doc: TantivyDocument = searcher.doc(addr)?;
                    Ok((score, doc))
                })
                .collect::<Result<Vec<_>>>()?
        } else if !entity_type_terms.is_empty() {
            let type_queries: Vec<_> = entity_type_terms
                .iter()
                .map(|t| {
                    let term = Term::from_field_text(self.fields.entity_type, t);
                    Box::new(TermQuery::new(term, IndexRecordOption::Basic)) as Box<dyn Query>
                })
                .map(|q| (Occur::Should, q))
                .collect();

            let query: Box<dyn Query> = if let Some(text_query) = name_query {
                Box::new(BooleanQuery::new(vec![
                    (Occur::Must, Box::new(BooleanQuery::new(type_queries))),
                    (Occur::Should, text_query),
                ]))
            } else {
                Box::new(BooleanQuery::new(type_queries))
            };

            let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;
            top_docs
                .into_iter()
                .map(|(score, addr)| {
                    let doc: TantivyDocument = searcher.doc(addr)?;
                    Ok((score, doc))
                })
                .collect::<Result<Vec<_>>>()?
        } else {
            let parser = QueryParser::for_index(
                &self.index,
                vec![self.fields.name, self.fields.search_summary],
            );
            let text_query = parser.parse_query(&search_text)?;
            let top_docs = searcher.search(&*text_query, &TopDocs::with_limit(limit))?;
            top_docs
                .into_iter()
                .map(|(score, addr)| {
                    let doc: TantivyDocument = searcher.doc(addr)?;
                    Ok((score, doc))
                })
                .collect::<Result<Vec<_>>>()?
        };

        results
            .into_iter()
            .map(|(_score, doc)| {
                let type_str = doc
                    .get_first(self.fields.entity_type)
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let entity_id = doc
                    .get_first(self.fields.entity_id)
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as i64;
                let name = doc
                    .get_first(self.fields.name)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let search_summary = doc
                    .get_first(self.fields.search_summary)
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string());

                let kind = match type_str {
                    "location" => NamedEntityKind::Location,
                    "organisation" => NamedEntityKind::Organisation,
                    "person" => NamedEntityKind::Person,
                    "bill" => NamedEntityKind::Bill,
                    "transaction" => NamedEntityKind::Transaction,
                    "subscription" => NamedEntityKind::Subscription,
                    "order" => NamedEntityKind::Order,
                    "event" => NamedEntityKind::Event,
                    other => {
                        tracing::warn!("Skipping Tantivy doc with unrecognised entity_type={:?} (id={}); index may be stale — reindex to fix", other, entity_id);
                        return None;
                    }
                };

                Some(Ok(EntitySearchResult {
                    id: entity_id,
                    entity_type: kind,
                    score: 1.0,
                    name,
                    search_summary,
                }))
            })
            .flatten()
            .collect()
    }
}

pub struct DbEntitySearchProvider {
    pool: Pool<SqliteConnectionManager>,
    index: EntitySearchIndex,
}

impl DbEntitySearchProvider {
    pub fn new(pool: Pool<SqliteConnectionManager>, index: EntitySearchIndex) -> Self {
        Self { pool, index }
    }

    /// Direct SQL lookup by sender email address.
    ///
    /// Queries `organisations.email` for an exact match and `persons.email` for
    /// an exact match, then also tries a domain-prefix match on organisations
    /// (e.g. sender "billing@linode.com" → domain "linode" → LIKE '%linode%').
    /// More reliable than BM25 for finding the same org/person across emails.
    fn lookup_by_sender_email(
        &self,
        sender_email: &str,
        entity_types: &[NamedEntityKind],
    ) -> anyhow::Result<Vec<EntitySearchResult>> {
        let conn = self.pool.get()?;
        let mut results: Vec<EntitySearchResult> = Vec::new();

        let want_orgs = entity_types.contains(&NamedEntityKind::Organisation);
        let want_persons = entity_types.contains(&NamedEntityKind::Person);

        // Extract base domain word for a fuzzy org name fallback
        // e.g. "billing@linode.com" → "linode"
        let domain_base = sender_email
            .split('@')
            .nth(1)
            .and_then(|d| d.split('.').next())
            .filter(|d| d.len() >= 3)
            .map(|d| format!("%{}%", d));

        if want_orgs {
            // Exact email match
            let mut stmt = conn.prepare(
                "SELECT id, name, search_summary FROM organisations WHERE email = ? LIMIT 5",
            )?;
            let rows = stmt.query_map(rusqlite::params![sender_email], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            })?;
            for row in rows.flatten() {
                results.push(EntitySearchResult {
                    id: row.0,
                    entity_type: NamedEntityKind::Organisation,
                    score: 1.0,
                    name: row.1,
                    search_summary: row.2,
                });
            }

            // Domain-prefix match on org name (fallback when email wasn't stored)
            if let Some(ref pattern) = domain_base {
                let seen_ids: std::collections::HashSet<i64> =
                    results.iter().map(|r| r.id).collect();
                let mut stmt = conn.prepare(
                    "SELECT id, name, search_summary FROM organisations \
                     WHERE name LIKE ? LIMIT 5",
                )?;
                let rows = stmt.query_map(rusqlite::params![pattern], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                    ))
                })?;
                for row in rows.flatten() {
                    if !seen_ids.contains(&row.0) {
                        results.push(EntitySearchResult {
                            id: row.0,
                            entity_type: NamedEntityKind::Organisation,
                            score: 0.8,
                            name: row.1,
                            search_summary: row.2,
                        });
                    }
                }
            }
        }

        if want_persons {
            let mut stmt = conn
                .prepare("SELECT id, name, search_summary FROM persons WHERE email = ? LIMIT 5")?;
            let rows = stmt.query_map(rusqlite::params![sender_email], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            })?;
            for row in rows.flatten() {
                results.push(EntitySearchResult {
                    id: row.0,
                    entity_type: NamedEntityKind::Person,
                    score: 1.0,
                    name: row.1,
                    search_summary: row.2,
                });
            }
        }

        Ok(results)
    }
}

#[async_trait]
impl EntitySearchProvider for DbEntitySearchProvider {
    async fn search_entities(
        &self,
        params: &SearchEntitiesParams,
    ) -> anyhow::Result<Vec<EntitySearchResult>> {
        let mut results = self.index.search(params)?;

        if let Some(ref sender_email) = params.sender_email {
            let sender_results = self.lookup_by_sender_email(sender_email, &params.entity_types)?;
            let seen: std::collections::HashSet<i64> = results.iter().map(|r| r.id).collect();
            results.extend(sender_results.into_iter().filter(|r| !seen.contains(&r.id)));
        }

        Ok(results)
    }
}

pub async fn reindex_all_entities(
    pool: &Pool<SqliteConnectionManager>,
    index: &EntitySearchIndex,
) -> Result<()> {
    let conn = pool.get()?;

    let locations_query = "SELECT id, name, search_summary FROM locations WHERE search_summary IS NOT NULL AND search_summary != ''";
    for row in conn.prepare(locations_query)?.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
        ))
    })? {
        if let Ok((id, name, summary)) = row {
            index.index_entity(NamedEntityKind::Location, id, &name, summary.as_deref())?;
        }
    }

    let orgs_query = "SELECT id, name, search_summary FROM organisations WHERE search_summary IS NOT NULL AND search_summary != ''";
    for row in conn.prepare(orgs_query)?.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
        ))
    })? {
        if let Ok((id, name, summary)) = row {
            index.index_entity(NamedEntityKind::Organisation, id, &name, summary.as_deref())?;
        }
    }

    let persons_query = "SELECT id, name, search_summary FROM persons WHERE search_summary IS NOT NULL AND search_summary != ''";
    for row in conn.prepare(persons_query)?.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
        ))
    })? {
        if let Ok((id, name, summary)) = row {
            index.index_entity(NamedEntityKind::Person, id, &name, summary.as_deref())?;
        }
    }

    let subs_query = "SELECT id, service_name FROM subscriptions";
    for row in conn.prepare(subs_query)?.query_map([], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
    })? {
        if let Ok((id, name)) = row {
            index.index_entity(NamedEntityKind::Subscription, id, &name, None)?;
        }
    }

    let orders_query = "SELECT id, COALESCE(order_reference, '') FROM orders";
    for row in conn.prepare(orders_query)?.query_map([], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
    })? {
        if let Ok((id, name)) = row {
            if !name.is_empty() {
                index.index_entity(NamedEntityKind::Order, id, &name, None)?;
            }
        }
    }

    let events_query = "SELECT id, name FROM events";
    for row in conn.prepare(events_query)?.query_map([], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
    })? {
        if let Ok((id, name)) = row {
            index.index_entity(NamedEntityKind::Event, id, &name, None)?;
        }
    }

    Ok(())
}
