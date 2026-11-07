//! Current authenticated user's cloud-owned Billing reads. No Space or caller-supplied user ID.
use serde::{Deserialize, Serialize};
pub const SUMMARY: &str = "/app/billing/summary/get";
pub const PAYMENT_LIST: &str = "/app/billing/payment/list";
pub const CREDIT_USAGE: &str = "/app/billing/credit/usage";
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EmptyInput {}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreditUsageInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_date: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_date: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Summary {
    pub user_id: String,
    pub current_tier: Option<String>,
    pub refresh_tier: Option<String>,
    pub stripe_subscription_status: Option<String>,
    pub total_balance: i32,
    pub used_automation: i32,
    pub used_chat: i32,
    pub used_integration: i32,
    pub next_refresh_timestamp: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Payment {
    pub description: Option<String>,
    /// Signed currency minor units. Refunds are negative.
    pub amount: i32,
    pub currency: String,
    pub timestamp: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaymentList {
    pub payments: Vec<Payment>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreditRow {
    pub date: String,
    pub source: String,
    pub credits: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreditUsage {
    pub audience: String,
    pub timezone: String,
    pub from_date: String,
    pub to_date: String,
    pub tracked_from: String,
    pub rows: Vec<CreditRow>,
}
