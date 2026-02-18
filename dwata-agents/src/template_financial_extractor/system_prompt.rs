/// Legacy system prompt builder — delegates to the prompts module.
/// Kept for backward compatibility with any direct callers.
pub fn build_system_prompt(template: &str) -> String {
    super::prompts::gemini::build_system_prompt(template)
}
