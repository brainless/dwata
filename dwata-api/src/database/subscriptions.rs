use crate::database::AsyncDbConnection;
use anyhow::Result;
use shared_types::{BillingCycle, Subscription};

fn billing_cycle_from_str(value: &str) -> Option<BillingCycle> {
    match value {
        "weekly" => Some(BillingCycle::Weekly),
        "monthly" => Some(BillingCycle::Monthly),
        "quarterly" => Some(BillingCycle::Quarterly),
        "semi_annual" => Some(BillingCycle::SemiAnnual),
        "annual" => Some(BillingCycle::Annual),
        _ => Some(BillingCycle::Other),
    }
}

pub async fn get_subscription(conn: AsyncDbConnection, id: i64) -> Result<Subscription> {
    let conn = conn.lock().await;

    let mut stmt = conn.prepare(
        "SELECT id, organisation_id, service_name, plan_name, billing_cycle, amount,
                currency, next_billing_date_raw, next_billing_date, start_date_raw, start_date,
                source_email_id, created_at, updated_at
         FROM subscriptions
         WHERE id = ?",
    )?;

    stmt.query_row([id], |row| {
        let billing_cycle_str: Option<String> = row.get(4)?;
        let billing_cycle = billing_cycle_str
            .as_deref()
            .and_then(|s| billing_cycle_from_str(s));

        Ok(Subscription {
            id: row.get(0)?,
            organisation_id: row.get(1)?,
            service_name: row.get(2)?,
            plan_name: row.get(3)?,
            billing_cycle,
            amount: row.get(5)?,
            currency: row.get(6)?,
            next_billing_date_raw: row.get(7)?,
            next_billing_date: row.get(8)?,
            start_date_raw: row.get(9)?,
            start_date: row.get(10)?,
            source_email_id: row.get(11)?,
            created_at: row.get(12)?,
            updated_at: row.get(13)?,
        })
    })
    .map_err(|e| anyhow::anyhow!("Failed to get subscription: {}", e))
}

pub async fn list_subscriptions(
    conn: AsyncDbConnection,
    limit: usize,
) -> Result<Vec<Subscription>> {
    let conn_guard = conn.lock().await;

    let mut stmt =
        conn_guard.prepare("SELECT id FROM subscriptions ORDER BY created_at DESC LIMIT ?")?;

    let ids: Vec<i64> = stmt
        .query_map([limit], |row| row.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;

    drop(stmt);
    drop(conn_guard);

    let mut subscriptions = Vec::new();
    for id in ids {
        if let Ok(subscription) = get_subscription(conn.clone(), id).await {
            subscriptions.push(subscription);
        }
    }

    Ok(subscriptions)
}
