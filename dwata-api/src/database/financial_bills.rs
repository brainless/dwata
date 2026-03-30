use anyhow::{anyhow, Result};
use shared_types::{Bill, BillStatus, DataSourceType, TransactionCategory};
use sqlx::{QueryBuilder, Row, Sqlite, SqlitePool};

fn data_source_type_to_str(data_source_type: &DataSourceType) -> &'static str {
    match data_source_type {
        DataSourceType::Email => "email",
        DataSourceType::Imap => "imap",
        DataSourceType::BankStatement => "bank-statement",
        DataSourceType::CreditCardStatement => "credit-card-statement",
        DataSourceType::BankFeed => "bank-feed",
        DataSourceType::CsvUpload => "csv-upload",
        DataSourceType::Manual => "manual",
        DataSourceType::Unknown => "unknown",
    }
}

fn data_source_type_from_str(value: &str) -> DataSourceType {
    match value {
        "email" => DataSourceType::Email,
        "imap" => DataSourceType::Imap,
        "bank-statement" => DataSourceType::BankStatement,
        "credit-card-statement" => DataSourceType::CreditCardStatement,
        "bank-feed" => DataSourceType::BankFeed,
        "csv-upload" => DataSourceType::CsvUpload,
        "manual" => DataSourceType::Manual,
        _ => DataSourceType::Unknown,
    }
}

fn bill_status_to_str(status: BillStatus) -> &'static str {
    match status {
        BillStatus::Received => "received",
        BillStatus::Unpaid => "unpaid",
        BillStatus::Paid => "paid",
        BillStatus::Overdue => "overdue",
        BillStatus::Cancelled => "cancelled",
    }
}

fn bill_status_from_str(status: &str) -> BillStatus {
    match status {
        "received" => BillStatus::Received,
        "unpaid" => BillStatus::Unpaid,
        "paid" => BillStatus::Paid,
        "overdue" => BillStatus::Overdue,
        "cancelled" => BillStatus::Cancelled,
        _ => BillStatus::Unpaid,
    }
}

fn category_from_str(value: &str) -> Option<TransactionCategory> {
    match value {
        "income" => Some(TransactionCategory::Income),
        "expense" => Some(TransactionCategory::Expense),
        "investment" => Some(TransactionCategory::Investment),
        "tax" => Some(TransactionCategory::Tax),
        "utility" => Some(TransactionCategory::Utility),
        "subscription" => Some(TransactionCategory::Subscription),
        "entertainment" => Some(TransactionCategory::Entertainment),
        "travel" => Some(TransactionCategory::Travel),
        "healthcare" => Some(TransactionCategory::Healthcare),
        "education" => Some(TransactionCategory::Education),
        "other" => Some(TransactionCategory::Other),
        _ => None,
    }
}

pub fn date_string_to_utc_ms(date: &str) -> Result<i64> {
    let parsed = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|_| anyhow!("Invalid date format `{date}`. Expected YYYY-MM-DD"))?;
    let date_time = parsed
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| anyhow!("Failed to construct date from `{date}`"))?;
    Ok(date_time.and_utc().timestamp_millis())
}

pub async fn insert_financial_bill(pool: &SqlitePool, bill: &Bill) -> Result<i64> {
    let now = chrono::Utc::now().timestamp();
    let data_source_type = data_source_type_to_str(&bill.data_source_type);
    let status = bill_status_to_str(bill.status);

    let inserted_id = sqlx::query_scalar::<_, i64>(
        "INSERT OR IGNORE INTO bills
         (data_source_type, data_source_id, status, category,
          organisation_id, subscription_id, document_reference, total_amount, currency,
          issued_date_raw, issued_date, due_date_raw, due_date,
          billing_period_start_raw, billing_period_start,
          billing_period_end_raw, billing_period_end,
          created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         RETURNING id",
    )
    .bind(data_source_type)
    .bind(&bill.data_source_id)
    .bind(status)
    .bind(bill.category.map(|c| format!("{:?}", c).to_lowercase()))
    .bind(bill.issuer_organisation_id)
    .bind(bill.subscription_id)
    .bind(bill.document_reference.as_deref())
    .bind(bill.total_amount.map(|a| a.to_string()))
    .bind(bill.currency.as_deref())
    .bind(bill.issued_date_raw.as_deref())
    .bind(bill.issued_date)
    .bind(bill.due_date_raw.as_deref())
    .bind(bill.due_date)
    .bind(bill.billing_period_start_raw.as_deref())
    .bind(bill.billing_period_start)
    .bind(bill.billing_period_end_raw.as_deref())
    .bind(bill.billing_period_end)
    .bind(now)
    .bind(now)
    .fetch_optional(pool)
    .await?;

    if let Some(id) = inserted_id {
        return Ok(id);
    }

    let id = if let Some(reference) = bill.document_reference.as_deref() {
        sqlx::query_scalar::<_, i64>(
            "SELECT id FROM bills
             WHERE data_source_type = ? AND data_source_id = ? AND document_reference = ?
             LIMIT 1",
        )
        .bind(data_source_type)
        .bind(&bill.data_source_id)
        .bind(reference)
        .fetch_one(pool)
        .await?
    } else {
        sqlx::query_scalar::<_, i64>(
            "SELECT id FROM bills
             WHERE data_source_type = ? AND data_source_id = ? AND total_amount = ? AND due_date = ?
             LIMIT 1",
        )
        .bind(data_source_type)
        .bind(&bill.data_source_id)
        .bind(bill.total_amount.map(|a| a.to_string()))
        .bind(bill.due_date)
        .fetch_one(pool)
        .await?
    };

    Ok(id)
}

pub async fn list_financial_bills_filtered(
    pool: &SqlitePool,
    status: Option<BillStatus>,
    start_due_date_ms: Option<i64>,
    end_due_date_ms: Option<i64>,
    limit: usize,
    offset: usize,
) -> Result<(Vec<Bill>, usize)> {
    let mut count_qb = QueryBuilder::<Sqlite>::new("SELECT COUNT(*) as cnt FROM bills fb");
    let mut has_where = false;
    let mut push_filter = |qb: &mut QueryBuilder<Sqlite>, clause: &str| {
        if !has_where {
            qb.push(" WHERE ");
            has_where = true;
        } else {
            qb.push(" AND ");
        }
        qb.push(clause);
    };

    if let Some(v) = status {
        push_filter(&mut count_qb, "fb.status = ");
        count_qb.push_bind(bill_status_to_str(v));
    }
    if let Some(v) = start_due_date_ms {
        push_filter(&mut count_qb, "fb.due_date >= ");
        count_qb.push_bind(v);
    }
    if let Some(v) = end_due_date_ms {
        push_filter(&mut count_qb, "fb.due_date <= ");
        count_qb.push_bind(v);
    }

    let total_count: i64 = count_qb.build_query_scalar::<i64>().fetch_one(pool).await?;

    let mut data_qb = QueryBuilder::<Sqlite>::new(
        "SELECT fb.id, fb.data_source_type, fb.data_source_id, fb.status, fb.category,
                fb.organisation_id, fb.subscription_id, fb.document_reference, fb.total_amount, fb.currency,
                fb.issued_date_raw, fb.issued_date, fb.due_date_raw, fb.due_date,
                fb.billing_period_start_raw, fb.billing_period_start,
                fb.billing_period_end_raw, fb.billing_period_end,
                fb.created_at, fb.updated_at
         FROM bills fb",
    );
    let mut has_where = false;
    let mut push_filter = |qb: &mut QueryBuilder<Sqlite>, clause: &str| {
        if !has_where {
            qb.push(" WHERE ");
            has_where = true;
        } else {
            qb.push(" AND ");
        }
        qb.push(clause);
    };

    if let Some(v) = status {
        push_filter(&mut data_qb, "fb.status = ");
        data_qb.push_bind(bill_status_to_str(v));
    }
    if let Some(v) = start_due_date_ms {
        push_filter(&mut data_qb, "fb.due_date >= ");
        data_qb.push_bind(v);
    }
    if let Some(v) = end_due_date_ms {
        push_filter(&mut data_qb, "fb.due_date <= ");
        data_qb.push_bind(v);
    }

    data_qb.push(" ORDER BY fb.due_date ASC, fb.id DESC LIMIT ");
    data_qb.push_bind(limit as i64);
    data_qb.push(" OFFSET ");
    data_qb.push_bind(offset as i64);

    let rows = data_qb.build().fetch_all(pool).await?;
    let bills = rows
        .into_iter()
        .map(|row| -> Result<Bill> {
            let data_source_type_str: Option<String> = row.try_get(1)?;
            let status_str: Option<String> = row.try_get(3)?;
            let category_str: Option<String> = row.try_get(4)?;
            Ok(Bill {
                id: row.try_get(0)?,
                data_source_type: data_source_type_str
                    .as_deref()
                    .map(data_source_type_from_str)
                    .unwrap_or(DataSourceType::Unknown),
                data_source_id: row.try_get(2).unwrap_or_default(),
                status: status_str
                    .as_deref()
                    .map(bill_status_from_str)
                    .unwrap_or(BillStatus::Unpaid),
                category: category_str.as_deref().and_then(category_from_str),
                issuer_organisation_id: row.try_get(5)?,
                subscription_id: row.try_get(6)?,
                document_reference: row.try_get(7)?,
                total_amount: row
                    .try_get::<Option<String>, _>(8)?
                    .and_then(|s| s.parse::<f64>().ok()),
                currency: row.try_get(9)?,
                issued_date_raw: row.try_get(10)?,
                issued_date: row.try_get(11)?,
                due_date_raw: row.try_get(12)?,
                due_date: row.try_get(13)?,
                billing_period_start_raw: row.try_get(14)?,
                billing_period_start: row.try_get(15)?,
                billing_period_end_raw: row.try_get(16)?,
                billing_period_end: row.try_get(17)?,
                created_at: row.try_get(18).unwrap_or(0),
                updated_at: row.try_get(19).unwrap_or(0),
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok((bills, total_count as usize))
}

pub async fn get_financial_bill(pool: &SqlitePool, id: i64) -> Result<Option<Bill>> {
    let row = sqlx::query(
        "SELECT fb.id, fb.data_source_type, fb.data_source_id, fb.status, fb.category,
                fb.organisation_id, fb.subscription_id, fb.document_reference, fb.total_amount, fb.currency,
                fb.issued_date_raw, fb.issued_date, fb.due_date_raw, fb.due_date,
                fb.billing_period_start_raw, fb.billing_period_start,
                fb.billing_period_end_raw, fb.billing_period_end,
                fb.created_at, fb.updated_at
         FROM bills fb
         WHERE fb.id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(row) => {
            let data_source_type_str: Option<String> = row.try_get(1)?;
            let status_str: Option<String> = row.try_get(3)?;
            let category_str: Option<String> = row.try_get(4)?;
            Ok(Some(Bill {
                id: row.try_get(0)?,
                data_source_type: data_source_type_str
                    .as_deref()
                    .map(data_source_type_from_str)
                    .unwrap_or(DataSourceType::Unknown),
                data_source_id: row.try_get(2).unwrap_or_default(),
                status: status_str
                    .as_deref()
                    .map(bill_status_from_str)
                    .unwrap_or(BillStatus::Unpaid),
                category: category_str.as_deref().and_then(category_from_str),
                issuer_organisation_id: row.try_get(5)?,
                subscription_id: row.try_get(6)?,
                document_reference: row.try_get(7)?,
                total_amount: row
                    .try_get::<Option<String>, _>(8)?
                    .and_then(|s| s.parse::<f64>().ok()),
                currency: row.try_get(9)?,
                issued_date_raw: row.try_get(10)?,
                issued_date: row.try_get(11)?,
                due_date_raw: row.try_get(12)?,
                due_date: row.try_get(13)?,
                billing_period_start_raw: row.try_get(14)?,
                billing_period_start: row.try_get(15)?,
                billing_period_end_raw: row.try_get(16)?,
                billing_period_end: row.try_get(17)?,
                created_at: row.try_get(18).unwrap_or(0),
                updated_at: row.try_get(19).unwrap_or(0),
            }))
        }
        None => Ok(None),
    }
}
