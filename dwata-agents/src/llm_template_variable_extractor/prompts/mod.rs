use crate::llm_template_variable_extractor::types::TemplateVariableType;

fn allowed_fields_section(template_type: TemplateVariableType) -> &'static str {
    match template_type {
        TemplateVariableType::Bill => {
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
        TemplateVariableType::Transaction => {
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
    template_type: TemplateVariableType,
    sample_subject: &str,
    sample_body: &str,
) -> String {
    let kind = match template_type {
        TemplateVariableType::Bill => "bill",
        TemplateVariableType::Transaction => "transaction",
    };
    format!(
        r#"Extract all template variables from the {kind} email sample below.

Important:
- "Extract variables" means identifying the dynamic data points that would be replaced by a template.
- It does NOT mean reversing, refunding, or cancelling any payment/transaction.

For each variable found in the email:
1. Identify the canonical variable name from the allowed fields list
2. Extract the exact value from the email sample

Rules:
1. Extract the EXACT value as it appears in the email - do not reformat or parse it
2. If a value contains multiple pieces of data (e.g., "03325490439 (Account No. 8006515265)"), extract the ENTIRE thing as one variable
3. Use only fields listed below; if no matching field exists, skip that data
4. For transaction vendor details, extract only `vendor_name`. Never use vendor IDs
5. When calling `submit_template_variables`, the values must be valid JSON strings

{allowed_fields}

Email sample to extract variables from:
Subject: {sample_subject}
---
{sample_body}

You MUST call the `submit_template_variables` tool with the extracted variables."#,
        kind = kind,
        allowed_fields = allowed_fields_section(template_type),
        sample_subject = sample_subject,
        sample_body = sample_body,
    )
}
