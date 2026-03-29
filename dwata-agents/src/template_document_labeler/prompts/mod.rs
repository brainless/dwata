/// System prompt for the document labeler agent.
///
/// Kept short intentionally — classification is a simple task and small LLMs
/// perform better with concise prompts.
pub fn build_system_prompt(template: &str) -> String {
    format!(
        r#"Classify this email template.

Determine:
1. doc_type — choose one: bill, invoice, payment-confirmation, receipt, bank-statement, tax-document
2. has_bill — true if it has an amount due, due date, or billing period
3. has_transaction — true if it confirms a completed payment or debit
4. has_event — true if it contains a meeting, appointment, or calendar event
5. has_order — true if it contains an e-commerce order or shipment

Signals for has_bill: "amount due", "due by", "billing period", "pay by", "your bill is ready"
Signals for has_transaction: "payment received", "you paid", "debited", "charged", "transaction successful"
Signals for has_event: "meeting", "appointment", "scheduled", "calendar invite", "join us", "event"
Signals for has_order: "order", "shipment", "tracking number", "shipped", "delivered", "your purchase", "item"

Multiple flags can be true.

## Template

```
{template}
```

Call the `label_document` tool with your classification."#,
        template = template
    )
}
