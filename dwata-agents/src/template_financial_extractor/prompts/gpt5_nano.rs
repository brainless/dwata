/// Ultra-concise prompt optimized for GPT-5 nano
///
/// GPT-5 nano is the smallest model, so we need:
/// - Minimal instructions
/// - Direct, imperative language
/// - No verbose explanations

pub fn build_system_prompt(template: &str) -> String {
    format!(
        r#"Translate placeholder variables in this email template to financial field names.

Fields: amount, currency, transaction_date, category, vendor

Template:
```
{template}
```

Map each placeholder to a field or null. Call `translate_variables` with your mappings."#,
        template = template
    )
}
