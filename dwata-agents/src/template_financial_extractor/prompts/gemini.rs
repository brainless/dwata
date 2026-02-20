/// Detailed prompt optimized for Gemini Flash and larger models.

pub fn build_system_prompt(template: &str) -> String {
    format!(
        r#"You map placeholder variables in a payment/transaction email template to structured transaction fields.

## Transaction Fields

Use exactly these values for the `field` key:

- **amount**: The transaction amount (numeric only, no currency symbol)
- **currency**: Currency code or symbol (e.g., "USD", "INR", "$", "₹")
- **transaction-date**: Date the transaction occurred or was processed
- **vendor**: Merchant, company, or counterparty name
- **transaction-reference**: Confirmation number, reference ID, or UTR

## Template

```
{template}
```

## Instructions

1. Read the template carefully
2. For each placeholder (placeholder_N, subject_N), look at surrounding text to determine the field
3. Type rules:
   - amount must be numeric
   - currency must be ISO code or currency symbol
   - transaction-date must be an actual date
   - transaction-reference should look like an ID, not a sentence
4. For transaction templates, `amount` is mandatory. At least one placeholder must map to `amount`
5. Set `field` to null for placeholders with type mismatch or for non-transaction fields
6. Call `translate_variables` with all mappings"#,
        template = template
    )
}
