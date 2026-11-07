//! Current authenticated user's cloud-owned Billing. No Space or caller-supplied user ID.
use serde::{Deserialize, Serialize};
pub const SUMMARY: &str = "/app/billing/summary/get";
pub const PAYMENT_LIST: &str = "/app/billing/payment/list";
pub const CREDIT_USAGE: &str = "/app/billing/credit/usage";
pub const CHECKOUT_CREATE: &str = "/app/billing/subscription/checkout/create";
pub const SUBSCRIPTION_CANCEL: &str = "/app/billing/subscription/cancel";
pub const SUBSCRIPTION_RESUME: &str = "/app/billing/subscription/resume";
pub const SUBSCRIPTION_CHANGE: &str = "/app/billing/subscription/change";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProductInput {
    #[serde(deserialize_with = "deserialize_product")]
    pub product_id: String,
}

fn deserialize_product<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<String, D::Error> {
    let value = String::deserialize(deserializer)?;
    if ["go", "plus", "pro", "max"].contains(&value.to_ascii_lowercase().as_str()) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom("Unsupported subscription product"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Checkout {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Subscription {
    pub current_tier: Option<String>,
    pub cancel_at_period_end: bool,
    pub current_period_end_timestamp: i64,
    pub scheduled_tier: Option<String>,
    pub scheduled_tier_start_timestamp: i64,
}
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
    /// Current codes: E (Internal), T/V0 (Free), V1 (Go), V2 (Plus). Null is unknown.
    pub current_tier: Option<String>,
    /// Plan at the next refresh; same codes as current_tier. Null is unknown.
    pub refresh_tier: Option<String>,
    pub stripe_subscription_status: Option<String>,
    /// Period allowance in raw account credits, not remaining credits. 100 credits = 1 App product token.
    pub total_balance: i32,
    /// Used raw account credits; product tokens are distinct from LLM input/output tokens.
    pub used_automation: i32,
    /// Used raw account credits (100 per App product token).
    pub used_chat: i32,
    /// Used raw account credits (100 per App product token).
    pub used_integration: i32,
    /// Unix milliseconds.
    pub next_refresh_timestamp: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Payment {
    pub description: Option<String>,
    /// Signed Stripe invoice/refund minor units for currency. Refunds are negative.
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
    /// Raw account credits (100 per App product token), not LLM input/output tokens.
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
