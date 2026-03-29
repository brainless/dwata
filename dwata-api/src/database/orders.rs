use crate::database::AsyncDbConnection;
use anyhow::Result;
use shared_types::{Order, OrderItem, OrderStatus};

fn order_status_from_str(value: &str) -> Option<OrderStatus> {
    match value {
        "placed" => Some(OrderStatus::Placed),
        "confirmed" => Some(OrderStatus::Confirmed),
        "shipped" => Some(OrderStatus::Shipped),
        "out_for_delivery" => Some(OrderStatus::OutForDelivery),
        "delivered" => Some(OrderStatus::Delivered),
        "cancelled" => Some(OrderStatus::Cancelled),
        "returned" => Some(OrderStatus::Returned),
        "refunded" => Some(OrderStatus::Refunded),
        _ => Some(OrderStatus::Unknown),
    }
}

pub async fn get_order(conn: AsyncDbConnection, id: i64) -> Result<Order> {
    let conn = conn.lock().await;

    let mut stmt = conn.prepare(
        "SELECT id, organisation_id, order_reference, order_date_raw, order_date, status,
                total_amount, currency, items, tracking_number, transaction_id,
                source_email_id, created_at, updated_at
         FROM orders
         WHERE id = ?",
    )?;

    stmt.query_row([id], |row| {
        let items_json: Option<String> = row.get(8)?;
        let items: Vec<OrderItem> = items_json
            .and_then(|json| serde_json::from_str(&json).ok())
            .unwrap_or_default();

        let status_str: Option<String> = row.get(5)?;
        let status = status_str.as_deref().and_then(|s| order_status_from_str(s));

        Ok(Order {
            id: row.get(0)?,
            organisation_id: row.get(1)?,
            order_reference: row.get(2)?,
            order_date_raw: row.get(3)?,
            order_date: row.get(4)?,
            status,
            total_amount: row.get(6)?,
            currency: row.get(7)?,
            items,
            tracking_number: row.get(9)?,
            transaction_id: row.get(10)?,
            source_email_id: row.get(11)?,
            created_at: row.get(12)?,
            updated_at: row.get(13)?,
        })
    })
    .map_err(|e| anyhow::anyhow!("Failed to get order: {}", e))
}

pub async fn list_orders(conn: AsyncDbConnection, limit: usize) -> Result<Vec<Order>> {
    let conn_guard = conn.lock().await;

    let mut stmt = conn_guard.prepare("SELECT id FROM orders ORDER BY created_at DESC LIMIT ?")?;

    let ids: Vec<i64> = stmt
        .query_map([limit], |row| row.get::<_, i64>(0))?
        .collect::<Result<Vec<_>, _>>()?;

    drop(stmt);
    drop(conn_guard);

    let mut orders = Vec::new();
    for id in ids {
        if let Ok(order) = get_order(conn.clone(), id).await {
            orders.push(order);
        }
    }

    Ok(orders)
}
