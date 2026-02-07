use crate::database::AsyncDbConnection;
use anyhow::Result;
use shared_types::{
    DataSourceType, FinancialDocumentType, FinancialTransaction, TransactionCategory,
    TransactionStatus, FinancialSummary,
};
use rusqlite::params;

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

pub async fn insert_financial_transaction(
    conn: AsyncDbConnection,
    transaction: &FinancialTransaction,
    extraction_job_id: Option<i64>,
) -> Result<i64> {
    let conn = conn.lock().await;
    let now = chrono::Utc::now().timestamp();

    let document_type = match transaction.document_type {
        FinancialDocumentType::Invoice => "invoice",
        FinancialDocumentType::Bill => "bill",
        FinancialDocumentType::BankStatement => "bank-statement",
        FinancialDocumentType::Receipt => "receipt",
        FinancialDocumentType::TaxDocument => "tax-document",
        FinancialDocumentType::PaymentConfirmation => "payment-confirmation",
    };

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
    let vendor_ref = transaction.vendor.as_deref().unwrap_or("");
    let notes_ref = transaction.notes.as_deref();
    let source_file_ref = transaction.source_file.as_deref();
    let transaction_reference_ref = transaction.transaction_reference.as_deref();

    let id: i64 = conn.query_row(
        "INSERT OR IGNORE INTO financial_transactions
         (data_source_type, data_source_id, extraction_job_id, document_type, description, amount, currency,
          transaction_date, category, vendor, source_vendor_id, destination_vendor_id, status, source_file, confidence,
          requires_review, extracted_at, created_at, updated_at, notes, transaction_reference)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         RETURNING id",
        params![
            data_source_type,
            &transaction.data_source_id,
            extraction_job_id,
            document_type,
            &transaction.description,
            transaction.amount,
            &transaction.currency,
            &transaction.transaction_date,
            category,
            vendor_ref,
            transaction.source_vendor_id,
            transaction.destination_vendor_id,
            status,
            source_file_ref,
            0.85f64,
            false,
            transaction.extracted_at,
            now,
            now,
            notes_ref,
            transaction_reference_ref,
        ],
        |row| row.get(0),
    ).unwrap_or_else(|_| {
        if let Some(transaction_reference) = transaction_reference_ref {
            conn.query_row(
                "SELECT id FROM financial_transactions
                 WHERE data_source_type = ? AND data_source_id = ? AND transaction_reference = ?
                 LIMIT 1",
                params![
                    data_source_type,
                    &transaction.data_source_id,
                    transaction_reference,
                ],
                |row| row.get(0),
            ).unwrap()
        } else {
            conn.query_row(
                "SELECT id FROM financial_transactions
                 WHERE data_source_type = ? AND data_source_id = ? AND amount = ? AND vendor = ? AND transaction_date = ? AND document_type = ?
                 LIMIT 1",
                params![
                    data_source_type,
                    &transaction.data_source_id,
                    transaction.amount,
                    vendor_ref,
                    &transaction.transaction_date,
                    document_type,
                ],
                |row| row.get(0),
            ).unwrap()
        }
    });

    Ok(id)
}

pub async fn list_financial_transactions(
    conn: AsyncDbConnection,
    limit: usize,
) -> Result<Vec<FinancialTransaction>> {
    let conn = conn.lock().await;

    let mut stmt = conn.prepare(
        "SELECT id, data_source_type, data_source_id, document_type, description, amount, currency,
                transaction_date, category, vendor, source_vendor_id, destination_vendor_id, status, source_file,
                extracted_at, notes, transaction_reference
         FROM financial_transactions
         ORDER BY transaction_date DESC
         LIMIT ?",
    )?;

    let rows = stmt.query_map([limit as i64], |row| {
        let data_source_type_str: String = row.get(1)?;
        let document_type_str: String = row.get(3)?;
        let document_type = match document_type_str.as_str() {
            "invoice" => FinancialDocumentType::Invoice,
            "bill" => FinancialDocumentType::Bill,
            "bank-statement" => FinancialDocumentType::BankStatement,
            "receipt" => FinancialDocumentType::Receipt,
            "tax-document" => FinancialDocumentType::TaxDocument,
            "payment-confirmation" => FinancialDocumentType::PaymentConfirmation,
            _ => FinancialDocumentType::Bill,
        };

        let status_str: String = row.get(12)?;
        let status = match status_str.as_str() {
            "paid" => TransactionStatus::Paid,
            "pending" => TransactionStatus::Pending,
            "overdue" => TransactionStatus::Overdue,
            "cancelled" => TransactionStatus::Cancelled,
            "refunded" => TransactionStatus::Refunded,
            _ => TransactionStatus::Pending,
        };

        let category_str: Option<String> = row.get(8)?;
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
            id: row.get(0)?,
            data_source_type: data_source_type_from_str(&data_source_type_str),
            data_source_id: row.get(2)?,
            document_type,
            description: row.get(4)?,
            amount: row.get(5)?,
            currency: row.get(6)?,
            transaction_date: row.get(7)?,
            category,
            vendor: row.get(9)?,
            source_vendor_id: row.get(10)?,
            destination_vendor_id: row.get(11)?,
            status,
            source_file: row.get(13)?,
            extracted_at: row.get(14)?,
            notes: row.get(15)?,
            transaction_reference: row.get(16)?,
        })
    })?;

    let mut transactions = Vec::new();
    for row_result in rows {
        transactions.push(row_result?);
    }

    Ok(transactions)
}

pub async fn get_financial_summary(
    conn: AsyncDbConnection,
    start_date: &str,
    end_date: &str,
) -> Result<FinancialSummary> {
    let conn = conn.lock().await;

    let total_income: f64 = conn.query_row(
        "SELECT COALESCE(SUM(amount), 0.0)
         FROM financial_transactions
         WHERE category = 'income'
           AND transaction_date >= ?
           AND transaction_date <= ?",
        [start_date, end_date],
        |row| row.get(0),
    )?;

    let total_expenses: f64 = conn.query_row(
        "SELECT COALESCE(SUM(ABS(amount)), 0.0)
         FROM financial_transactions
         WHERE category = 'expense'
           AND transaction_date >= ?
           AND transaction_date <= ?",
        [start_date, end_date],
        |row| row.get(0),
    )?;

    let (pending_bills, overdue_payments): (i32, i32) = conn.query_row(
        "SELECT
            COALESCE(SUM(CASE WHEN status = 'pending' THEN 1 ELSE 0 END), 0) as pending,
            COALESCE(SUM(CASE WHEN status = 'overdue' THEN 1 ELSE 0 END), 0) as overdue
         FROM financial_transactions
         WHERE transaction_date >= ?",
        [start_date],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

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
