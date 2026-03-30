use std::collections::HashMap;

use async_trait::async_trait;
use dwata_agents::entity_search::NamedEntityKind;
use dwata_agents::entity_types::{
    ExtractedBill, ExtractedEvent, ExtractedLocation, ExtractedOrder, ExtractedOrganisation,
    ExtractedPerson, ExtractedSubscription, ExtractedTransaction,
};
use dwata_agents::kg_persistence::KgPersistenceProvider;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::params;

use crate::search::entity_index::EntitySearchIndex;

pub struct KgPersistenceLayer {
    pool: Pool<SqliteConnectionManager>,
    entity_index: Option<EntitySearchIndex>,
}

impl KgPersistenceLayer {
    pub fn new(
        pool: Pool<SqliteConnectionManager>,
        entity_index: Option<EntitySearchIndex>,
    ) -> Self {
        Self { pool, entity_index }
    }

    pub fn with_entity_index(mut self, entity_index: EntitySearchIndex) -> Self {
        self.entity_index = Some(entity_index);
        self
    }

    pub fn pool(&self) -> &Pool<SqliteConnectionManager> {
        &self.pool
    }

    pub fn entity_index(&self) -> Option<&EntitySearchIndex> {
        self.entity_index.as_ref()
    }

    pub fn resolve_fk(&self, id_map: &HashMap<i64, i64>, llm_id: Option<i64>) -> Option<i64> {
        llm_id.and_then(|id| {
            if id > 0 {
                id_map.get(&id).copied()
            } else {
                Some(-id)
            }
        })
    }

    fn parse_date(raw: Option<&str>) -> Option<i64> {
        raw.and_then(|s| {
            dateparser::parse(s).ok().and_then(|dt| {
                let naive_dt = dt.date_naive().and_hms_opt(0, 0, 0)?;
                let utc_dt = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
                    naive_dt,
                    chrono::Utc,
                );
                Some(utc_dt.timestamp_millis())
            })
        })
    }

    fn index_entity(&self, kind: NamedEntityKind, db_id: i64, name: &str, summary: Option<&str>) {
        if let Some(ref idx) = self.entity_index {
            if let Err(e) = idx.index_entity(kind, db_id, name, summary) {
                tracing::warn!(
                    "Failed to index entity {} (id={}): {}",
                    kind.as_str(),
                    db_id,
                    e
                );
            }
        }
    }

    pub fn insert_location(
        &self,
        loc: &ExtractedLocation,
        id_map: &mut HashMap<i64, i64>,
    ) -> anyhow::Result<i64> {
        let conn = self.pool.get()?;
        let now = chrono::Utc::now().timestamp();

        let db_id: i64 = conn.query_row(
            "INSERT INTO locations
             (name, address_line1, address_line2, city, region, country_code, postal_code, search_summary, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             RETURNING id",
            params![
                loc.name.as_ref(),
                loc.address_line1.as_ref(),
                loc.address_line2.as_ref(),
                loc.city.as_ref(),
                loc.region.as_ref(),
                loc.country_code.as_ref(),
                loc.postal_code.as_ref(),
                loc.search_summary.as_ref(),
                now,
                now,
            ],
            |row| row.get(0),
        )?;

        id_map.insert(loc.id, db_id);
        let display_name = loc
            .name
            .as_deref()
            .unwrap_or(loc.city.as_deref().unwrap_or("unknown"));
        self.index_entity(
            NamedEntityKind::Location,
            db_id,
            display_name,
            loc.search_summary.as_deref(),
        );

        Ok(db_id)
    }

    pub fn insert_organisation(
        &self,
        org: &ExtractedOrganisation,
        id_map: &mut HashMap<i64, i64>,
        sender_email: Option<&str>,
    ) -> anyhow::Result<i64> {
        let conn = self.pool.get()?;
        let now = chrono::Utc::now().timestamp();

        let location_db_id = self.resolve_fk(id_map, org.location_id);

        // Backfill: if the LLM left email blank but we know the sender's email and
        // the sender's domain matches this org's name, use the sender email.
        let effective_email = org.email.clone().or_else(|| {
            let sender = sender_email?;
            let domain_base = sender.split('@').nth(1)?.split('.').next()?;
            if domain_base.len() >= 3
                && org
                    .name
                    .to_lowercase()
                    .contains(&domain_base.to_lowercase())
            {
                tracing::debug!(
                    "Backfilling email for org '{}' from sender address '{}'",
                    org.name,
                    sender
                );
                Some(sender.to_string())
            } else {
                None
            }
        });

        // Deduplicate by name: if an org with this name already exists, reuse it.
        let existing_id: Option<i64> = conn
            .query_row(
                "SELECT id FROM organisations WHERE name = ? LIMIT 1",
                params![&org.name],
                |row| row.get(0),
            )
            .ok();

        let db_id = if let Some(id) = existing_id {
            // Update fields that may have been filled in by this extraction.
            conn.execute(
                "UPDATE organisations SET
                     industry    = COALESCE(industry, ?),
                     email       = COALESCE(email, ?),
                     location_id = COALESCE(location_id, ?),
                     website     = COALESCE(website, ?),
                     search_summary = COALESCE(search_summary, ?),
                     updated_at  = ?
                 WHERE id = ?",
                params![
                    org.industry.as_ref(),
                    effective_email.as_ref(),
                    location_db_id,
                    org.website.as_ref(),
                    org.search_summary.as_ref(),
                    now,
                    id,
                ],
            )?;
            id
        } else {
            conn.query_row(
                "INSERT INTO organisations
                 (name, description, industry, email, location_id, website, search_summary, created_at, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                 RETURNING id",
                params![
                    &org.name,
                    Option::<&str>::None,
                    org.industry.as_ref(),
                    effective_email.as_ref(),
                    location_db_id,
                    org.website.as_ref(),
                    org.search_summary.as_ref(),
                    now,
                    now,
                ],
                |row| row.get(0),
            )?
        };

        for role in &org.roles {
            conn.execute(
                "INSERT OR IGNORE INTO organisation_roles (organisation_id, role) VALUES (?, ?)",
                params![db_id, role.to_string()],
            )?;
        }

        id_map.insert(org.id, db_id);
        self.index_entity(
            NamedEntityKind::Organisation,
            db_id,
            &org.name,
            org.search_summary.as_deref(),
        );

        Ok(db_id)
    }

    pub fn insert_person(
        &self,
        person: &ExtractedPerson,
        source_email_id: Option<i64>,
        id_map: &mut HashMap<i64, i64>,
    ) -> anyhow::Result<i64> {
        let conn = self.pool.get()?;
        let now = chrono::Utc::now().timestamp();

        let org_db_id = self.resolve_fk(id_map, person.organisation_id).or_else(|| {
            // Backfill: if the LLM left organisation_id blank but the person's email
            // domain matches a known organisation, link them automatically.
            let email = person.email.as_deref()?;
            let domain = email.split('@').nth(1)?;
            conn.query_row(
                "SELECT id FROM organisations WHERE email LIKE ? LIMIT 1",
                params![format!("%@{}", domain)],
                |row| row.get::<_, i64>(0),
            )
            .ok()
        });

        // Deduplicate by email address when available (unique index enforces this).
        let existing_id: Option<i64> = person.email.as_deref().and_then(|email| {
            conn.query_row(
                "SELECT id FROM persons WHERE email = ? LIMIT 1",
                params![email],
                |row| row.get(0),
            )
            .ok()
        });

        let db_id = if let Some(id) = existing_id {
            // Enrich fields that may be newly available.
            conn.execute(
                "UPDATE persons SET
                     organisation_id = COALESCE(organisation_id, ?),
                     phone           = COALESCE(phone, ?),
                     search_summary  = COALESCE(search_summary, ?),
                     updated_at      = ?
                 WHERE id = ?",
                params![
                    org_db_id,
                    person.phone.as_ref(),
                    person.search_summary.as_ref(),
                    now,
                    id,
                ],
            )?;
            id
        } else {
            conn.query_row(
                "INSERT INTO persons
                 (email_id, name, email, phone, organisation_id, search_summary, created_at, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                 RETURNING id",
                params![
                    source_email_id
                        .or(person.email_id)
                        .filter(|_| person.email_id.is_some()),
                    &person.name,
                    person.email.as_ref(),
                    person.phone.as_ref(),
                    org_db_id,
                    person.search_summary.as_ref(),
                    now,
                    now,
                ],
                |row| row.get(0),
            )?
        };

        id_map.insert(person.id, db_id);
        self.index_entity(
            NamedEntityKind::Person,
            db_id,
            &person.name,
            person.search_summary.as_deref(),
        );

        Ok(db_id)
    }

    pub fn insert_subscription(
        &self,
        sub: &ExtractedSubscription,
        source_email_id: Option<i64>,
        id_map: &mut HashMap<i64, i64>,
    ) -> anyhow::Result<i64> {
        let conn = self.pool.get()?;
        let now = chrono::Utc::now().timestamp();

        let org_db_id = self.resolve_fk(id_map, sub.organisation_id);

        let db_id: i64 = conn.query_row(
            "INSERT INTO subscriptions
             (organisation_id, service_name, plan_name, billing_cycle, amount, currency,
              next_billing_date_raw, next_billing_date, start_date_raw, start_date,
              source_email_id, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             RETURNING id",
            params![
                org_db_id,
                &sub.service_name,
                sub.plan_name.as_ref(),
                sub.billing_cycle.as_ref(),
                sub.amount,
                sub.currency.as_ref(),
                sub.next_billing_date.as_ref(),
                Self::parse_date(sub.next_billing_date.as_deref()),
                sub.start_date.as_ref(),
                Self::parse_date(sub.start_date.as_deref()),
                source_email_id,
                now,
                now,
            ],
            |row| row.get(0),
        )?;

        id_map.insert(sub.id, db_id);
        self.index_entity(
            NamedEntityKind::Subscription,
            db_id,
            &sub.service_name,
            None,
        );

        Ok(db_id)
    }

    pub fn insert_bill(
        &self,
        bill: &ExtractedBill,
        source_email_id: Option<i64>,
        id_map: &mut HashMap<i64, i64>,
    ) -> anyhow::Result<i64> {
        let conn = self.pool.get()?;
        let now = chrono::Utc::now().timestamp();

        let org_db_id = self.resolve_fk(id_map, bill.issuer_organisation_id);
        let sub_db_id = self.resolve_fk(id_map, bill.subscription_id);

        let db_id: i64 = conn.query_row(
            "INSERT INTO bills
             (organisation_id, subscription_id, document_reference, total_amount, currency,
              issued_date_raw, issued_date, due_date_raw, due_date,
              billing_period_start_raw, billing_period_start,
              billing_period_end_raw, billing_period_end,
              source_email_id, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             RETURNING id",
            params![
                org_db_id,
                sub_db_id,
                bill.document_reference.as_ref(),
                bill.total_amount.as_ref(),
                bill.currency.as_ref(),
                bill.issued_date.as_ref(),
                Self::parse_date(bill.issued_date.as_deref()),
                bill.due_date.as_ref(),
                Self::parse_date(bill.due_date.as_deref()),
                bill.billing_period_start.as_ref(),
                Self::parse_date(bill.billing_period_start.as_deref()),
                bill.billing_period_end.as_ref(),
                Self::parse_date(bill.billing_period_end.as_deref()),
                source_email_id,
                now,
                now,
            ],
            |row| row.get(0),
        )?;

        id_map.insert(bill.id, db_id);

        Ok(db_id)
    }

    pub fn insert_transaction(
        &self,
        tx: &ExtractedTransaction,
        source_email_id: Option<i64>,
        id_map: &mut HashMap<i64, i64>,
    ) -> anyhow::Result<i64> {
        let conn = self.pool.get()?;
        let now = chrono::Utc::now().timestamp();

        let payer_db_id = self.resolve_fk(id_map, tx.payer_organisation_id);
        let payee_db_id = self.resolve_fk(id_map, tx.payee_organisation_id);
        let bill_db_id = self.resolve_fk(id_map, tx.bill_id);

        let db_id: i64 = conn.query_row(
            "INSERT INTO transactions
             (amount, currency, transaction_date_raw, transaction_date,
              transaction_reference, payer_organisation_id, payee_organisation_id,
              bill_id, source_email_id, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             RETURNING id",
            params![
                &tx.amount.to_string(),
                &tx.currency,
                tx.transaction_date.as_ref(),
                Self::parse_date(tx.transaction_date.as_deref()),
                tx.transaction_reference.as_ref(),
                payer_db_id,
                payee_db_id,
                bill_db_id,
                source_email_id,
                now,
                now,
            ],
            |row| row.get(0),
        )?;

        id_map.insert(tx.id, db_id);

        Ok(db_id)
    }

    pub fn insert_order(
        &self,
        order: &ExtractedOrder,
        source_email_id: Option<i64>,
        id_map: &mut HashMap<i64, i64>,
    ) -> anyhow::Result<i64> {
        let conn = self.pool.get()?;
        let now = chrono::Utc::now().timestamp();

        let org_db_id = self.resolve_fk(id_map, order.organisation_id);
        let tx_db_id = self.resolve_fk(id_map, order.transaction_id);

        let items_json = order
            .items
            .as_ref()
            .map(|items| {
                serde_json::to_string(
                    &items
                        .iter()
                        .map(|s| serde_json::json!({ "name": s }))
                        .collect::<Vec<_>>(),
                )
                .ok()
            })
            .flatten();

        let db_id: i64 = conn.query_row(
            "INSERT INTO orders
             (organisation_id, order_reference, order_date_raw, order_date, status,
              total_amount, currency, items, tracking_number, transaction_id,
              source_email_id, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             RETURNING id",
            params![
                org_db_id,
                order.order_reference.as_ref(),
                order.order_date.as_ref(),
                Self::parse_date(order.order_date.as_deref()),
                order.status.as_ref(),
                order.total_amount,
                order.currency.as_ref(),
                items_json.as_ref(),
                order.tracking_number.as_ref(),
                tx_db_id,
                source_email_id,
                now,
                now,
            ],
            |row| row.get(0),
        )?;

        id_map.insert(order.id, db_id);
        let display_name = order.order_reference.as_deref().unwrap_or("unknown-order");
        self.index_entity(NamedEntityKind::Order, db_id, display_name, None);

        Ok(db_id)
    }

    pub fn insert_event(
        &self,
        event: &ExtractedEvent,
        source_email_id: Option<i64>,
        id_map: &mut HashMap<i64, i64>,
    ) -> anyhow::Result<i64> {
        let conn = self.pool.get()?;
        let now = chrono::Utc::now().timestamp();

        let location_db_id = self.resolve_fk(id_map, event.location_id);

        let attendees_json = event
            .attendees
            .as_ref()
            .map(|a| serde_json::to_string(a).ok())
            .flatten();

        let db_id: i64 = conn.query_row(
            "INSERT INTO events
             (name, description, event_date_raw, event_date, location_id,
              attendees, source_email_id, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
             RETURNING id",
            params![
                &event.name,
                event.description.as_ref(),
                event.event_date.as_ref(),
                Self::parse_date(event.event_date.as_deref()),
                location_db_id,
                attendees_json.as_ref(),
                source_email_id,
                now,
                now,
            ],
            |row| row.get(0),
        )?;

        id_map.insert(event.id, db_id);
        self.index_entity(NamedEntityKind::Event, db_id, &event.name, None);

        Ok(db_id)
    }

    pub fn insert_named_entity<T: InsertableEntity>(
        &self,
        entity: &T,
        source_email_id: Option<i64>,
        id_map: &mut HashMap<i64, i64>,
    ) -> anyhow::Result<i64> {
        entity.insert_into_db(self, source_email_id, id_map)
    }

    pub fn persist_extraction_result(
        &self,
        params: &dwata_agents::entity_types::ExtractedEntitiesParams,
        source_email_id: Option<i64>,
        sender_email: Option<&str>,
    ) -> anyhow::Result<HashMap<i64, i64>> {
        let mut id_map: HashMap<i64, i64> = HashMap::new();

        if let Some(ref locations) = params.locations {
            for loc in locations {
                self.insert_location(loc, &mut id_map)?;
            }
        }

        if let Some(ref orgs) = params.organisations {
            for org in orgs {
                self.insert_organisation(org, &mut id_map, sender_email)?;
            }
        }

        if let Some(ref persons) = params.persons {
            for person in persons {
                self.insert_person(person, source_email_id, &mut id_map)?;
            }
        }

        if let Some(ref subs) = params.subscriptions {
            for sub in subs {
                self.insert_subscription(sub, source_email_id, &mut id_map)?;
            }
        }

        if let Some(ref bills) = params.bills {
            for bill in bills {
                self.insert_bill(bill, source_email_id, &mut id_map)?;
            }
        }

        if let Some(ref txs) = params.transactions {
            for tx in txs {
                self.insert_transaction(tx, source_email_id, &mut id_map)?;
            }
        }

        if let Some(ref orders) = params.orders {
            for order in orders {
                self.insert_order(order, source_email_id, &mut id_map)?;
            }
        }

        if let Some(ref events) = params.events {
            for event in events {
                self.insert_event(event, source_email_id, &mut id_map)?;
            }
        }

        Ok(id_map)
    }
}

pub trait InsertableEntity: Sized {
    fn insert_into_db(
        &self,
        layer: &KgPersistenceLayer,
        source_email_id: Option<i64>,
        id_map: &mut HashMap<i64, i64>,
    ) -> anyhow::Result<i64>;
}

impl InsertableEntity for ExtractedLocation {
    fn insert_into_db(
        &self,
        layer: &KgPersistenceLayer,
        _: Option<i64>,
        id_map: &mut HashMap<i64, i64>,
    ) -> anyhow::Result<i64> {
        layer.insert_location(self, id_map)
    }
}

impl InsertableEntity for ExtractedOrganisation {
    fn insert_into_db(
        &self,
        layer: &KgPersistenceLayer,
        _: Option<i64>,
        id_map: &mut HashMap<i64, i64>,
    ) -> anyhow::Result<i64> {
        layer.insert_organisation(self, id_map, None)
    }
}

impl InsertableEntity for ExtractedPerson {
    fn insert_into_db(
        &self,
        layer: &KgPersistenceLayer,
        source_email_id: Option<i64>,
        id_map: &mut HashMap<i64, i64>,
    ) -> anyhow::Result<i64> {
        layer.insert_person(self, source_email_id, id_map)
    }
}

impl InsertableEntity for ExtractedSubscription {
    fn insert_into_db(
        &self,
        layer: &KgPersistenceLayer,
        source_email_id: Option<i64>,
        id_map: &mut HashMap<i64, i64>,
    ) -> anyhow::Result<i64> {
        layer.insert_subscription(self, source_email_id, id_map)
    }
}

impl InsertableEntity for ExtractedBill {
    fn insert_into_db(
        &self,
        layer: &KgPersistenceLayer,
        source_email_id: Option<i64>,
        id_map: &mut HashMap<i64, i64>,
    ) -> anyhow::Result<i64> {
        layer.insert_bill(self, source_email_id, id_map)
    }
}

impl InsertableEntity for ExtractedTransaction {
    fn insert_into_db(
        &self,
        layer: &KgPersistenceLayer,
        source_email_id: Option<i64>,
        id_map: &mut HashMap<i64, i64>,
    ) -> anyhow::Result<i64> {
        layer.insert_transaction(self, source_email_id, id_map)
    }
}

impl InsertableEntity for ExtractedOrder {
    fn insert_into_db(
        &self,
        layer: &KgPersistenceLayer,
        source_email_id: Option<i64>,
        id_map: &mut HashMap<i64, i64>,
    ) -> anyhow::Result<i64> {
        layer.insert_order(self, source_email_id, id_map)
    }
}

impl InsertableEntity for ExtractedEvent {
    fn insert_into_db(
        &self,
        layer: &KgPersistenceLayer,
        source_email_id: Option<i64>,
        id_map: &mut HashMap<i64, i64>,
    ) -> anyhow::Result<i64> {
        layer.insert_event(self, source_email_id, id_map)
    }
}

#[async_trait]
impl KgPersistenceProvider for KgPersistenceLayer {
    async fn persist_pass_result(
        &self,
        params: &dwata_agents::entity_types::ExtractedEntitiesParams,
        source_email_id: Option<i64>,
        sender_email: Option<&str>,
    ) -> anyhow::Result<()> {
        self.persist_extraction_result(params, source_email_id, sender_email)?;
        Ok(())
    }
}
