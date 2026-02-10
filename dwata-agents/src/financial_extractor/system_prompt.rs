use shared_types::FinancialPattern;

/// Build a simplified system prompt for Ollama (to avoid size limits with tools)
fn build_ollama_system_prompt(
    email_subject: &str,
    email_body: &str,
    high_signal_line: Option<&str>,
    improved_attempt: bool,
) -> (String, Option<String>) {
    let system_prompt = r#"You are a financial pattern extractor. Create regex patterns to extract financial data from emails.

## Your Task
1. Analyze the email provided by the user
2. Create a regex pattern with capture groups for: amount (required), vendor, date, reference
3. Use test_pattern tool to validate your regex
4. If test succeeds, immediately call save_pattern with the same regex
5. Finish with a brief message

## Regex Requirements
- Use Rust regex syntax (the `regex` crate)
- Amount group must capture numbers like "1,234.56"
- Must include at least one vendor group (source_vendor_group or destination_vendor_group)
- Use numbered groups: group 1, group 2, etc.

## Important
- After first successful test_pattern, immediately call save_pattern
- Maximum 5 test_pattern attempts
- Keep patterns simple and focused"#.to_string();

    let email_content = format!(
        "Email to analyze:\n\n**Subject:** {}\n\n**Body:**\n{}\n\n{}{}",
        email_subject,
        email_body,
        if let Some(line) = high_signal_line {
            format!("**High-signal line:** {}\n\n", line)
        } else {
            String::new()
        },
        if improved_attempt {
            "Note: Previous attempt failed. Use the high-signal line if available. Keep regex single-line."
        } else {
            "Please create a regex pattern to extract the financial data."
        }
    );

    (system_prompt, Some(email_content))
}

pub fn build_system_prompt(
    email_subject: &str,
    email_body: &str,
    existing_patterns: &[FinancialPattern],
    high_signal_line: Option<&str>,
    improved_attempt: bool,
    provider_name: &str,
) -> (String, Option<String>) {
    // For Ollama, use a much simpler prompt to avoid hitting Ollama's limitations
    // with long prompts + complex tool schemas
    if provider_name == "ollama" {
        return build_ollama_system_prompt(email_subject, email_body, high_signal_line, improved_attempt);
    }

    // Full detailed prompt for Gemini
    let include_email_in_system = true;
    let email_section = if include_email_in_system {
        format!("\n## Email to Analyze\n\n**Subject:** {}\n\n**Body:**\n{}", email_subject, email_body)
    } else {
        String::new()
    };

    let system_prompt = format!(
        r#"You are a financial data extraction pattern generator. Your goal is to create regex patterns that extract financial information from emails.

## Target Data Structure

You will extract a FinancialTransaction with these fields:
- **amount** (f64, REQUIRED): The transaction amount (e.g., 1234.56)
- **source_vendor** (String, OPTIONAL): The sender/payer (who money comes from)
- **destination_vendor** (String, OPTIONAL): The receiver/merchant (who money goes to)
- **transaction_date** (String, OPTIONAL): When the transaction occurred
- **reference** (String, OPTIONAL): Invoice ID, confirmation code, or other reference
- **document_type**: One of [invoice, bill, receipt, payment-confirmation, bank-statement, tax-document]
- **status**: One of [paid, pending, overdue, cancelled, refunded]

## Regex Pattern Requirements

Your regex pattern must:
1. Use standard Rust regex syntax (the `regex` crate)
2. Use numbered capture groups: (pattern) creates group 1, (pattern) creates group 2, etc.
3. The amount_group must capture numeric amounts (e.g., "1,234.56" or "1234.56")
4. You MUST provide at least one of source_vendor_group or destination_vendor_group
5. source_vendor_group captures the payer/sender name
6. destination_vendor_group captures the merchant/receiver name
7. The date_group (optional) should capture date strings
8. The reference_group (optional) should capture IDs like invoice numbers, confirmation codes, or order IDs

### Examples of Good Patterns

Pattern: `payment of \$?([\d,]+\.?\d{{0,2}}) to ([A-Za-z\s]+)`
- Group 1 (amount_group): captures "1,234.56"
- Group 2 (destination_vendor_group): captures "Stripe Inc"
- Matches: "payment of $1,234.56 to Stripe Inc"

Pattern: `invoice ([A-Z0-9-]+) for \$?([\d,]+\.?\d{{0,2}}) due ([A-Za-z]+ \d{{1,2}})`
- Group 1 (reference_group): captures "INV-1234"
- Group 2 (amount_group): captures "500.00"
- Group 3 (date_group): captures "January 15"
- Matches: "invoice INV-1234 for $500.00 due January 15"

## Existing Patterns (for reference)

{}

## High-Signal Line (if present)

{}

## Your Task

1. Analyze the email content carefully
2. Identify financial information (amounts, vendors, dates)
3. Create a regex pattern with appropriate capture groups
4. Use the test_pattern tool to validate your regex
5. Iterate until the pattern extracts correct data
6. Use the save_pattern tool to persist the final pattern
7. Finish with a final assistant message and no tool calls

## Available Tools

### test_pattern
Test a regex pattern against the email content.
Parameters:
- regex_pattern: The regex to test
- amount_group: Which capture group contains the amount (starting from 1)
- source_vendor_group: Optional - which capture group contains the payer/sender
- destination_vendor_group: Optional - which capture group contains the merchant/receiver
- date_group: Optional - which capture group contains the date
- reference_group: Optional - which capture group contains the reference

Returns: JSON list of extracted transactions. An empty list means no match.

### save_pattern
Save a validated pattern to the database.
Parameters:
- name: Short name for the pattern (e.g., "stripe_payment_confirmation")
- regex_pattern: The validated regex
- document_type: Type of document (payment-confirmation, invoice, bill, receipt, etc.)
- status: Transaction status (paid, pending, overdue, etc.)
- amount_group: Which capture group has the amount
- source_vendor_group: Optional - which capture group has the payer/sender
- destination_vendor_group: Optional - which capture group has the merchant/receiver
- date_group: Optional - which capture group has the date
- reference_group: Optional - which capture group has the reference

Returns: Pattern ID

## Important Notes

- Start with a simple pattern and refine it
- Test the pattern before saving
- If the pattern doesn't match, analyze why and adjust
- Make patterns specific enough to avoid false positives
- But not so specific that they only match one email
- Once you successfully save a pattern, your task is complete
- Success criteria for test_pattern: at least one transaction with a non-empty amount
- You may attempt at most 5 test_pattern calls before deciding it is not feasible
- If you cannot find a working pattern, respond with a final assistant message explaining why
- After save_pattern, do not call any more tools
- After the first successful test_pattern, immediately call save_pattern with the same regex and group indices. Do not call test_pattern again after a success.

## First Attempt Guidance

{}{}"#,
        format_existing_patterns(existing_patterns),
        high_signal_line.unwrap_or("None detected."),
        first_attempt_guidance(improved_attempt),
        email_section,
    );

    let email_content_for_user = if !include_email_in_system {
        Some(format!("Email to analyze:\n\n**Subject:** {}\n\n**Body:**\n{}", email_subject, email_body))
    } else {
        None
    };

    (system_prompt, email_content_for_user)
}

fn format_existing_patterns(patterns: &[FinancialPattern]) -> String {
    if patterns.is_empty() {
        return "No existing patterns yet.".to_string();
    }

    let mut output = String::new();
    output.push_str(&format!("Total patterns: {}\n\n", patterns.len()));

    for pattern in patterns.iter().take(10) {
        output.push_str(&format!(
            "- **{}**: `{}` (doc_type: {}, status: {})\n",
            pattern.name,
            pattern.regex_pattern,
            pattern.document_type,
            pattern.status
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

fn first_attempt_guidance(improved_attempt: bool) -> &'static str {
    if improved_attempt {
        "Use the high-signal line as the anchor if available. Avoid `.*` across lines. Prefer a single-line regex like: `(?i)Receipt from\\s+([A-Za-z0-9 .,&-]+)\\s+\\$?([\\d,]+\\.\\d{2})\\s+Paid\\s+([A-Za-z]+\\s+\\d{1,2},\\s+\\d{4})` and then adjust minimally. Ensure at least one of source_vendor_group or destination_vendor_group is set."
    } else {
        "First attempt must anchor on a high-signal line that includes vendor, amount, and date. Avoid `.*` across lines and keep the regex single-line when possible. Include at least one of source_vendor_group or destination_vendor_group. If the test succeeds, immediately save the pattern."
    }
}
