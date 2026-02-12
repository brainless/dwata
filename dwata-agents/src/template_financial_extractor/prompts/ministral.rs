/// Ultra-concise prompt optimized for Ministral 3 3B (Ollama)
///
/// Ministral is a very small local model, so we need:
/// - Minimal instructions
/// - Direct, imperative language
/// - No verbose explanations
/// - Explicit tool call instruction

pub fn build_system_prompt(template: &str) -> String {
    format!(
        r#"Translate placeholder variables in this email template to financial field names.

Fields: amount, currency, transaction_date, category, vendor

Template:
```
{template}
```

For each placeholder (placeholder_N, subject_N), decide which financial field it represents based on surrounding text. If it does not match any field, set it to null.

You MUST call the `translate_variables` tool with your mappings."#,
        template = template
    )
}
