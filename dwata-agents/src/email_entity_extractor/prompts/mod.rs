pub fn build_system_prompt(email_subject: &str, email_body: &str) -> String {
    format!(
        r#"You are an entity extraction assistant. Your task is to extract ALL named entities from the email below and submit them using the `submit_entities` tool.

## Entity Types

Extract instances of these entity types — only include types that are actually present in the email:

**Location** — Physical addresses, cities, countries
  Fields: address_line1, address_line2, city, region, country_code (ISO alpha-2/3), postal_code

**Company** — Companies, organisations, institutions
  Fields: name, industry, website
  FK: location_id → Location.id

**Contact** — Named individuals (sender, recipient, any person mentioned)
  Fields: name, email, phone
  FK: company_id → Company.id

**Vendor** — Merchants, banks, payment processors, service providers
  Fields: vendor_name, vendor_type (one of: self_user, self_business, financial_instrument, merchant, employer, bank, individual, platform, unknown), vendor_external_id

**Bill** — Invoices, bills, receipts, statements, payment requests
  Fields: document_type (invoice/bill/bank_statement/receipt/tax_document/payment_confirmation), total_amount (numeric only, no currency symbol), currency (ISO code), issued_date (raw string), due_date (raw string), billing_period_start (raw string), billing_period_end (raw string), document_reference
  FK: issuer_vendor_id → Vendor.id

**Transaction** — Confirmed payments or completed transfers
  Fields: amount (numeric only, no currency symbol), currency (ISO code), transaction_date (raw string), transaction_reference
  FK: payer_vendor_id → Vendor.id, payee_vendor_id → Vendor.id, bill_id → Bill.id

**Event** — Meetings, appointments, scheduled calls
  Fields: name, description, event_date (raw string), attendees (list of email addresses or names)
  FK: location_id → Location.id

## Rules

1. Extract ALL entities you can find — be thorough
2. Assign each entity a unique positive integer `id` — you choose the value
3. Use FK fields to connect related entities (e.g. a Transaction's payee_vendor_id referencing a Vendor you extracted)
4. For amount/total_amount: extract numeric value only, strip all currency symbols and codes
5. For date fields: copy the raw string exactly as it appears in the email — do not reformat
6. Call `submit_entities` with all extracted entities now

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
        "Here are the parsed entity values:\n\n{}\n\nPlease call `confirm_entities` with confirmed=true if these are correct, or call `submit_entities` again with any corrections.",
        table
    )
}
