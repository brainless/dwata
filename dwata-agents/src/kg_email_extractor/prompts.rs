/// User turn that starts each pass.
pub fn start_pass_message(pass_name: &str) -> String {
    format!(
        "Run the **{}** pass now. Extract all relevant entities and call `submit_entities`.",
        pass_name
    )
}

/// Message sent back to the LLM after receiving entities from a pass,
/// before the next pass begins.
pub fn pass_complete_message(pass_name: &str, next_pass_name: &str) -> String {
    format!(
        "Pass **{}** complete — entities persisted. \
         Now run the **{}** pass. \
         Call `submit_entities` with entities for this pass only.",
        pass_name, next_pass_name
    )
}

/// Message when the LLM does not call the tool in time.
pub fn nudge_message() -> &'static str {
    "Please call `submit_entities` now with all entities you found for this pass. \
     If none are present, call it with an empty payload."
}
