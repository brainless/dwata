/// Model-specific prompts for template-based financial variable translation
///
/// Different models have different capabilities and require different prompt styles:
/// - Ministral 3 3B: Ultra-concise for small local model (Ollama)
/// - GPT-5 nano: Ultra-concise, minimal instructions
/// - GPT-5 mini: Concise with key details
/// - Gemini/larger models: Detailed, structured prompts

pub mod gemini;
pub mod gpt5_mini;
pub mod gpt5_nano;
pub mod ministral;

/// Get the appropriate system prompt based on model name
pub fn get_system_prompt(model: &str, template: &str) -> String {
    if model.contains("ministral") {
        ministral::build_system_prompt(template)
    } else if model.contains("nano") || model.contains("gpt-5-nano") {
        gpt5_nano::build_system_prompt(template)
    } else if model.contains("mini") || model.contains("gpt-5-mini") {
        gpt5_mini::build_system_prompt(template)
    } else {
        gemini::build_system_prompt(template)
    }
}
