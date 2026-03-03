use crate::llm_reverse_template_extractor::types::ReverseTemplateType;

fn allowed_fields_section(template_type: ReverseTemplateType) -> &'static str {
    match template_type {
        ReverseTemplateType::Bill => {
            r#"Allowed bill fields (use only these names):
- total-amount
- currency
- issued-date
- due-date
- billing-period-start
- billing-period-end
- document-reference
- service-identifier"#
        }
        ReverseTemplateType::Transaction => {
            r#"Allowed transaction fields (use only these names):
- amount
- currency
- transaction-date
- vendor
- transaction-reference"#
        }
    }
}

pub fn build_system_prompt(
    template_type: ReverseTemplateType,
    sample_subject: &str,
    sample_body: &str,
) -> String {
    let kind = match template_type {
        ReverseTemplateType::Bill => "bill",
        ReverseTemplateType::Transaction => "transaction",
    };
    format!(
        r#"Reverse this {kind} sample into a Jinja2 template.

Rules:
1. Output one template with this exact outer format:
   Subject: ...
   ---
   ...
2. Replace variable parts with Jinja2 placeholders using canonical field names directly.
3. Do NOT use generic placeholders like placeholder_1 or subject_1.
4. Keep static text intact as much as possible.
5. Use only fields listed below; if no matching field exists, keep raw text.
6. Placeholder syntax must be exactly: {{ field-name }}
7. Never use Jinja filters, functions, pipes, indexing, or attribute access.
8. Never emit placeholders like:
   - {{transaction-date|date("Y-m")}}
   - {{ vendor.address }}
   - {{ total-amount | replace("Rs.", "") }}
9. Every placeholder variable name must be one of the allowed field names below.

{allowed_fields}

Sample email:
Subject: {sample_subject}
---
{sample_body}

You MUST call the `submit_reverse_template` tool with the final template."#,
        kind = kind,
        allowed_fields = allowed_fields_section(template_type),
        sample_subject = sample_subject,
        sample_body = sample_body,
    )
}
