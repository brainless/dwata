/// Ultra-concise prompt optimized for GPT-5 nano.

pub fn build_system_prompt(template: &str) -> String {
    format!(
        r#"Map placeholders in this payment email template to transaction fields.

Fields: amount, currency, transaction-date, vendor, transaction-reference

Template:
```
{template}
```

Set field to null if not a transaction field.
Type rules: amount numeric, currency code/symbol, transaction-date date-like, transaction-reference ID-like. If type mismatch, set null.
Must map at least one placeholder to amount.
Call `translate_variables`."#,
        template = template
    )
}
