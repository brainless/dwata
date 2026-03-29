use crate::entity_search::NamedEntityKind;

pub fn generate_entity_manifest(for_kinds: Option<&[NamedEntityKind]>) -> String {
    let kinds: Vec<NamedEntityKind> = for_kinds.map(|k| k.to_vec()).unwrap_or_else(|| {
        vec![
            NamedEntityKind::Location,
            NamedEntityKind::Organisation,
            NamedEntityKind::Person,
            NamedEntityKind::Bill,
            NamedEntityKind::Transaction,
            NamedEntityKind::Subscription,
            NamedEntityKind::Order,
            NamedEntityKind::Event,
        ]
    });

    let mut out = String::new();
    out.push_str("## Entity Type Schema\n\n");
    out.push_str("Use these entity schemas when extracting. Reference entities by their `id` field in FK relationships.\n\n");

    for kind in &kinds {
        out.push_str(&format!("### {}\n", capitalize(kind.plural())));
        out.push_str(&format!("**type:** `{}`\n\n", kind.as_str()));
        out.push_str(&describe_entity(kind));
        out.push('\n');
    }

    out
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

fn describe_entity(kind: &NamedEntityKind) -> String {
    match kind {
        NamedEntityKind::Location => r#"**Fields:**
- `id` (integer) — unique identifier, used as FK target
- `name` (string, optional) — named place, e.g. "Central Park"
- `address_line1` (string, optional)
- `address_line2` (string, optional)
- `city` (string, optional)
- `region` (string, optional)
- `country_code` (string, optional) — ISO 3166-1 alpha-2/3
- `postal_code` (string, optional)
- `search_summary` (string, optional) — BM25-searchable relational context

**Referenced by:** Organisation, Event
"#.to_string(),

        NamedEntityKind::Organisation => r#"**Fields:**
- `id` (integer) — unique identifier, used as FK target
- `name` (string) — organisation name
- `industry` (string, optional) — e.g. "streaming", "banking"
- `website` (string, optional) — URL
- `email` (string, optional) — primary billing/contact email
- `roles` (list of strings) — one or more of: business, bank, insurer, payment_platform, employer, utility, service_provider, government, educational, non_profit, unknown
- `location_id` (integer, optional) — FK to Location.id
- `search_summary` (string, optional) — BM25-searchable relational context

**Referenced by:** Person, Bill, Transaction, Subscription, Order
"#.to_string(),

        NamedEntityKind::Person => r#"**Fields:**
- `id` (integer) — unique identifier, used as FK target
- `name` (string) — full name
- `email` (string, optional)
- `phone` (string, optional)
- `organisation_id` (integer, optional) — FK to Organisation.id
- `email_id` (integer, optional) — FK to email this person was extracted from
- `search_summary` (string, optional) — BM25-searchable relational context
"#.to_string(),

        NamedEntityKind::Bill => r#"**Fields:**
- `id` (integer) — unique identifier, used as FK target
- `total_amount` (string, optional) — numeric string only, strip currency symbols
- `currency` (string, optional) — ISO currency code (e.g. INR, USD)
- `issued_date` (string, optional) — raw date string exactly as in source
- `due_date` (string, optional) — raw date string exactly as in source
- `billing_period_start` (string, optional) — raw date string
- `billing_period_end` (string, optional) — raw date string
- `document_reference` (string, optional) — invoice/bill number
- `issuer_organisation_id` (integer, optional) — FK to Organisation.id
- `subscription_id` (integer, optional) — FK to Subscription.id

**Referenced by:** Transaction
"#.to_string(),

        NamedEntityKind::Transaction => r#"**Fields:**
- `id` (integer) — unique identifier, used as FK target
- `amount` (number) — numeric value, strip currency symbols
- `currency` (string) — ISO currency code
- `transaction_date` (string, optional) — raw date string exactly as in source
- `transaction_reference` (string, optional) — payment ref, tx id
- `payer_organisation_id` (integer, optional) — FK to Organisation.id (who paid)
- `payee_organisation_id` (integer, optional) — FK to Organisation.id (who received)
- `bill_id` (integer, optional) — FK to Bill.id this settles

**Referenced by:** Order
"#.to_string(),

        NamedEntityKind::Subscription => r#"**Fields:**
- `id` (integer) — unique identifier, used as FK target
- `organisation_id` (integer, optional) — FK to Organisation.id
- `service_name` (string) — human-readable service name, e.g. "Netflix"
- `plan_name` (string, optional) — plan tier, e.g. "Premium"
- `billing_cycle` (string, optional) — one of: weekly, monthly, quarterly, semi_annual, annual, other
- `amount` (number, optional) — recurring charge amount
- `currency` (string, optional)
- `next_billing_date` (string, optional) — raw date string
- `start_date` (string, optional) — raw date string

**Referenced by:** Bill
"#.to_string(),

        NamedEntityKind::Order => r#"**Fields:**
- `id` (integer) — unique identifier, used as FK target
- `organisation_id` (integer, optional) — FK to Organisation.id (seller)
- `order_reference` (string, optional)
- `order_date` (string, optional) — raw date string
- `status` (string, optional) — one of: placed, confirmed, shipped, out_for_delivery, delivered, cancelled, returned, refunded, unknown
- `total_amount` (number, optional)
- `currency` (string, optional)
- `items` (list of strings, optional) — product names/descriptions
- `tracking_number` (string, optional)
- `transaction_id` (integer, optional) — FK to Transaction.id
"#.to_string(),

        NamedEntityKind::Event => r#"**Fields:**
- `id` (integer) — unique identifier, used as FK target
- `name` (string) — event title
- `description` (string, optional)
- `event_date` (string, optional) — raw date/time string
- `location_id` (integer, optional) — FK to Location.id
- `attendees` (list of strings, optional) — email addresses or names

**FK targets:** Location
"#.to_string(),
    }
}

pub fn existing_entities_section(results: &[crate::entity_search::EntitySearchResult]) -> String {
    if results.is_empty() {
        return String::new();
    }

    let mut out = String::new();
    out.push_str("## Existing Entities (Pre-populated)\n\n");
    out.push_str("Reference these existing entities by their `id` when linking new entities.\n\n");

    for result in results {
        out.push_str(&format!(
            "- **[{}]({})** — id:{}\n",
            result.name,
            result.entity_type.as_str(),
            result.id
        ));
        if let Some(ref summary) = result.search_summary {
            out.push_str(&format!("  {}\n", summary));
        }
    }

    out
}
