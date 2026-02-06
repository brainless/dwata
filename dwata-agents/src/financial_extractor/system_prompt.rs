use shared_types::FinancialPattern;

pub fn build_system_prompt(
    email_subject: &str,
    email_body: &str,
    existing_patterns: &[FinancialPattern],
) -> String {
    format!(
        r#"You are a financial data extraction pattern generator. Your goal is to create regex patterns that extract financial information from emails.

## Target Data Structure

You will extract a FinancialTransaction with these fields:
- **amount** (f64, REQUIRED): The transaction amount (e.g., 1234.56)
- **vendor** (String, OPTIONAL): Who the payment is to/from
- **transaction_date** (String, OPTIONAL): When the transaction occurred
- **document_type**: One of [invoice, bill, receipt, payment-confirmation, bank-statement, tax-document]
- **status**: One of [paid, pending, overdue, cancelled, refunded]

## Regex Pattern Requirements

Your regex pattern must:
1. Use standard Rust regex syntax (the `regex` crate)
2. Use numbered capture groups: (pattern) creates group 1, (pattern) creates group 2, etc.
3. The amount_group must capture numeric amounts (e.g., "1,234.56" or "1234.56")
4. The vendor_group (optional) should capture company/vendor names
5. The date_group (optional) should capture date strings

### Examples of Good Patterns

Pattern: `payment of \$?([\d,]+\.?\d{{0,2}}) to ([A-Za-z\s]+)`
- Group 1 (amount_group): captures "1,234.56"
- Group 2 (vendor_group): captures "Stripe Inc"
- Matches: "payment of $1,234.56 to Stripe Inc"

Pattern: `invoice for \$?([\d,]+\.?\d{{0,2}}) due ([A-Za-z]+ \d{{1,2}})`
- Group 1 (amount_group): captures "500.00"
- Group 2 (date_group): captures "January 15"
- Matches: "invoice for $500.00 due January 15"

## Existing Patterns (for reference)

{}

## Email to Analyze

**Subject:** {}

**Body:**
{}

## Your Task

1. Analyze the email content carefully
2. Identify financial information (amounts, vendors, dates)
3. Create a regex pattern with appropriate capture groups
4. Use the test_pattern tool to validate your regex
5. Iterate until the pattern extracts correct data
6. Use the save_pattern tool to persist the final pattern

## Available Tools

### test_pattern
Test a regex pattern against the email content.
Parameters:
- regex_pattern: The regex to test
- amount_group: Which capture group contains the amount (starting from 1)
- vendor_group: Optional - which capture group contains the vendor
- date_group: Optional - which capture group contains the date

Returns: List of extracted transactions

### save_pattern
Save a validated pattern to the database.
Parameters:
- name: Short name for the pattern (e.g., "stripe_payment_confirmation")
- regex_pattern: The validated regex
- description: What this pattern matches
- document_type: Type of document (payment-confirmation, invoice, bill, receipt, etc.)
- status: Transaction status (paid, pending, overdue, etc.)
- confidence: How confident you are in this pattern (0.0 to 1.0)
- amount_group: Which capture group has the amount
- vendor_group: Optional - which capture group has the vendor
- date_group: Optional - which capture group has the date

Returns: Pattern ID

## Important Notes

- Start with a simple pattern and refine it
- Test the pattern before saving
- If the pattern doesn't match, analyze why and adjust
- Make patterns specific enough to avoid false positives
- But not so specific that they only match one email
- Once you successfully save a pattern, your task is complete"#,
        format_existing_patterns(existing_patterns),
        email_subject,
        email_body,
    )
}

fn format_existing_patterns(patterns: &[FinancialPattern]) -> String {
    if patterns.is_empty() {
        return "No existing patterns yet.".to_string();
    }

    let mut output = String::new();
    output.push_str(&format!("Total patterns: {}\n\n", patterns.len()));

    for pattern in patterns.iter().take(10) {
        output.push_str(&format!(
            "- **{}**: `{}` (doc_type: {}, status: {}, confidence: {:.2})\n",
            pattern.name,
            pattern.regex_pattern,
            pattern.document_type,
            pattern.status,
            pattern.confidence
        ));
    }

    if patterns.len() > 10 {
        output.push_str(&format!(
            "\n... and {} more patterns\n",
            patterns.len() - 10
        ));
    }

    output
}
