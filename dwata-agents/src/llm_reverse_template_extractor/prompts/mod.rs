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
- service_identifier
- category"#
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
        r#"Reconstruct the likely original source {kind} email template from the rendered sample below.

Important disambiguation:
- "Reverse template extraction" means reverse-engineering the email template text that generated this message.
- It does NOT mean reversing, refunding, or cancelling any payment/transaction.

STRICT RULE: Only use raw variable placeholders. Inside {{{{ }}}}, you must have ONLY the field name, nothing else.

FORBIDDEN - Every one of these is WRONG:
- Any method call: {{{{ name.something() }}}}, {{{{ name.split() }}}}, {{{{ name[0] }}}}
- Any pipe/filter: {{{{ name|filter }}}}
- Any attribute: {{{{ name.attr }}}}
- Any function: {{{{ name.replace(...) }}}}

CORRECT format: {{{{ field_name }}}} - just the name, nothing else.

Examples of WRONG output:
- {{{{ document_reference.split()[1] }}}}    <-- method call forbidden
- {{{{ billing_period_start|replace("'", "") }}}}   <-- pipe forbidden  
- {{{{ total_amount.upper() }}}}              <-- method call forbidden
- {{{{ items[0] }}}}                           <-- indexing forbidden

Examples of CORRECT output:
- {{{{ document_reference }}}}
- {{{{ billing_period_start }}}}
- {{{{ total_amount }}}}
- {{{{ vendor_name }}}}

IMPORTANT: If the sample email contains complex data like "03325490439 (Account No. 8006515265)", extract the ENTIRE thing as one variable (e.g., document_reference), do NOT try to parse it with split/indices. Keep the parsing logic in your head - only output simple variable names.

Rules:
1. Output one template with this exact outer format:
   Subject: ...
   ---
   ...
2. Use only simple placeholders {{{{ field_name }}}} - nothing else inside the braces.
3. Do NOT use generic placeholders like placeholder_1 or subject_1.
4. Keep static text intact as much as possible.
5. Use only fields listed below; if no matching field exists, keep raw text.
6. Every placeholder variable name must be one of the allowed field names below.
7. For transaction vendor details, extract only `vendor_name`. Never use vendor IDs (for example, `payer_vendor_id`/`payee_vendor_id`) in placeholders.
8. When calling `submit_reverse_template`, the `template_body` value must be valid JSON string content:
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
