/// Ultra-concise prompt optimized for Ministral 3 3B (Ollama).

pub fn build_system_prompt(template: &str) -> String {
    format!(
        r#"Map placeholders in this payment email template to transaction fields.

Fields: amount, currency, transaction-date, vendor, transaction-reference

Template:
```
{template}
```

For each placeholder (placeholder_N, subject_N), pick the matching field based on surrounding text. Set field to null if it does not match any field.

You MUST call the `translate_variables` tool with your mappings."#,
        template = template
    )
}
