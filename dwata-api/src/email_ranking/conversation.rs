use crate::email_ranking::multi_factor::{RankingContext, RankingFactor};
use shared_types::email::Email;

/// Factor: Conversation thread participation
/// Higher score if email is part of an ongoing conversation, especially one where user participated
pub struct ConversationFactor;

impl RankingFactor for ConversationFactor {
    fn name(&self) -> &str {
        "conversation_thread"
    }

    fn score_email(&self, email: &Email, context: &RankingContext) -> f64 {
        let thread_id = email.thread_id.clone();

        // Get thread info if available
        if let Some(thread_info) = context.thread_info.get(&thread_id) {
            let thread_length = thread_info.email_count;
            let has_user_reply = thread_info.has_user_reply;

            // Base score from thread length
            // Single email = 10 points
            // 2-3 emails = 30 points (brief exchange)
            // 4-6 emails = 50 points (ongoing conversation)
            // 7+ emails = 70 points (long conversation)
            let length_score = match thread_length {
                0 | 1 => 10.0,
                2..=3 => 30.0,
                4..=6 => 50.0,
                _ => 70.0,
            };

            // Bonus for user participation
            // If user replied in this thread, add up to 30 more points
            let participation_bonus = if has_user_reply { 30.0 } else { 0.0 };

            let total = length_score + participation_bonus;
            if total > 100.0 {
                100.0
            } else {
                total
            }
        } else {
            // No thread info available - treat as isolated email
            10.0
        }
    }
}
