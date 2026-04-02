use chrono::Utc;

#[derive(Debug, Clone)]
pub struct SenderAggregate {
    pub email_count: i64,
    pub last_email_received_ms: i64,
    pub active_months: i64,
    pub user_reply_count: i64,
    pub engaged_thread_count: i64,
}

pub fn normalize_sender_key(sender: &str) -> String {
    let raw = sender.trim();
    let key = if let (Some(start), Some(end)) = (raw.rfind('<'), raw.rfind('>')) {
        if start + 1 < end {
            &raw[start + 1..end]
        } else {
            raw
        }
    } else {
        raw
    };

    key.trim_matches('"').trim().to_lowercase()
}

pub fn compute_sender_score(stats: &SenderAggregate, current_time_ms: i64) -> f64 {
    let frequency_score = score_frequency(stats.email_count);
    let recency_score = score_recency(stats.last_email_received_ms, current_time_ms);
    let engagement_score = score_user_engagement(stats.user_reply_count);
    let conversation_score =
        score_regular_conversation(stats.active_months, stats.engaged_thread_count);

    let score = frequency_score * 0.20
        + recency_score * 0.20
        + engagement_score * 0.40
        + conversation_score * 0.20;

    score.clamp(0.0, 100.0)
}

fn score_frequency(email_count: i64) -> f64 {
    if email_count <= 0 {
        return 0.0;
    }

    // Fast rise at first, then flatten so high-volume senders don't dominate.
    let count = email_count as f64;
    ((count + 1.0).ln() / (30.0_f64 + 1.0).ln() * 100.0).clamp(0.0, 100.0)
}

fn score_recency(last_email_received_ms: i64, current_time_ms: i64) -> f64 {
    if last_email_received_ms <= 0 {
        return 0.0;
    }

    let now = if current_time_ms > 0 {
        current_time_ms
    } else {
        Utc::now().timestamp_millis()
    };

    let age_ms = (now - last_email_received_ms).max(0);
    let age_days = age_ms as f64 / (1000.0 * 60.0 * 60.0 * 24.0);

    // Decay over ~6 months at sender level.
    (100.0 * (-0.005 * age_days).exp()).max(10.0)
}

fn score_user_engagement(user_reply_count: i64) -> f64 {
    match user_reply_count {
        0 => 0.0,
        1..=2 => 45.0,
        3..=5 => 75.0,
        _ => 100.0,
    }
}

fn score_regular_conversation(active_months: i64, engaged_thread_count: i64) -> f64 {
    let month_score: f64 = match active_months {
        0 => 0.0,
        1 => 20.0,
        2..=3 => 50.0,
        4..=6 => 75.0,
        _ => 100.0,
    };

    let thread_bonus: f64 = match engaged_thread_count {
        0 => 0.0,
        1 => 10.0,
        2..=3 => 20.0,
        4..=6 => 30.0,
        _ => 40.0,
    };

    (month_score + thread_bonus).clamp(0.0, 100.0)
}
