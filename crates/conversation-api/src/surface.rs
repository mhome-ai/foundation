use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;

pub const FIRST_PARTY_SURFACE_ID: &str = "meow-link";
const MESSAGING_SURFACE_PREFIX: &str = "messaging";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessagingSurfaceId {
    pub provider: String,
    pub account_id: String,
    pub conversation_id: String,
}

pub fn messaging_surface_id(
    provider: &str,
    account_id: &str,
    conversation_id: &str,
) -> Option<String> {
    let provider = normalize_provider(provider)?;
    let account_id = account_id.trim();
    let conversation_id = conversation_id.trim();
    if account_id.is_empty() || conversation_id.is_empty() {
        return None;
    }
    Some(format!(
        "{MESSAGING_SURFACE_PREFIX}:{provider}:{}:{}",
        encode_component(account_id),
        encode_component(conversation_id)
    ))
}

pub fn parse_messaging_surface_id(surface_id: &str) -> Option<MessagingSurfaceId> {
    let mut parts = surface_id.split(':');
    if parts.next()? != MESSAGING_SURFACE_PREFIX {
        return None;
    }
    let provider = normalize_provider(parts.next()?)?;
    let account_id = decode_component(parts.next()?)?;
    let conversation_id = decode_component(parts.next()?)?;
    if parts.next().is_some() || account_id.is_empty() || conversation_id.is_empty() {
        return None;
    }
    Some(MessagingSurfaceId {
        provider,
        account_id,
        conversation_id,
    })
}

fn normalize_provider(provider: &str) -> Option<String> {
    let provider = provider.trim().to_ascii_lowercase();
    (!provider.is_empty()
        && provider
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_'))
    .then_some(provider)
}

fn encode_component(value: &str) -> String {
    URL_SAFE_NO_PAD.encode(value.as_bytes())
}

fn decode_component(value: &str) -> Option<String> {
    let decoded = URL_SAFE_NO_PAD.decode(value).ok()?;
    if URL_SAFE_NO_PAD.encode(&decoded) != value {
        return None;
    }
    let decoded = String::from_utf8(decoded).ok()?;
    (!decoded.trim().is_empty() && decoded.trim() == decoded).then_some(decoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messaging_surface_round_trips_without_delimiter_ambiguity() {
        let surface = messaging_surface_id(" Telegram ", "bot:1", "chat:2").unwrap();
        assert_eq!(surface, "messaging:telegram:Ym90OjE:Y2hhdDoy");
        assert_eq!(
            parse_messaging_surface_id(&surface),
            Some(MessagingSurfaceId {
                provider: "telegram".to_string(),
                account_id: "bot:1".to_string(),
                conversation_id: "chat:2".to_string(),
            })
        );
    }

    #[test]
    fn invalid_or_empty_components_are_rejected() {
        assert!(messaging_surface_id("not-valid!", "a", "c").is_none());
        assert!(messaging_surface_id("feishu", "", "c").is_none());
        assert!(parse_messaging_surface_id("messaging:feishu:bad!:bad!").is_none());
    }

    #[test]
    fn messaging_surface_rejects_non_canonical_components() {
        assert!(parse_messaging_surface_id("messaging:telegram:Ym90OjE=:Y2hhdDoy").is_none());
        assert!(parse_messaging_surface_id("messaging:telegram:IGJvdCA:Y2hhdDoy").is_none());
    }
}
