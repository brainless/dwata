/// Concise prompt optimized for GPT-5 mini.

pub fn build_system_prompt(template: &str) -> String {
    format!(
        r#"Map placeholder variables in this payment email template to transaction fields.

## Fields

- **amount**: Transaction amount (numeric only)
- **currency**: Currency code or symbol
- **transaction-date**: Date of transaction
- **vendor**: Merchant or counterparty name
- **transaction-reference**: Confirmation or reference number

## Template

```
{template}
```

Map each placeholder to a field or null. Call `translate_variables`."#,
        template = template
    )
}
