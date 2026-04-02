use crate::email_ranking::multi_factor::{RankingContext, RankingFactor};
use shared_types::email::Email;

/// Factor: User engagement with sender
/// Higher score if user has replied to this sender before
pub struct UserEngagementFactor;

impl RankingFactor for UserEngagementFactor {
    fn name(&self) -> &str {
        "user_engagement"
    }

    fn score_email(&self, email: &Email, context: &RankingContext) -> f64 {
        let sender = crate::email_ranking::sender::normalize_sender_key(&email.from_address);

        // Get count of user's replies to this sender
        let reply_count = context.user_reply_counts.get(&sender).copied().unwrap_or(0);

        // Score based on reply count
        // 0 replies = 0 points
        // 1-2 replies = 40 points (light engagement)
        // 3-5 replies = 70 points (moderate engagement)
        // 6+ replies = 100 points (high engagement)
        match reply_count {
            0 => 0.0,
            1..=2 => 40.0,
            3..=5 => 70.0,
            _ => 100.0,
        }
    }
}
