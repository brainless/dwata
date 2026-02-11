/// Concise prompt optimized for GPT-5 mini
///
/// GPT-5 mini is small but capable, so we:
/// - Keep instructions concise
/// - Include key details
/// - Provide minimal context
/// - One-sentence descriptions

pub fn build_system_prompt(
    email_subject: &str,
    email_body: &str,
    high_signal_line: Option<&str>,
    improved_attempt: bool,
) -> (String, Option<String>) {
    let guidance = if improved_attempt {
        if let Some(line) = high_signal_line {
            format!("\n\n**Focus Line:** {}\nCreate single-line regex matching this line exactly.", line)
        } else {
            "\n\n**Retry:** Previous attempt failed. Use simpler single-line regex.".to_string()
        }
    } else {
        String::new()
    };

    let system_prompt = format!(
        r#"You extract financial data from emails using regex patterns and function calls.

## Available Functions

**test_pattern** - Validates regex against email content
- regex_pattern: Your regex with capture groups
- amount_group: Which group (1, 2, 3...) has the dollar amount
- destination_vendor_group: Which group has the merchant/vendor name
- date_group: (optional) Which group has the date
- reference_group: (optional) Which group has invoice/receipt ID

**save_pattern** - Saves validated regex to database
- Same parameters as test_pattern, plus:
- name: Short name like "stripe_receipt"
- document_type: "receipt", "invoice", "bill", etc
- status: "paid", "pending", etc

## Process

1. Analyze email to find amount and vendor
2. Create regex with numbered capture groups (pattern) (pattern)
3. Call test_pattern with your regex
4. If test returns data, immediately call save_pattern
5. Done

## Regex Rules

- Amount group REQUIRED - must capture the dollar amount like 10.00
- Vendor group REQUIRED - either source_vendor_group OR destination_vendor_group
- Use simple patterns - avoid complex lookaheads
- Single-line patterns work best{}"#,
        guidance
    );

    // Include email in user message
    let email_content = format!(
        "**Subject:** {}\n\n**Body:**\n{}{}",
        email_subject,
        email_body,
        if let Some(line) = high_signal_line {
            format!("\n\n**Key line:** {}", line)
        } else {
            String::new()
        }
    );

    (system_prompt, Some(email_content))
}
