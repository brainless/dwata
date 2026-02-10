mod extractor;

pub use extractor::FinancialPatternExtractor;

use regex::Regex;
use shared_types::{
    DataSourceType, FinancialDocumentType, FinancialTransaction, TransactionCategory,
    TransactionStatus,
};

pub struct FinancialPattern {
    pub name: String,
    pub regex: Regex,
    pub transaction_type: FinancialDocumentType,
    pub status: TransactionStatus,
    pub amount_group: usize,
    pub vendor_group: Option<usize>,
    pub source_vendor_group: Option<usize>,
    pub destination_vendor_group: Option<usize>,
    pub date_group: Option<usize>,
    pub reference_group: Option<usize>,
    pub currency_group: Option<usize>,
}

impl FinancialPattern {
    pub fn extract(&self, text: &str) -> Option<FinancialTransaction> {
        let captures = self.regex.captures(text)?;

        let amount_str = captures.get(self.amount_group)?.as_str();
        let amount: f64 = amount_str.replace(',', "").parse().ok()?;

        let source_vendor = self
            .source_vendor_group
            .and_then(|g| captures.get(g))
            .map(|m| m.as_str().trim().to_string());

        let destination_vendor = self
            .destination_vendor_group
            .and_then(|g| captures.get(g))
            .map(|m| m.as_str().trim().to_string());

        if source_vendor.is_none() && destination_vendor.is_none() {
            return None;
        }

        let vendor = destination_vendor.clone().or_else(|| source_vendor.clone());

        let transaction_date = if let Some(date_group) = self.date_group {
            captures
                .get(date_group)
                .map(|m| m.as_str().trim().to_string())
        } else {
            None
        };

        let category = match self.transaction_type {
            FinancialDocumentType::PaymentConfirmation => Some(TransactionCategory::Expense),
            FinancialDocumentType::Invoice => Some(TransactionCategory::Income),
            _ => None,
        };

        let currency = self
            .currency_group
            .and_then(|g| captures.get(g))
            .map(|m| m.as_str())
            .and_then(normalize_currency)
            .unwrap_or_else(|| "USD".to_string());

        Some(FinancialTransaction {
            id: 0,
            data_source_type: DataSourceType::Unknown,
            data_source_id: String::new(),
            document_type: self.transaction_type,
            description: format!(
                "{} from {}",
                self.name,
                vendor.as_ref().unwrap_or(&"unknown".to_string())
            ),
            amount,
            currency,
            transaction_date: transaction_date
                .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d").to_string()),
            category,
            vendor,
            source_vendor_id: None,
            destination_vendor_id: None,
            status: self.status,
            source_file: None,
            extracted_at: 0,
            notes: None,
            transaction_reference: self
                .reference_group
                .and_then(|g| captures.get(g))
                .map(|m| m.as_str().trim().to_string()),
        })
    }
}

pub(crate) fn normalize_currency(raw: &str) -> Option<String> {
    let cleaned = raw.trim();
    if cleaned.is_empty() {
        return None;
    }

    let upper = cleaned.to_ascii_uppercase();
    let upper = upper.as_str();

    let iso = match upper {
        "$" | "USD" | "US$" | "US DOLLAR" | "USD$" => "USD",
        "EUR" | "€" => "EUR",
        "GBP" | "£" => "GBP",
        "JPY" | "¥" | "JP¥" => "JPY",
        "INR" | "₹" => "INR",
        "AUD" | "A$" => "AUD",
        "CAD" | "C$" => "CAD",
        "CHF" => "CHF",
        "CNY" | "RMB" | "CN¥" => "CNY",
        "HKD" | "HK$" => "HKD",
        "SGD" | "SG$" => "SGD",
        "NZD" | "NZ$" => "NZD",
        "SEK" => "SEK",
        "NOK" => "NOK",
        "DKK" => "DKK",
        "BRL" | "R$" => "BRL",
        "MXN" | "MX$" => "MXN",
        "ZAR" => "ZAR",
        _ => {
            if upper.len() == 3 && upper.chars().all(|c| c.is_ascii_uppercase()) {
                upper
            } else {
                return None;
            }
        }
    };

    Some(iso.to_string())
}

fn create_financial_patterns() -> Vec<FinancialPattern> {
    vec![
        // Payment Confirmation Patterns
        FinancialPattern {
            name: "payment_to_vendor".to_string(),
            regex: Regex::new(r"(?i)payment of \$?([\d,]+\.?\d{0,2}) to ([A-Za-z\s]+)").unwrap(),
            transaction_type: FinancialDocumentType::PaymentConfirmation,
            status: TransactionStatus::Paid,
            amount_group: 1,
            vendor_group: Some(2),
            date_group: None,
            source_vendor_group: None,
            destination_vendor_group: Some(2),
            reference_group: None,
            currency_group: None,
        },
        FinancialPattern {
            name: "paid_vendor_short".to_string(),
            regex: Regex::new(r"(?i)paid \$?([\d,]+\.?\d{0,2}) to ([A-Za-z\s]+)").unwrap(),
            transaction_type: FinancialDocumentType::PaymentConfirmation,
            status: TransactionStatus::Paid,
            amount_group: 1,
            vendor_group: Some(2),
            date_group: None,
            source_vendor_group: None,
            destination_vendor_group: Some(2),
            reference_group: None,
            currency_group: None,
        },
        FinancialPattern {
            name: "your_payment_to_vendor".to_string(),
            regex: Regex::new(r"(?i)your \$?([\d,]+\.?\d{0,2}) payment to ([A-Za-z\s]+)").unwrap(),
            transaction_type: FinancialDocumentType::PaymentConfirmation,
            status: TransactionStatus::Paid,
            amount_group: 1,
            vendor_group: Some(2),
            date_group: None,
            source_vendor_group: None,
            destination_vendor_group: Some(2),
            reference_group: None,
            currency_group: None,
        },
        FinancialPattern {
            name: "successfully_paid_vendor".to_string(),
            regex: Regex::new(r"(?i)successfully paid \$?([\d,]+\.?\d{0,2}) to ([A-Za-z\s]+)").unwrap(),
            transaction_type: FinancialDocumentType::PaymentConfirmation,
            status: TransactionStatus::Paid,
            amount_group: 1,
            vendor_group: Some(2),
            date_group: None,
            source_vendor_group: None,
            destination_vendor_group: Some(2),
            reference_group: None,
            currency_group: None,
        },
        FinancialPattern {
            name: "payment_processed_vendor".to_string(),
            regex: Regex::new(r"(?i)payment processed: \$?([\d,]+\.?\d{0,2}) to ([A-Za-z\s]+)").unwrap(),
            transaction_type: FinancialDocumentType::PaymentConfirmation,
            status: TransactionStatus::Paid,
            amount_group: 1,
            vendor_group: Some(2),
            date_group: None,
            source_vendor_group: None,
            destination_vendor_group: Some(2),
            reference_group: None,
            currency_group: None,
        },

        // Bill/Invoice Due Patterns
        FinancialPattern {
            name: "bill_due_explicit".to_string(),
            regex: Regex::new(r"(?i)bill (?:of|for) \$?([\d,]+\.?\d{0,2}) (?:is )?due (?:on )?([A-Za-z]+ \d{1,2})").unwrap(),
            transaction_type: FinancialDocumentType::Bill,
            status: TransactionStatus::Pending,
            amount_group: 1,
            vendor_group: None,
            date_group: Some(2),
            source_vendor_group: None,
            destination_vendor_group: None,
            reference_group: None,
            currency_group: None,
        },
        FinancialPattern {
            name: "invoice_for_amount_due".to_string(),
            regex: Regex::new(r"(?i)invoice for \$?([\d,]+\.?\d{0,2}) due ([A-Za-z]+ \d{1,2})").unwrap(),
            transaction_type: FinancialDocumentType::Invoice,
            status: TransactionStatus::Pending,
            amount_group: 1,
            vendor_group: None,
            date_group: Some(2),
            source_vendor_group: None,
            destination_vendor_group: None,
            reference_group: None,
            currency_group: None,
        },
        FinancialPattern {
            name: "your_vendor_bill_due".to_string(),
            regex: Regex::new(r"(?i)your ([A-Za-z\s]+) bill \(\$?([\d,]+\.?\d{0,2})\) is due ([A-Za-z]+ \d{1,2})").unwrap(),
            transaction_type: FinancialDocumentType::Bill,
            status: TransactionStatus::Pending,
            amount_group: 2,
            vendor_group: Some(1),
            date_group: Some(3),
            source_vendor_group: None,
            destination_vendor_group: Some(1),
            reference_group: None,
            currency_group: None,
        },
        FinancialPattern {
            name: "due_by_date".to_string(),
            regex: Regex::new(r"(?i)due: \$?([\d,]+\.?\d{0,2}) by (\d{2}/\d{2}/\d{4})").unwrap(),
            transaction_type: FinancialDocumentType::Bill,
            status: TransactionStatus::Pending,
            amount_group: 1,
            vendor_group: None,
            date_group: Some(2),
            source_vendor_group: None,
            destination_vendor_group: None,
            reference_group: None,
            currency_group: None,
        },

        // Payment Received Patterns
        FinancialPattern {
            name: "payment_received_from".to_string(),
            regex: Regex::new(r"(?i)received \$?([\d,]+\.?\d{0,2}) from ([A-Za-z\s]+)").unwrap(),
            transaction_type: FinancialDocumentType::Invoice,
            status: TransactionStatus::Paid,
            amount_group: 1,
            vendor_group: Some(2),
            date_group: None,
            source_vendor_group: None,
            destination_vendor_group: Some(2),
            reference_group: None,
            currency_group: None,
        },
        FinancialPattern {
            name: "payment_of_amount_from".to_string(),
            regex: Regex::new(r"(?i)payment of \$?([\d,]+\.?\d{0,2}) from ([A-Za-z\s]+)").unwrap(),
            transaction_type: FinancialDocumentType::Invoice,
            status: TransactionStatus::Paid,
            amount_group: 1,
            vendor_group: Some(2),
            date_group: None,
            source_vendor_group: None,
            destination_vendor_group: Some(2),
            reference_group: None,
            currency_group: None,
        },
        FinancialPattern {
            name: "you_received_payment_from".to_string(),
            regex: Regex::new(r"(?i)you received a payment: \$?([\d,]+\.?\d{0,2}) from ([A-Za-z\s]+)").unwrap(),
            transaction_type: FinancialDocumentType::Invoice,
            status: TransactionStatus::Paid,
            amount_group: 1,
            vendor_group: Some(2),
            date_group: None,
            source_vendor_group: None,
            destination_vendor_group: Some(2),
            reference_group: None,
            currency_group: None,
        },

        // Overdue/Late Patterns
        FinancialPattern {
            name: "payment_overdue".to_string(),
            regex: Regex::new(r"(?i)payment of \$?([\d,]+\.?\d{0,2}) is overdue").unwrap(),
            transaction_type: FinancialDocumentType::Bill,
            status: TransactionStatus::Overdue,
            amount_group: 1,
            vendor_group: None,
            date_group: None,
            source_vendor_group: None,
            destination_vendor_group: None,
            reference_group: None,
            currency_group: None,
        },
        FinancialPattern {
            name: "payment_past_due".to_string(),
            regex: Regex::new(r"(?i)\$?([\d,]+\.?\d{0,2}) payment past due").unwrap(),
            transaction_type: FinancialDocumentType::Bill,
            status: TransactionStatus::Overdue,
            amount_group: 1,
            vendor_group: None,
            date_group: None,
            source_vendor_group: None,
            destination_vendor_group: None,
            reference_group: None,
            currency_group: None,
        },
        FinancialPattern {
            name: "overdue_bill_days_late".to_string(),
            regex: Regex::new(r"(?i)overdue bill: \$?([\d,]+\.?\d{0,2}) \((\d+) days late\)").unwrap(),
            transaction_type: FinancialDocumentType::Bill,
            status: TransactionStatus::Overdue,
            amount_group: 1,
            vendor_group: None,
            date_group: None,
            source_vendor_group: None,
            destination_vendor_group: None,
            reference_group: None,
            currency_group: None,
        },

        // Additional patterns for variety
        FinancialPattern {
            name: "subscription_renewal".to_string(),
            regex: Regex::new(r"(?i)subscription renewal: \$?([\d,]+\.?\d{0,2}) for ([A-Za-z\s]+)").unwrap(),
            transaction_type: FinancialDocumentType::PaymentConfirmation,
            status: TransactionStatus::Paid,
            amount_group: 1,
            vendor_group: Some(2),
            date_group: None,
            source_vendor_group: None,
            destination_vendor_group: Some(2),
            reference_group: None,
            currency_group: None,
        },
        FinancialPattern {
            name: "purchase_confirmation".to_string(),
            regex: Regex::new(r"(?i)purchase of \$?([\d,]+\.?\d{0,2}) from ([A-Za-z\s]+)").unwrap(),
            transaction_type: FinancialDocumentType::Receipt,
            status: TransactionStatus::Paid,
            amount_group: 1,
            vendor_group: Some(2),
            date_group: None,
            source_vendor_group: None,
            destination_vendor_group: Some(2),
            reference_group: None,
            currency_group: None,
        },
        FinancialPattern {
            name: "charge_of_amount".to_string(),
            regex: Regex::new(r"(?i)charge of \$?([\d,]+\.?\d{0,2}) to your account from ([A-Za-z\s]+)").unwrap(),
            transaction_type: FinancialDocumentType::Receipt,
            status: TransactionStatus::Paid,
            amount_group: 1,
            vendor_group: Some(2),
            date_group: None,
            source_vendor_group: None,
            destination_vendor_group: Some(2),
            reference_group: None,
            currency_group: None,
        },
        FinancialPattern {
            name: "refund_of_amount".to_string(),
            regex: Regex::new(r"(?i)refund of \$?([\d,]+\.?\d{0,2}) from ([A-Za-z\s]+)").unwrap(),
            transaction_type: FinancialDocumentType::Receipt,
            status: TransactionStatus::Refunded,
            amount_group: 1,
            vendor_group: Some(2),
            date_group: None,
            source_vendor_group: None,
            destination_vendor_group: Some(2),
            reference_group: None,
            currency_group: None,
        },
        FinancialPattern {
            name: "invoice_payment_received".to_string(),
            regex: Regex::new(r"(?i)invoice payment: \$?([\d,]+\.?\d{0,2}) received from ([A-Za-z\s]+)").unwrap(),
            transaction_type: FinancialDocumentType::Invoice,
            status: TransactionStatus::Paid,
            amount_group: 1,
            vendor_group: Some(2),
            date_group: None,
            source_vendor_group: None,
            destination_vendor_group: Some(2),
            reference_group: None,
            currency_group: None,
        },

        // Invoice/Billing notification patterns
        FinancialPattern {
            name: "invoice_amount_line".to_string(),
            regex: Regex::new(r"(?i)invoice amount: \$?([\d,]+\.?\d{0,2})").unwrap(),
            transaction_type: FinancialDocumentType::Bill,
            status: TransactionStatus::Pending,
            amount_group: 1,
            vendor_group: None,
            date_group: None,
            source_vendor_group: None,
            destination_vendor_group: None,
            reference_group: None,
            currency_group: None,
        },
        FinancialPattern {
            name: "amount_paid_line".to_string(),
            regex: Regex::new(r"(?i)amount paid: \$?([\d,]+\.?\d{0,2})").unwrap(),
            transaction_type: FinancialDocumentType::PaymentConfirmation,
            status: TransactionStatus::Paid,
            amount_group: 1,
            vendor_group: None,
            date_group: None,
            source_vendor_group: None,
            destination_vendor_group: None,
            reference_group: None,
            currency_group: None,
        },
        FinancialPattern {
            name: "usage_charges_line".to_string(),
            regex: Regex::new(r"(?i)usage charges[^:]*: \$?([\d,]+\.?\d{0,2})").unwrap(),
            transaction_type: FinancialDocumentType::Bill,
            status: TransactionStatus::Paid,
            amount_group: 1,
            vendor_group: None,
            date_group: None,
            source_vendor_group: None,
            destination_vendor_group: None,
            reference_group: None,
            currency_group: None,
        },
        FinancialPattern {
            name: "total_charged_line".to_string(),
            regex: Regex::new(r"(?i)total charged: \$?([\d,]+\.?\d{0,2})").unwrap(),
            transaction_type: FinancialDocumentType::Receipt,
            status: TransactionStatus::Paid,
            amount_group: 1,
            vendor_group: None,
            date_group: None,
            source_vendor_group: None,
            destination_vendor_group: None,
            reference_group: None,
            currency_group: None,
        },
        FinancialPattern {
            name: "due_on_date".to_string(),
            regex: Regex::new(r"(?i)due on ([A-Za-z]+ \d{1,2}).*?invoice amount: \$?([\d,]+\.?\d{0,2})").unwrap(),
            transaction_type: FinancialDocumentType::Bill,
            status: TransactionStatus::Pending,
            amount_group: 2,
            vendor_group: None,
            date_group: Some(1),
            source_vendor_group: None,
            destination_vendor_group: None,
            reference_group: None,
            currency_group: None,
        },

        // Payment with Rs. (Indian Rupees) and account numbers
        FinancialPattern {
            name: "payment_of_rs_to_vendor".to_string(),
            regex: Regex::new(r"(?i)payment of (?:Rs\.?|INR)[\s]+([\d,]+\.?\d{0,2})[^.]*to ([A-Za-z\s]+)").unwrap(),
            transaction_type: FinancialDocumentType::PaymentConfirmation,
            status: TransactionStatus::Paid,
            amount_group: 1,
            vendor_group: Some(2),
            date_group: None,
            source_vendor_group: None,
            destination_vendor_group: Some(2),
            reference_group: None,
            currency_group: None,
        },
    ]
}
