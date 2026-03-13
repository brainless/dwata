use crate::date_parser::{parse_to_iso, parse_to_utc_timestamp_ms};
use calamine::{open_workbook_auto, Data, Reader};
use shared_types::{DataSourceType, Transaction, TransactionStatus};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ColumnarSheet {
    pub name: String,
    pub intro_lines: Vec<String>,
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct StatementTemplate {
    pub sheet_name: String,
    pub placeholders: Vec<(String, String)>,
    pub row_template: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatementField {
    Amount,
    Currency,
    TransactionDate,
    Vendor,
    TransactionReference,
}

pub fn read_statement_sheets(
    path: &Path,
    sheet_filter: Option<&str>,
) -> anyhow::Result<Vec<ColumnarSheet>> {
    let mut workbook = open_workbook_auto(path)?;
    let mut out = Vec::new();

    for name in workbook.sheet_names().to_owned() {
        if let Some(filter) = sheet_filter {
            if name != filter {
                continue;
            }
        }
        let Ok(range) = workbook.worksheet_range(&name) else {
            continue;
        };
        if let Some(sheet) =
            to_columnar_sheet(&name, &range.rows().map(|r| r.to_vec()).collect::<Vec<_>>())
        {
            out.push(sheet);
        }
    }
    Ok(out)
}

fn to_columnar_sheet(name: &str, rows: &[Vec<Data>]) -> Option<ColumnarSheet> {
    let text_rows: Vec<Vec<String>> = rows
        .iter()
        .map(|row| row.iter().map(cell_to_string).collect::<Vec<_>>())
        .collect();

    let header_idx = detect_header_row(&text_rows)?;
    let raw_headers = text_rows.get(header_idx)?.clone();
    let headers = normalize_headers(&raw_headers);

    let intro_lines = text_rows
        .iter()
        .take(header_idx)
        .filter_map(|row| {
            let line = row
                .iter()
                .filter(|s| !s.trim().is_empty())
                .cloned()
                .collect::<Vec<_>>()
                .join(" | ");
            if line.trim().is_empty() {
                None
            } else {
                Some(line)
            }
        })
        .collect::<Vec<_>>();

    let mut data_rows = Vec::new();
    let mut blank_streak = 0usize;
    for row in text_rows.iter().skip(header_idx + 1) {
        let non_empty = row.iter().filter(|s| !s.trim().is_empty()).count();
        if non_empty < 2 {
            blank_streak += 1;
            if blank_streak >= 3 && !data_rows.is_empty() {
                break;
            }
            continue;
        }
        blank_streak = 0;
        data_rows.push(trim_or_pad(row, headers.len()));
    }

    if data_rows.is_empty() {
        return None;
    }

    Some(ColumnarSheet {
        name: name.to_string(),
        intro_lines,
        headers,
        rows: data_rows,
    })
}

pub fn build_template(sheet: &ColumnarSheet) -> StatementTemplate {
    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut placeholders = Vec::with_capacity(sheet.headers.len());
    for h in &sheet.headers {
        let base = normalize_placeholder(h);
        let n = counts.entry(base.clone()).or_insert(0);
        *n += 1;
        let ph = if *n == 1 { base } else { format!("{base}_{n}") };
        placeholders.push((h.clone(), ph));
    }
    let row_template = placeholders
        .iter()
        .map(|(_, ph)| format!("{{{{ {ph} }}}}"))
        .collect::<Vec<_>>()
        .join(" | ");

    StatementTemplate {
        sheet_name: sheet.name.clone(),
        placeholders,
        row_template,
    }
}

pub fn infer_field_mapping(headers: &[String]) -> HashMap<String, StatementField> {
    let mut map = HashMap::new();
    let mut saw_amount = false;
    for h in headers {
        let n = h.to_ascii_lowercase();
        if !saw_amount
            && (n.contains("amount")
                || n.contains("withdrawal")
                || n.contains("deposit")
                || n.contains("debit")
                || n.contains("credit"))
        {
            map.insert(h.clone(), StatementField::Amount);
            saw_amount = true;
            continue;
        }
        if n.contains("date") || n == "dt" {
            map.insert(h.clone(), StatementField::TransactionDate);
            continue;
        }
        if n.contains("narration") || n.contains("description") || n.contains("particular") {
            map.insert(h.clone(), StatementField::Vendor);
            continue;
        }
        if n.contains("ref") || n.contains("chq") || n.contains("utr") || n.contains("txn") {
            map.insert(h.clone(), StatementField::TransactionReference);
            continue;
        }
        if n.contains("currency") || n == "ccy" {
            map.insert(h.clone(), StatementField::Currency);
            continue;
        }
    }
    map
}

pub fn extract_transactions(
    sheet: &ColumnarSheet,
    field_map: &HashMap<String, StatementField>,
    default_currency: Option<&str>,
) -> Vec<Transaction> {
    let header_index = sheet
        .headers
        .iter()
        .enumerate()
        .map(|(i, h)| (h, i))
        .collect::<HashMap<_, _>>();

    let mut transactions = Vec::new();
    let extracted_at = chrono::Utc::now().timestamp_millis();
    for (row_idx, row) in sheet.rows.iter().enumerate() {
        let raw_row = sheet
            .headers
            .iter()
            .cloned()
            .zip(row.iter().cloned())
            .collect::<HashMap<_, _>>();

        let mut date = None;
        let mut amount = None;
        let mut currency = default_currency.map(|s| s.to_string());
        let mut vendor = None;
        let mut reference = None;

        for (header, field) in field_map {
            let Some(idx) = header_index.get(header).copied() else {
                continue;
            };
            let value = row.get(idx).map(String::as_str).unwrap_or("").trim();
            if value.is_empty() {
                continue;
            }
            match field {
                StatementField::TransactionDate => {
                    if date.is_none() {
                        date = parse_to_iso(value);
                    }
                }
                StatementField::Amount => {
                    if amount.is_none() {
                        let mut parsed = parse_amount(value);
                        let header_norm = header.to_ascii_lowercase();
                        if (header_norm.contains("withdrawal") || header_norm.contains("debit"))
                            && parsed.is_some()
                        {
                            parsed = parsed.map(|x| -x.abs());
                        }
                        if (header_norm.contains("deposit") || header_norm.contains("credit"))
                            && parsed.is_some()
                        {
                            parsed = parsed.map(|x| x.abs());
                        }
                        amount = parsed;
                    }
                }
                StatementField::Currency => {
                    currency = Some(value.to_string());
                }
                StatementField::Vendor => {
                    vendor = Some(value.to_string());
                }
                StatementField::TransactionReference => {
                    reference = Some(value.to_string());
                }
            }
        }

        if amount.is_none() {
            amount = amount_from_dr_cr(sheet, row);
        }
        if let (Some(txn_date), Some(amount)) = (date, amount) {
            let parsed_date = parse_to_utc_timestamp_ms(&txn_date);
            transactions.push(Transaction {
                id: 0,
                data_source_type: DataSourceType::BankStatement,
                data_source_id: format!("{}:{}", sheet.name, row_idx + 1),
                transaction_date_raw: Some(txn_date),
                transaction_date: parsed_date,
                amount,
                currency: currency.clone().unwrap_or_else(|| "UNK".to_string()),
                payer_organisation_id: None,
                payee_organisation_id: None,
                status: TransactionStatus::Paid,
                source_file: None,
                extracted_at,
                bill_id: None,
                transaction_reference: reference.clone(),
            });
        }
    }
    transactions
}

pub fn infer_currency_from_intro(intro_lines: &[String]) -> Option<String> {
    for line in intro_lines {
        let upper = line.to_ascii_uppercase();
        for token in upper.split(|c: char| !c.is_ascii_alphanumeric()) {
            if token.len() == 3 && token.chars().all(|c| c.is_ascii_alphabetic()) {
                if matches!(token, "INR" | "USD" | "EUR" | "GBP" | "JPY" | "AUD" | "CAD") {
                    return Some(token.to_string());
                }
            }
        }
    }
    None
}

fn amount_from_dr_cr(sheet: &ColumnarSheet, row: &[String]) -> Option<f64> {
    let mut debit_idx = None;
    let mut credit_idx = None;
    for (i, h) in sheet.headers.iter().enumerate() {
        let n = h.to_ascii_lowercase();
        if debit_idx.is_none() && (n.contains("withdrawal") || n.contains("debit")) {
            debit_idx = Some(i);
        }
        if credit_idx.is_none() && (n.contains("deposit") || n.contains("credit")) {
            credit_idx = Some(i);
        }
    }
    if let Some(i) = debit_idx {
        let v = row.get(i).map(String::as_str).unwrap_or("").trim();
        if let Some(x) = parse_amount(v) {
            return Some(-x.abs());
        }
    }
    if let Some(i) = credit_idx {
        let v = row.get(i).map(String::as_str).unwrap_or("").trim();
        if let Some(x) = parse_amount(v) {
            return Some(x.abs());
        }
    }
    None
}

fn parse_amount(raw: &str) -> Option<f64> {
    let cleaned = raw
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.' || *c == '-' || *c == ',')
        .collect::<String>()
        .replace(',', "");
    if cleaned.is_empty() || cleaned == "-" || cleaned == "." || cleaned == "-." {
        return None;
    }
    cleaned.parse::<f64>().ok()
}

fn detect_header_row(rows: &[Vec<String>]) -> Option<usize> {
    let mut best: Option<(usize, i32)> = None;
    for (i, row) in rows.iter().take(80).enumerate() {
        let non_empty = row.iter().filter(|s| !s.trim().is_empty()).count();
        if non_empty < 3 {
            continue;
        }
        let joined = row.join(" ").to_ascii_lowercase();
        let mut score = 0;
        for token in [
            "date",
            "narration",
            "description",
            "ref",
            "withdrawal",
            "deposit",
            "debit",
            "credit",
            "amount",
            "balance",
        ] {
            if joined.contains(token) {
                score += 1;
            }
        }
        if score >= 2 {
            if let Some((_, best_score)) = best {
                if score > best_score {
                    best = Some((i, score));
                }
            } else {
                best = Some((i, score));
            }
        }
    }
    best.map(|(i, _)| i)
}

fn normalize_headers(headers: &[String]) -> Vec<String> {
    headers
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let t = h.trim();
            if t.is_empty() {
                format!("column_{}", i + 1)
            } else {
                t.to_string()
            }
        })
        .collect()
}

fn normalize_placeholder(s: &str) -> String {
    let mut out = String::new();
    for ch in s.to_ascii_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    let trimmed = out.trim_matches('_');
    if trimmed.is_empty() {
        "column".to_string()
    } else {
        trimmed.to_string()
    }
}

fn trim_or_pad(row: &[String], target: usize) -> Vec<String> {
    let mut out = row.iter().take(target).cloned().collect::<Vec<_>>();
    while out.len() < target {
        out.push(String::new());
    }
    out
}

fn cell_to_string(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        _ => cell.to_string().trim().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_header() {
        let rows = vec![
            vec!["Account".to_string(), "".to_string()],
            vec![
                "Date".to_string(),
                "Narration".to_string(),
                "Withdrawal Amt.".to_string(),
                "Deposit Amt.".to_string(),
            ],
        ];
        assert_eq!(detect_header_row(&rows), Some(1));
    }

    #[test]
    fn extracts_typed_rows() {
        let sheet = ColumnarSheet {
            name: "x".to_string(),
            intro_lines: vec!["Currency : INR".to_string()],
            headers: vec![
                "Date".to_string(),
                "Narration".to_string(),
                "Withdrawal Amt.".to_string(),
                "Deposit Amt.".to_string(),
                "Chq./Ref.No.".to_string(),
            ],
            rows: vec![
                vec![
                    "2013-05-16".to_string(),
                    "Vendor A".to_string(),
                    "100.50".to_string(),
                    "".to_string(),
                    "R1".to_string(),
                ],
                vec![
                    "2013-05-17".to_string(),
                    "Vendor B".to_string(),
                    "".to_string(),
                    "200".to_string(),
                    "R2".to_string(),
                ],
            ],
        };
        let map = infer_field_mapping(&sheet.headers);
        let txns = extract_transactions(&sheet, &map, Some("INR"));
        assert_eq!(txns.len(), 2);
        assert_eq!(txns[0].amount, -100.5);
        assert_eq!(txns[1].amount, 200.0);
    }
}
