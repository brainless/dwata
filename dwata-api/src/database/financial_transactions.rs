use anyhow::Result;
use shared_types::{
    DataSourceType, FinancialSummary, FinancialTransaction, TransactionCategory, TransactionParty,
    TransactionStatus,
};
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

fn mk_party(vendor_id: Option<i64>) -> TransactionParty {
    TransactionParty { vendor_id }
}

pub async fn insert_financial_transaction(
    pool: &SqlitePool,
    transaction: &FinancialTransaction,
) -> Result<i64> {
    let now = chrono::Utc::now().timestamp();

    let status = match transaction.status {
        TransactionStatus::Paid => "paid",
        TransactionStatus::Pending => "pending",
        TransactionStatus::Overdue => "overdue",
        TransactionStatus::Cancelled => "cancelled",
        TransactionStatus::Refunded => "refunded",
    };

    let category = transaction.category.as_ref().map(|c| match c {
        TransactionCategory::Income => "income",
        TransactionCategory::Expense => "expense",
        TransactionCategory::Investment => "investment",
        TransactionCategory::Tax => "tax",
        TransactionCategory::Utility => "utility",
        TransactionCategory::Subscription => "subscription",
        TransactionCategory::Entertainment => "entertainment",
        TransactionCategory::Travel => "travel",
        TransactionCategory::Healthcare => "healthcare",
        TransactionCategory::Education => "education",
        TransactionCategory::Other => "other",
    });

    let data_source_type = data_source_type_to_str(&transaction.data_source_type);
    let notes_ref = transaction.notes.as_ref();
    let source_file_ref = transaction.source_file.as_ref();
    let transaction_reference_ref = transaction.transaction_reference.as_ref();

    let inserted_id = sqlx::query_scalar::<_, i64>(
        "INSERT OR IGNORE INTO financial_transactions
         (data_source_type, data_source_id, amount, currency,
          transaction_date, category, source_vendor_id, destination_vendor_id, status, source_file,
          requires_review, extracted_at, created_at, updated_at, notes, transaction_reference)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         RETURNING id",
    )
    .bind(data_source_type)
    .bind(&transaction.data_source_id)
    .bind(transaction.amount)
    .bind(&transaction.currency)
    .bind(&transaction.transaction_date)
    .bind(category)
    .bind(transaction.payer.vendor_id)
    .bind(transaction.payee.vendor_id)
    .bind(status)
    .bind(source_file_ref)
    .bind(false)
    .bind(transaction.extracted_at)
    .bind(now)
    .bind(now)
    .bind(notes_ref)
    .bind(transaction_reference_ref)
    .fetch_optional(pool)
    .await?;

    if let Some(id) = inserted_id {
        return Ok(id);
    }

    let id = if let Some(transaction_reference) = transaction_reference_ref {
        sqlx::query_scalar::<_, i64>(
            "SELECT id FROM financial_transactions
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
            "SELECT id FROM financial_transactions
             WHERE data_source_type = ? AND data_source_id = ? AND amount = ? AND transaction_date = ?
             LIMIT 1",
        )
        .bind(data_source_type)
        .bind(&transaction.data_source_id)
        .bind(transaction.amount)
        .bind(&transaction.transaction_date)
        .fetch_one(pool)
        .await?
    };

    Ok(id)
}

pub async fn list_financial_transactions_filtered(
    pool: &SqlitePool,
    source_vendor_id: Option<i64>,
    destination_vendor_id: Option<i64>,
    start_date: Option<&str>,
    end_date: Option<&str>,
    min_amount: Option<f64>,
    max_amount: Option<f64>,
    limit: usize,
    offset: usize,
) -> Result<(Vec<FinancialTransaction>, usize)> {
    let mut count_qb = QueryBuilder::<Sqlite>::new(
        "SELECT COUNT(*) as cnt
         FROM financial_transactions ft",
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

    if let Some(v) = source_vendor_id {
        push_filter(&mut count_qb, "ft.source_vendor_id = ");
        count_qb.push_bind(v);
    }
    if let Some(v) = destination_vendor_id {
        push_filter(&mut count_qb, "ft.destination_vendor_id = ");
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
                ft.transaction_date, ft.category, ft.source_vendor_id, ft.destination_vendor_id, ft.status, ft.source_file,
                ft.extracted_at, ft.notes, ft.transaction_reference
         FROM financial_transactions ft",
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

    if let Some(v) = source_vendor_id {
        push_filter(&mut data_qb, "ft.source_vendor_id = ");
        data_qb.push_bind(v);
    }
    if let Some(v) = destination_vendor_id {
        push_filter(&mut data_qb, "ft.destination_vendor_id = ");
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
        .map(|row| -> Result<FinancialTransaction> {
            let data_source_type_str: String = row.try_get(1)?;
            let status_str: String = row.try_get(9)?;
            let status = match status_str.as_str() {
                "paid" => TransactionStatus::Paid,
                "pending" => TransactionStatus::Pending,
                "overdue" => TransactionStatus::Overdue,
                "cancelled" => TransactionStatus::Cancelled,
                "refunded" => TransactionStatus::Refunded,
                _ => TransactionStatus::Pending,
            };

            let category_str: Option<String> = row.try_get(6)?;
            let category = category_str.map(|c| match c.as_str() {
                "income" => TransactionCategory::Income,
                "expense" => TransactionCategory::Expense,
                "investment" => TransactionCategory::Investment,
                "tax" => TransactionCategory::Tax,
                "utility" => TransactionCategory::Utility,
                "subscription" => TransactionCategory::Subscription,
                "entertainment" => TransactionCategory::Entertainment,
                "travel" => TransactionCategory::Travel,
                "healthcare" => TransactionCategory::Healthcare,
                "education" => TransactionCategory::Education,
                _ => TransactionCategory::Other,
            });

            Ok(FinancialTransaction {
                id: row.try_get(0)?,
                data_source_type: data_source_type_from_str(&data_source_type_str),
                data_source_id: row.try_get(2)?,
                amount: row.try_get(3)?,
                currency: row.try_get(4)?,
                transaction_date: row.try_get(5)?,
                category,
                payer: mk_party(row.try_get(7)?),
                payee: mk_party(row.try_get(8)?),
                status,
                source_file: row.try_get(10)?,
                extracted_at: row.try_get(11)?,
                notes: row.try_get(12)?,
                transaction_reference: row.try_get(13)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok((transactions, total_count as usize))
}

pub async fn list_financial_transactions(
    pool: &SqlitePool,
    limit: usize,
) -> Result<Vec<FinancialTransaction>> {
    let (transactions, _) =
        list_financial_transactions_filtered(pool, None, None, None, None, None, None, limit, 0)
            .await?;
    Ok(transactions)
}

pub async fn get_financial_summary(
    pool: &SqlitePool,
    start_date: &str,
    end_date: &str,
) -> Result<FinancialSummary> {
    let total_income: f64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(amount), 0.0)
         FROM financial_transactions
         WHERE category = 'income'
           AND transaction_date >= ?
           AND transaction_date <= ?",
    )
    .bind(start_date)
    .bind(end_date)
    .fetch_one(pool)
    .await?;

    let total_expenses: f64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(ABS(amount)), 0.0)
         FROM financial_transactions
         WHERE category = 'expense'
           AND transaction_date >= ?
           AND transaction_date <= ?",
    )
    .bind(start_date)
    .bind(end_date)
    .fetch_one(pool)
    .await?;

    let pending_statuses = sqlx::query(
        "SELECT
            COALESCE(SUM(CASE WHEN status = 'pending' THEN 1 ELSE 0 END), 0) as pending,
            COALESCE(SUM(CASE WHEN status = 'overdue' THEN 1 ELSE 0 END), 0) as overdue
         FROM financial_transactions
         WHERE transaction_date >= ?",
    )
    .bind(start_date)
    .fetch_one(pool)
    .await?;
    let pending_bills: i32 = pending_statuses.try_get(0)?;
    let overdue_payments: i32 = pending_statuses.try_get(1)?;

    Ok(FinancialSummary {
        total_income,
        total_expenses,
        net_balance: total_income - total_expenses,
        pending_bills,
        overdue_payments,
        currency: "USD".to_string(),
        period_start: start_date.to_string(),
        period_end: end_date.to_string(),
    })
}
