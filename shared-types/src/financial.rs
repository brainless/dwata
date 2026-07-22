use serde::{Deserialize, Serialize};

use crate::bill::Bill;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialPagination {
    pub page: usize,
    pub limit: usize,
    pub total_count: usize,
    pub total_pages: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListFinancialBillsResponse {
    pub bills: Vec<Bill>,
    pub pagination: FinancialPagination,
}
