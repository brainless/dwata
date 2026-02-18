/// System prompt for the bill/invoice variable extractor agent.
///
/// Model-aware routing: small models get ultra-concise prompts,
/// larger models get more context.
pub fn build_system_prompt(model: &str, template: &str) -> String {
    if model.contains("ministral") || model.contains("nano") {
        build_short_prompt(template)
    } else {
        build_full_prompt(template)
    }
}

fn build_short_prompt(template: &str) -> String {
    format!(
        r#"Map placeholder variables in this bill/invoice email template to bill fields.

Fields: total-amount, currency, issued-date, due-date, billing-period-start, billing-period-end, document-reference, service-identifier

Template:
```
{template}
```

For service-identifier, also set service_identifier_kind to one of: phone-number, account-number, policy-number, meter-number, subscription-id, contract-id, other.

Set field to null if the placeholder is not a bill field. Call `translate_bill_variables`."#,
        template = template
    )
}

fn build_full_prompt(template: &str) -> String {
    format!(
        r#"You map placeholder variables in a bill or invoice email template to structured bill fields.

## Bill Fields

Use exactly these values for the `field` key:

- **total-amount**: The total amount payable or due (numeric only, no currency symbol)
- **currency**: Currency code or symbol (e.g., "USD", "INR", "$", "₹")
- **issued-date**: The date the bill or invoice was generated or issued by the vendor
- **due-date**: The date by which payment must be made
- **billing-period-start**: Start date of the service or billing period
- **billing-period-end**: End date of the service or billing period
- **document-reference**: Bill number, invoice number, or reference ID from the issuer
- **service-identifier**: The account being billed (phone number, account number, policy number, etc.)

When `field` is **service-identifier**, also set `service_identifier_kind` to one of:
phone-number, account-number, policy-number, meter-number, subscription-id, contract-id, other

## Template

```
{template}
```

## Instructions

1. Read the template carefully
2. For each placeholder (placeholder_N, subject_N), look at surrounding text to determine the bill field
3. Set `field` to null for placeholders that are not bill fields (e.g., customer name, greeting)
4. Call `translate_bill_variables` with all mappings"#,
        template = template
    )
}
