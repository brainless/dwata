use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Type of extraction pass
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum KgPassType {
    IdentityResolution,
    FinancialExtraction,
    EventExtraction,
    OrderExtraction,
}

impl KgPassType {
    pub fn name(&self) -> &'static str {
        match self {
            KgPassType::IdentityResolution => "identity_resolution",
            KgPassType::FinancialExtraction => "financial_extraction",
            KgPassType::EventExtraction => "event_extraction",
            KgPassType::OrderExtraction => "order_extraction",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            KgPassType::IdentityResolution => {
                "Extract locations, organisations, and persons with their relationships"
            }
            KgPassType::FinancialExtraction => {
                "Extract bills, transactions, and subscriptions linked to identified entities"
            }
            KgPassType::EventExtraction => "Extract calendar events and meetings",
            KgPassType::OrderExtraction => "Extract e-commerce orders and shipments",
        }
    }
}
