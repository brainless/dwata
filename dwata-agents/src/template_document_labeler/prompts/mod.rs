/// System prompt for the document labeler agent.
///
/// Kept short intentionally — classification is a simple task and small LLMs
/// perform better with concise prompts.
pub fn build_system_prompt(template: &str) -> String {
    format!(
        r#"Classify this financial email template.

Determine:
1. doc_type — choose one: bill, invoice, payment-confirmation, receipt, bank-statement, tax-document
2. has_bill — true if it has an amount due, due date, or billing period
3. has_transaction — true if it confirms a completed payment or debit

Signals for has_bill: "amount due", "due by", "billing period", "pay by", "your bill is ready"
Signals for has_transaction: "payment received", "you paid", "debited", "charged", "transaction successful"

Both can be true (e.g. a receipt that also shows the original bill amount).

## Template

```
{template}
```

Call the `label_document` tool with your classification."#,
        template = template
    )
}
