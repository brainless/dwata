pub const DEFAULT_FINANCIAL_KEYWORDS: &[&str] = &[
    "payment",
    "paid",
    "invoice",
    "receipt",
    "transaction",
    "transfer",
    "deposit",
    "withdrawal",
    "charge",
    "charged",
    "refund",
    "statement",
    "balance",
    "credit",
    "debit",
];

pub fn build_tantivy_query(keywords: &[&str]) -> String {
    let mut parts = Vec::with_capacity(keywords.len());
    for keyword in keywords {
        let escaped = keyword.replace('"', " ");
        if !escaped.trim().is_empty() {
            parts.push(escaped);
        }
    }
    parts.join(" OR ")
}
