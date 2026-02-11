/// Model-specific prompts for financial data extraction
///
/// Different models have different capabilities and require different prompt styles:
/// - GPT-5 nano: Ultra-concise, direct instructions
/// - GPT-5 mini: Concise with key details
/// - Gemini/larger models: Detailed, structured prompts

pub mod gpt5_nano;
pub mod gpt5_mini;
pub mod gemini;

/// Get the appropriate system prompt based on provider and model
pub fn get_system_prompt(
    provider: &str,
    model: &str,
    email_subject: &str,
    email_body: &str,
    high_signal_line: Option<&str>,
    improved_attempt: bool,
) -> (String, Option<String>) {
    // Check model name first, then provider
    if model.contains("nano") || model.contains("gpt-5-nano") {
        gpt5_nano::build_system_prompt(email_subject, email_body, high_signal_line, improved_attempt)
    } else if model.contains("mini") || model.contains("gpt-5-mini") {
        gpt5_mini::build_system_prompt(email_subject, email_body, high_signal_line, improved_attempt)
    } else if provider == "ollama" {
        // Ollama uses simplified prompts (already in system_prompt.rs)
        super::system_prompt::build_system_prompt(
            email_subject,
            email_body,
            &[],
            high_signal_line,
            improved_attempt,
            provider,
        )
    } else {
        // Default to Gemini-style detailed prompt
        gemini::build_system_prompt(email_subject, email_body, high_signal_line, improved_attempt)
    }
}
