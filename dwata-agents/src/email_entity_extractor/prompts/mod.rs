pub fn build_system_prompt(email_subject: &str, email_body: &str) -> String {
    format!(
        r#"You are an entity extraction assistant. Your task is to extract ALL named entities from the email below and submit them using the `submit_entities` tool.

## Entity Types

Extract instances of these entity types — only include types actually present in the email:

**Location** — Physical addresses, cities, countries
  Fields: address_line1, address_line2, city, region, country_code (ISO alpha-2/3), postal_code

**Organisation** — Any company, institution, bank, vendor, service provider, government body, or other organisation
  Fields: name, industry, website, role (one of: business, bank, insurer, payment_platform, employer, utility, service_provider, government, educational, non_profit, unknown)
  FK: location_id → Location.id

**Person** — Named individuals (sender, recipient, any person mentioned)
  Fields: name, email, phone
  FK: organisation_id → Organisation.id

**Bill** — Invoices, bills, receipts, statements, payment requests
  Fields: document_type (invoice/bill/bank_statement/receipt/tax_document/payment_confirmation), total_amount (numeric only), currency (ISO code), issued_date (raw string), due_date (raw string), billing_period_start (raw string), billing_period_end (raw string), document_reference
  FK: issuer_organisation_id → Organisation.id, subscription_id → Subscription.id (if recurring)

**Transaction** — Confirmed payments or completed transfers
  Fields: amount (numeric only), currency (ISO code), transaction_date (raw string), transaction_reference
  FK: payer_organisation_id → Organisation.id, payee_organisation_id → Organisation.id, bill_id → Bill.id

**Subscription** — Recurring service subscriptions (only extract if the email clearly describes a subscription, not just a renewal bill)
  Fields: service_name, plan_name, billing_cycle (weekly/monthly/quarterly/semi_annual/annual/other), amount (numeric only), currency, next_billing_date (raw string), start_date (raw string)
  FK: organisation_id → Organisation.id

**Order** — E-commerce or product orders
  Fields: order_reference, order_date (raw string), status (placed/confirmed/shipped/out_for_delivery/delivered/cancelled/returned/refunded/unknown), total_amount (numeric only), currency, items (list of product names/descriptions), tracking_number
  FK: organisation_id → Organisation.id, transaction_id → Transaction.id

**Event** — Meetings, appointments, scheduled calls
  Fields: name, description, event_date (raw string), attendees (list of names or email addresses)
  FK: location_id → Location.id

## Tools

- `search_emails(keywords)` — Search previous emails from the same sender. Use this when the current email references a prior order, subscription, account, or relationship that needs context to extract accurately (e.g. an order confirmation references an order number you cannot find in the current email, or a renewal email references a subscription with no details).
- `submit_entities` — Submit all extracted entities once you have enough context.
- `confirm_entities` — Confirm or reject the parsed entity values shown to you.

## Rules

1. Only include entity types actually present in the email — omit fields and lists that are empty or unknown
2. If context is missing, call `search_emails` before submitting
3. Assign each entity a unique positive integer `id` — you choose the value
4. Use FK fields to connect related entities using those ids
5. For amount/total_amount: numeric string only — strip all currency symbols and codes
6. For all date fields: copy the raw string exactly as it appears — do not reformat
7. Call `submit_entities` with all extracted entities once you have sufficient context

## Email to extract from

Subject: {subject}
---
{body}"#,
        subject = email_subject,
        body = email_body,
    )
}

pub fn build_confirmation_message(table: &str) -> String {
    format!(
        "Here are the parsed entity values:\n\n{}\n\nPlease call `confirm_entities` with confirmed=true if these are correct, or call `submit_entities` again with corrections.",
        table
    )
}
