use crate::email_ranking::multi_factor::{RankingContext, RankingFactor};
use shared_types::email::Email;

/// Factor: Temporal recency
/// Slight preference for more recent emails
/// Uses exponential decay over 1 year period
pub struct TemporalFactor;

impl RankingFactor for TemporalFactor {
    fn name(&self) -> &str {
        "recency"
    }

    fn score_email(&self, email: &Email, context: &RankingContext) -> f64 {
        let email_time = email.date_received;
        let current_time = context.current_time_ms;

        // Calculate age in days
        let age_ms = current_time - email_time;
        let age_days = age_ms as f64 / (1000.0 * 60.0 * 60.0 * 24.0);

        // Exponential decay over 365 days
        // Recent (0-30 days) = ~90-100 points
        // 90 days old = ~78 points
        // 180 days old = ~61 points
        // 365 days old = ~37 points
        // Very old (2+ years) = ~13 points minimum
        let decay_rate = 0.003; // Controls decay speed
        let score = 100.0 * (-decay_rate * age_days).exp();

        // Ensure minimum of 10 points so very old emails aren't completely ignored
        score.max(10.0)
    }
}
