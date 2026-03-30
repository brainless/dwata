use anyhow::{anyhow, Result};
use shared_types::{DataSourceType, Transaction, TransactionStatus};
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

fn transaction_status_to_str(status: &TransactionStatus) -> &'static str {
    match status {
        TransactionStatus::Paid => "paid",
        TransactionStatus::Cancelled => "cancelled",
        TransactionStatus::Refunded => "refunded",
    }
}

fn transaction_status_from_str(value: &str) -> TransactionStatus {
    match value {
        "paid" => TransactionStatus::Paid,
        "cancelled" => TransactionStatus::Cancelled,
        "refunded" => TransactionStatus::Refunded,
        _ => TransactionStatus::Paid,
    }
}

pub async fn insert_financial_transaction(
    pool: &SqlitePool,
    transaction: &Transaction,
) -> Result<i64> {
    let now = chrono::Utc::now().timestamp();

    let data_source_type = data_source_type_to_str(&transaction.data_source_type);
    let status = transaction_status_to_str(&transaction.status);
    let transaction_date_ref = transaction.transaction_date;
    let transaction_reference_ref = transaction.transaction_reference.as_ref();
    let source_file_ref = transaction.source_file.as_ref();

    let inserted_id = sqlx::query_scalar::<_, i64>(
        "INSERT OR IGNORE INTO transactions
         (data_source_type, data_source_id, amount, currency,
          transaction_date_raw, transaction_date, payer_organisation_id, payee_organisation_id,
          status, source_file, bill_id, transaction_reference, extracted_at, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         RETURNING id",
    )
    .bind(data_source_type)
    .bind(&transaction.data_source_id)
    .bind(transaction.amount.to_string())
    .bind(&transaction.currency)
    .bind(&transaction.transaction_date_raw)
    .bind(transaction_date_ref)
    .bind(transaction.payer_organisation_id)
    .bind(transaction.payee_organisation_id)
    .bind(status)
    .bind(source_file_ref)
    .bind(transaction.bill_id)
    .bind(transaction_reference_ref)
    .bind(transaction.extracted_at)
    .bind(now)
    .bind(now)
    .fetch_optional(pool)
    .await?;

    if let Some(id) = inserted_id {
        return Ok(id);
    }

    let id = if let Some(transaction_reference) = transaction_reference_ref {
        sqlx::query_scalar::<_, i64>(
            "SELECT id FROM transactions
             WHERE data_source_type = ? AND data_source_id = ? AND transaction_reference = ?
             LIMIT 1",
        )
        .bind(data_source_type)
        .bind(&transaction.data_source_id)
        .bind(transaction_reference)
        .fetch_one(pool)
        .await?
    } else {
        sqlx::query_scalar::<_, i64>(
            "SELECT id FROM transactions
             WHERE data_source_type = ? AND data_source_id = ? AND amount = ? AND transaction_date = ?
             LIMIT 1",
        )
        .bind(data_source_type)
        .bind(&transaction.data_source_id)
        .bind(transaction.amount.to_string())
        .bind(transaction_date_ref)
        .fetch_one(pool)
        .await?
    };

    Ok(id)
}

pub async fn list_financial_transactions_filtered(
    pool: &SqlitePool,
    payer_organisation_id: Option<i64>,
    payee_organisation_id: Option<i64>,
    bill_id: Option<i64>,
    start_date: Option<i64>,
    end_date: Option<i64>,
    min_amount: Option<f64>,
    max_amount: Option<f64>,
    limit: usize,
    offset: usize,
) -> Result<(Vec<Transaction>, usize)> {
    let mut count_qb = QueryBuilder::<Sqlite>::new(
        "SELECT COUNT(*) as cnt
         FROM transactions ft",
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

    if let Some(v) = payer_organisation_id {
        push_filter(&mut count_qb, "ft.payer_organisation_id = ");
        count_qb.push_bind(v);
    }
    if let Some(v) = payee_organisation_id {
        push_filter(&mut count_qb, "ft.payee_organisation_id = ");
        count_qb.push_bind(v);
    }
    if let Some(v) = bill_id {
        push_filter(&mut count_qb, "ft.bill_id = ");
        count_qb.push_bind(v);
    }
    if let Some(v) = start_date {
        push_filter(&mut count_qb, "ft.transaction_date >= ");
        count_qb.push_bind(v);
    }
    if let Some(v) = end_date {
        push_filter(&mut count_qb, "ft.transaction_date <= ");
        count_qb.push_bind(v);
    }
    if let Some(v) = min_amount {
        push_filter(&mut count_qb, "ft.amount >= ");
        count_qb.push_bind(v);
    }
    if let Some(v) = max_amount {
        push_filter(&mut count_qb, "ft.amount <= ");
        count_qb.push_bind(v);
    }

    let total_count: i64 = count_qb.build_query_scalar::<i64>().fetch_one(pool).await?;

    let mut data_qb = QueryBuilder::<Sqlite>::new(
        "SELECT ft.id, ft.data_source_type, ft.data_source_id, ft.amount, ft.currency,
                ft.transaction_date_raw, ft.transaction_date, ft.payer_organisation_id, ft.payee_organisation_id,
                ft.status, ft.source_file, ft.bill_id, ft.extracted_at, ft.transaction_reference
         FROM transactions ft",
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

    if let Some(v) = payer_organisation_id {
        push_filter(&mut data_qb, "ft.payer_organisation_id = ");
        data_qb.push_bind(v);
    }
    if let Some(v) = payee_organisation_id {
        push_filter(&mut data_qb, "ft.payee_organisation_id = ");
        data_qb.push_bind(v);
    }
    if let Some(v) = bill_id {
        push_filter(&mut data_qb, "ft.bill_id = ");
        data_qb.push_bind(v);
    }
    if let Some(v) = start_date {
        push_filter(&mut data_qb, "ft.transaction_date >= ");
        data_qb.push_bind(v);
    }
    if let Some(v) = end_date {
        push_filter(&mut data_qb, "ft.transaction_date <= ");
        data_qb.push_bind(v);
    }
    if let Some(v) = min_amount {
        push_filter(&mut data_qb, "ft.amount >= ");
        data_qb.push_bind(v);
    }
    if let Some(v) = max_amount {
        push_filter(&mut data_qb, "ft.amount <= ");
        data_qb.push_bind(v);
    }

    data_qb.push(" ORDER BY ft.transaction_date DESC LIMIT ");
    data_qb.push_bind(limit as i64);
    data_qb.push(" OFFSET ");
    data_qb.push_bind(offset as i64);

    let rows = data_qb.build().fetch_all(pool).await?;
    let transactions = rows
        .into_iter()
        .map(|row| -> Result<Transaction> {
            let data_source_type_str: Option<String> = row.try_get(1)?;
            let status_str: Option<String> = row.try_get(9)?;
            let amount_str: Option<String> = row.try_get(3)?;
            let amount = amount_str
                .as_deref()
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);

            Ok(Transaction {
                id: row.try_get(0)?,
                data_source_type: data_source_type_str
                    .as_deref()
                    .map(data_source_type_from_str)
                    .unwrap_or(DataSourceType::Unknown),
                data_source_id: row.try_get(2).unwrap_or_default(),
                amount,
                currency: row.try_get(4).unwrap_or_default(),
                transaction_date_raw: row.try_get(5)?,
                transaction_date: row.try_get(6)?,
                payer_organisation_id: row.try_get(7)?,
                payee_organisation_id: row.try_get(8)?,
                status: status_str
                    .as_deref()
                    .map(transaction_status_from_str)
                    .unwrap_or(TransactionStatus::Paid),
                source_file: row.try_get(10)?,
                bill_id: row.try_get(11)?,
                extracted_at: row.try_get(12).unwrap_or(0),
                transaction_reference: row.try_get(13)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok((transactions, total_count as usize))
}

pub async fn get_financial_transaction(pool: &SqlitePool, id: i64) -> Result<Option<Transaction>> {
    let row = sqlx::query(
        "SELECT ft.id, ft.data_source_type, ft.data_source_id, ft.amount, ft.currency,
                ft.transaction_date_raw, ft.transaction_date, ft.payer_organisation_id, ft.payee_organisation_id,
                ft.status, ft.source_file, ft.bill_id, ft.extracted_at, ft.transaction_reference
         FROM transactions ft
         WHERE ft.id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(row) => {
            let data_source_type_str: Option<String> = row.try_get(1)?;
            let status_str: Option<String> = row.try_get(9)?;
            let amount_str: Option<String> = row.try_get(3)?;
            let amount = amount_str
                .as_deref()
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);

            Ok(Some(Transaction {
                id: row.try_get(0)?,
                data_source_type: data_source_type_str
                    .as_deref()
                    .map(data_source_type_from_str)
                    .unwrap_or(DataSourceType::Unknown),
                data_source_id: row.try_get(2).unwrap_or_default(),
                amount,
                currency: row.try_get(4).unwrap_or_default(),
                transaction_date_raw: row.try_get(5)?,
                transaction_date: row.try_get(6)?,
                payer_organisation_id: row.try_get(7)?,
                payee_organisation_id: row.try_get(8)?,
                status: status_str
                    .as_deref()
                    .map(transaction_status_from_str)
                    .unwrap_or(TransactionStatus::Paid),
                source_file: row.try_get(10)?,
                bill_id: row.try_get(11)?,
                extracted_at: row.try_get(12).unwrap_or(0),
                transaction_reference: row.try_get(13)?,
            }))
        }
        None => Ok(None),
    }
}

pub async fn list_financial_transactions(
    pool: &SqlitePool,
    limit: usize,
) -> Result<Vec<Transaction>> {
    let (transactions, _) = list_financial_transactions_filtered(
        pool, None, None, None, None, None, None, None, limit, 0,
    )
    .await?;
    Ok(transactions)
}
