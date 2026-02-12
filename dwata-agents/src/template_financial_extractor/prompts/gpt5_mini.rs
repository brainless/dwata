/// Concise prompt optimized for GPT-5 mini
///
/// GPT-5 mini is small but capable, so we:
/// - Keep instructions concise
/// - Include key details
/// - Provide minimal context

pub fn build_system_prompt(template: &str) -> String {
    format!(
        r#"You translate placeholder variables in a Jinja2 email template to financial field names.

## Target Fields

- **amount** (f64): Transaction amount
- **currency** (String): Currency code or symbol
- **transaction_date** (String): Date of transaction
- **category** (String): One of: income, expense, investment, tax, utility, subscription, entertainment, travel, healthcare, education, other
- **vendor** (String): Merchant or counterparty name

## Template

```
{template}
```

## Instructions

Map each placeholder (placeholder_N, subject_N) to one of the target fields based on surrounding context. Set to null if it doesn't match any financial field. Call the `translate_variables` tool with your mappings."#,
        template = template
    )
}
