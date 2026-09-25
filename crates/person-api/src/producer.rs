use crate::EmbeddingSpace;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DescribeRequest {
    pub embedding_space: EmbeddingSpace,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DescribeResponse {
    pub protocol: String,
    pub epoch: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn producer_contract_rejects_policy_negotiation() {
        let model = serde_json::json!({"id":"m","dimensions":2,"recognizerSha256":"a".repeat(64),"preprocessing":"rgb","normalized":true,"metric":"cosine"});
        let input = serde_json::json!({"embeddingSpace": model});
        assert!(serde_json::from_value::<DescribeRequest>(input.clone()).is_ok());
        let mut legacy = input;
        legacy["proposedPolicy"] = serde_json::json!({});
        assert!(serde_json::from_value::<DescribeRequest>(legacy).is_err());
    }
}
