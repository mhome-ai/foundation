//! Reliable delivery over the authenticated Node general-webhook request channel.
//! ACK transfers ownership to the receiver's durable inbox, not to an automation result.
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const DELIVERY_SCHEMA: &str = "mhome.webhook.delivery.v1";
pub const ACK_SCHEMA: &str = "mhome.webhook.receipt.v1";
pub const TARGET_PREFIX: &str = "/webhook/general/";
pub const MAX_DELIVERY_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WebhookDelivery {
    pub schema_version: String,
    pub delivery_id: String,
    pub subscription_id: String,
    pub occurred_at_ms: i64,
    pub data: Value,
}

impl WebhookDelivery {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema_version != DELIVERY_SCHEMA {
            return Err("unsupported webhook delivery schema");
        }
        for id in [&self.delivery_id, &self.subscription_id] {
            if id.trim().is_empty() || id.len() > 256 {
                return Err("invalid webhook delivery identity");
            }
        }
        if self.occurred_at_ms < 0 {
            return Err("invalid webhook occurrence time");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebhookReceipt {
    pub schema_version: String,
    pub delivery_id: String,
    pub accepted: bool,
    pub duplicate: bool,
}

impl WebhookReceipt {
    pub fn accepts(&self, delivery_id: &str) -> bool {
        self.schema_version == ACK_SCHEMA && self.delivery_id == delivery_id && self.accepted
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_or_mismatched_ack_cannot_discard_an_outbox_record() {
        assert!(
            serde_json::from_value::<WebhookReceipt>(serde_json::json!({"accepted":true})).is_err()
        );
        let ack = WebhookReceipt {
            schema_version: ACK_SCHEMA.into(),
            delivery_id: "a".into(),
            accepted: true,
            duplicate: false,
        };
        assert!(ack.accepts("a"));
        assert!(!ack.accepts("b"));
    }
    #[test]
    fn envelope_roundtrips_without_changing_capability_data() {
        let delivery = WebhookDelivery {
            schema_version: DELIVERY_SCHEMA.into(),
            delivery_id: "delivery".into(),
            subscription_id: "subscription".into(),
            occurred_at_ms: 1,
            data: serde_json::json!({"occurredAt":{"_type":"Timestamp","value":1}}),
        };
        assert!(delivery.validate().is_ok());
        let decoded: WebhookDelivery =
            serde_json::from_value(serde_json::to_value(&delivery).unwrap()).unwrap();
        assert_eq!(decoded, delivery);
    }
}
