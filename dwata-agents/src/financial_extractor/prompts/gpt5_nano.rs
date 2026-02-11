/// Ultra-concise prompt optimized for GPT-5 nano
///
/// GPT-5 nano is the smallest model, so we need:
/// - Minimal instructions
/// - Direct, imperative language
/// - No examples or verbose explanations
/// - Focus on the core task only

pub fn build_system_prompt(
    email_subject: &str,
    email_body: &str,
    high_signal_line: Option<&str>,
    improved_attempt: bool,
) -> (String, Option<String>) {
    let hint = if improved_attempt && high_signal_line.is_some() {
        format!("\n\nFocus on this line: {}", high_signal_line.unwrap())
    } else {
        String::new()
    };

    let system_prompt = format!(
        r#"Extract financial data using regex patterns.

Tools:
- test_pattern: Test regex on email, returns extracted data
- save_pattern: Save working regex to database

Task:
1. Create regex with capture groups for amount and vendor
2. Call test_pattern with regex and group numbers
3. If successful, call save_pattern immediately
4. Done

Requirements:
- Amount group REQUIRED (captures dollar amount)
- Vendor group REQUIRED (source_vendor_group OR destination_vendor_group)
- Use numbered groups: (pattern) = group 1, (pattern) = group 2, etc
- Keep regex simple{}"#,
        hint
    );

    // For nano, include email in user message to save system prompt tokens
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
