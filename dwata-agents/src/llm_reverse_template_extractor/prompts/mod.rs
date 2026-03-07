use crate::llm_reverse_template_extractor::types::ReverseTemplateType;

fn allowed_fields_section(template_type: ReverseTemplateType) -> &'static str {
    match template_type {
        ReverseTemplateType::Bill => {
            r#"Allowed bill fields (use only these names):
- total_amount
- currency
- issued_date
- due_date
- billing_period_start
- billing_period_end
- document_reference
- service_identifier"#
        }
        ReverseTemplateType::Transaction => {
            r#"Allowed transaction fields (use only these names):
- amount
- currency
- transaction_date
- vendor_name
- transaction_reference"#
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
        r#"Reconstruct the likely original source {kind} email template in Jinja2 from the rendered sample below.

Important disambiguation:
- "Reverse template extraction" means reverse-engineering the email template text that generated this message.
- It does NOT mean reversing, refunding, or cancelling any payment/transaction.

Rules:
1. Output one template with this exact outer format:
   Subject: ...
   ---
   ...
2. Infer the template that could have produced the sample email, and replace variable parts with Jinja2 placeholders using canonical field names directly.
3. Do NOT use generic placeholders like placeholder_1 or subject_1.
4. Keep static text intact as much as possible.
5. Use only fields listed below; if no matching field exists, keep raw text.
6. Placeholder syntax must be exactly: {{{{ field_name }}}}
7. Never use Jinja filters, functions, pipes, indexing, or attribute access.
8. Never emit placeholders like:
   - {{{{transaction_date|date("Y-m")}}}}
   - {{{{ vendor.address }}}}
   - {{{{ total_amount | replace("Rs.", "") }}}}
9. Every placeholder variable name must be one of the allowed field names below.
10. When calling `submit_reverse_template`, the `template_body` value must be valid JSON string content:
   - keep it as ONE JSON string value
   - represent all line breaks as escaped newlines (`\n`)
   - do not include raw newline characters inside the JSON string literal

{allowed_fields}

Rendered sample email to reverse-engineer into a template:
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
