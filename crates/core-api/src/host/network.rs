//! Host-owned IPv4 LAN observations, shared through local IPC.
use serde::{Deserialize, Serialize};
use std::net::Ipv4Addr;

pub const NETWORK_REFRESH_INTERVAL_SECS: u64 = 15;
/// Allows for both the Host sampling interval and a consumer's polling interval.
pub const NETWORK_MAX_AGE_MS: i64 = 45_000;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostNetworkSnapshot {
    pub lan_ipv4: Option<Ipv4Addr>,
    pub observed_at_ms: i64,
}

impl HostNetworkSnapshot {
    pub fn available_ipv4(&self, now_ms: i64) -> Option<Ipv4Addr> {
        let age = now_ms.checked_sub(self.observed_at_ms)?;
        self.lan_ipv4.filter(|ip| {
            ip.is_private() && self.observed_at_ms > 0 && (0..NETWORK_MAX_AGE_MS).contains(&age)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_fresh_private_ipv4_is_available() {
        for ip in ["192.168.0.2", "10.1.2.3", "172.31.2.3"] {
            let snapshot = HostNetworkSnapshot {
                lan_ipv4: Some(ip.parse().unwrap()),
                observed_at_ms: 100_000,
            };
            assert_eq!(snapshot.available_ipv4(100_000), snapshot.lan_ipv4);
            assert_eq!(snapshot.available_ipv4(144_999), snapshot.lan_ipv4);
            assert_eq!(snapshot.available_ipv4(145_000), None);
            assert_eq!(snapshot.available_ipv4(99_999), None);
        }
        for ip in [
            "127.0.0.1",
            "0.0.0.0",
            "8.8.8.8",
            "169.254.1.2",
            "172.32.1.2",
        ] {
            assert_eq!(
                HostNetworkSnapshot {
                    lan_ipv4: Some(ip.parse().unwrap()),
                    observed_at_ms: 100_000,
                }
                .available_ipv4(100_000),
                None
            );
        }
        assert_eq!(HostNetworkSnapshot::default().available_ipv4(1), None);
    }
}
