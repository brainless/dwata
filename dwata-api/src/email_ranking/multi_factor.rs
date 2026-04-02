use shared_types::email::Email;
use std::collections::HashMap;

/// Trait for all ranking factors
pub trait RankingFactor {
    fn name(&self) -> &str;
    fn score_email(&self, email: &Email, context: &RankingContext) -> f64;
}

/// Context for ranking - provides access to related data
pub struct RankingContext {
    /// Map of sender email -> count of user's replies to this sender
    pub user_reply_counts: HashMap<String, i64>,
    /// Map of sender email -> sender reputation score (0-100)
    pub sender_scores: HashMap<String, f64>,
    /// Map of thread_id -> (total_emails, user_participated)
    pub thread_info: HashMap<Option<String>, ThreadInfo>,
    /// Current timestamp for temporal scoring
    pub current_time_ms: i64,
}

#[derive(Debug, Clone, Copy)]
pub struct ThreadInfo {
    pub email_count: i64,
    pub has_user_reply: bool,
}

impl RankingContext {
    pub fn new(current_time_ms: i64) -> Self {
        Self {
            user_reply_counts: HashMap::new(),
            sender_scores: HashMap::new(),
            thread_info: HashMap::new(),
            current_time_ms,
        }
    }
}

/// Weighted ranking configuration
#[derive(Debug, Clone)]
pub struct RankingWeights {
    pub financial_content: f64,
    pub user_engagement: f64,
    pub conversation_thread: f64,
    pub recency: f64,
    /// Additive sender score boost relative to 0-100 sender score.
    /// Example: 0.20 means +20 points max to the base email score.
    pub sender_reputation_boost: f64,
}

impl Default for RankingWeights {
    fn default() -> Self {
        Self {
            financial_content: 0.40,   // Must have financial signals
            user_engagement: 0.30,     // User replied to sender is strong signal
            conversation_thread: 0.20, // Part of ongoing conversation
            recency: 0.10,             // Recent emails slightly preferred
            sender_reputation_boost: 0.20,
        }
    }
}

impl RankingWeights {
    /// Validate weights sum to 1.0 (or close to it)
    pub fn is_valid(&self) -> bool {
        let sum =
            self.financial_content + self.user_engagement + self.conversation_thread + self.recency;
        (sum - 1.0).abs() < 0.001
    }
}

/// Multi-factor email ranking
pub struct MultiFactorRankedEmail {
    pub email_id: i64,
    pub credential_id: i64,
    pub from_address: String,
    pub subject: Option<String>,
    pub date_received: i64,
    pub base_score: f64,  // 0-100, email-only factors
    pub final_score: f64, // 0-100, base + sender boost
    pub factor_scores: FactorScores,
}

/// Individual factor scores
#[derive(Debug, Clone)]
pub struct FactorScores {
    pub financial_content: f64,
    pub user_engagement: f64,
    pub conversation_thread: f64,
    pub recency: f64,
    pub sender_reputation: f64,
}

/// Calculate multi-factor ranking for a list of emails
pub fn rank_emails_multi_factor(
    emails: Vec<Email>,
    context: &RankingContext,
    weights: &RankingWeights,
) -> Vec<MultiFactorRankedEmail> {
    // Create factors
    let factors: Vec<Box<dyn RankingFactor>> = vec![
        Box::new(crate::email_ranking::financial_content::FinancialContentFactor),
        Box::new(crate::email_ranking::user_engagement::UserEngagementFactor),
        Box::new(crate::email_ranking::conversation::ConversationFactor),
        Box::new(crate::email_ranking::temporal::TemporalFactor),
    ];

    let mut ranked: Vec<MultiFactorRankedEmail> = emails
        .into_iter()
        .map(|email| {
            let factor_scores = FactorScores {
                financial_content: factors[0].score_email(&email, context),
                user_engagement: factors[1].score_email(&email, context),
                conversation_thread: factors[2].score_email(&email, context),
                recency: factors[3].score_email(&email, context),
                sender_reputation: context
                    .sender_scores
                    .get(&crate::email_ranking::sender::normalize_sender_key(
                        &email.from_address,
                    ))
                    .copied()
                    .unwrap_or(0.0),
            };

            let base_score = factor_scores.financial_content * weights.financial_content
                + factor_scores.user_engagement * weights.user_engagement
                + factor_scores.conversation_thread * weights.conversation_thread
                + factor_scores.recency * weights.recency;
            let final_score = (base_score
                + factor_scores.sender_reputation * weights.sender_reputation_boost)
                .min(100.0);

            MultiFactorRankedEmail {
                email_id: email.id,
                credential_id: email.credential_id,
                from_address: email.from_address.clone(),
                subject: email.subject.clone(),
                date_received: email.date_received,
                base_score,
                final_score,
                factor_scores,
            }
        })
        .collect();

    // Sort by final score descending, then by date_received descending as tiebreaker
    ranked.sort_by(|a, b| {
        b.final_score
            .partial_cmp(&a.final_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.date_received.cmp(&a.date_received))
    });

    ranked
}

/// Check if email meets minimum criteria to be considered for extraction
pub fn meets_extraction_criteria(email: &Email) -> bool {
    // Use the financial content factor's check
    crate::email_ranking::financial_content::meets_financial_criteria(email)
}
