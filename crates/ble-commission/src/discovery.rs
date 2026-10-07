//! Discovery leases and the candidate table.
use std::collections::HashMap;

use app_facade_api::pod::{
    AdapterState, AdapterStatus, BleAdvertisement, DeviceKind, DiscoverySnapshot, PodCandidate,
    CANDIDATE_TTL_MS,
};

use crate::platform::Advertisement;

/// Smaller RSSI changes do not count as a change of the snapshot.
const RSSI_PUBLISH_DELTA: i32 = 6;
const LOCAL_NAME_PREFIX: &str = "MeowPod ";
/// Wait before retrying a scan the radio refused, unless the adapter changes.
pub(crate) const SCAN_RETRY_MS: i64 = 5_000;

pub(crate) struct Discovery {
    pub revision: u64,
    pub scanning: bool,
    pub adapter: AdapterStatus,
    pub leases: HashMap<String, i64>,
    pub candidates: HashMap<String, PodCandidate>,
    pub scan_refused_at: Option<i64>,
}

impl Discovery {
    pub fn new() -> Self {
        Self {
            revision: 0,
            scanning: false,
            adapter: AdapterStatus {
                state: AdapterState::Unavailable,
                message: None,
            },
            leases: HashMap::new(),
            candidates: HashMap::new(),
            scan_refused_at: None,
        }
    }

    pub fn snapshot(&self) -> DiscoverySnapshot {
        let mut candidates: Vec<PodCandidate> = self.candidates.values().cloned().collect();
        candidates.sort_by(|a, b| {
            b.rssi
                .cmp(&a.rssi)
                .then(a.short_id.cmp(&b.short_id))
                .then(a.candidate_id.cmp(&b.candidate_id))
        });
        DiscoverySnapshot {
            revision: self.revision,
            scanning: self.scanning,
            adapter: self.adapter.clone(),
            candidates,
        }
    }

    pub fn has_live_lease(&mut self, now: i64) -> bool {
        self.leases.retain(|_, expires| *expires > now);
        !self.leases.is_empty()
    }

    pub fn set_adapter(&mut self, status: AdapterStatus) -> bool {
        if self.adapter == status {
            return false;
        }
        self.adapter = status;
        self.scan_refused_at = None;
        if self.adapter.state != AdapterState::Ready {
            self.candidates.clear();
            self.scanning = false;
        }
        self.revision += 1;
        true
    }

    pub fn set_scanning(&mut self, scanning: bool) -> bool {
        if self.scanning == scanning {
            return false;
        }
        self.scanning = scanning;
        self.revision += 1;
        true
    }

    pub fn prune(&mut self, now: i64) -> bool {
        let cutoff = now - CANDIDATE_TTL_MS;
        let before = self.candidates.len();
        self.candidates
            .retain(|_, candidate| candidate.last_seen_ms >= cutoff);
        if self.candidates.len() == before {
            return false;
        }
        self.revision += 1;
        true
    }

    /// Records an advertisement; `true` when the snapshot changed.
    pub fn observe(&mut self, advertisement: &Advertisement, now: i64) -> bool {
        let Some(decoded) = advertisement
            .manufacturer_data
            .as_deref()
            .and_then(BleAdvertisement::decode)
        else {
            return false;
        };
        let Some(kind) = DeviceKind::from_advertised(decoded.kind) else {
            return false;
        };
        let previous = self.candidates.get(&advertisement.peripheral_id);
        let name = advertisement
            .local_name
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_string)
            .or_else(|| {
                previous
                    .filter(|previous| previous.kind == kind)
                    .map(|previous| previous.name.clone())
            })
            .unwrap_or_else(|| match kind {
                DeviceKind::Pod => "MeowPod".into(),
                DeviceKind::Host => "MeowLink Host".into(),
            });
        let model = match kind {
            DeviceKind::Pod => name
                .strip_prefix(LOCAL_NAME_PREFIX)
                .map(|model| model.trim().to_string())
                .filter(|model| !model.is_empty()),
            DeviceKind::Host => None,
        };
        let candidate = PodCandidate {
            candidate_id: advertisement.peripheral_id.clone(),
            kind,
            name,
            model,
            short_id: decoded.short_id_hex(),
            rssi: advertisement
                .rssi
                .or(previous.map(|previous| previous.rssi))
                .unwrap_or(i16::MIN),
            commissionable: decoded.commissionable(),
            wifi_configured: decoded.wifi_configured(),
            last_seen_ms: now,
        };
        let changed = match previous {
            None => true,
            Some(previous) => {
                previous.name != candidate.name
                    || previous.kind != candidate.kind
                    || previous.short_id != candidate.short_id
                    || previous.commissionable != candidate.commissionable
                    || previous.wifi_configured != candidate.wifi_configured
                    || (i32::from(previous.rssi) - i32::from(candidate.rssi)).abs()
                        >= RSSI_PUBLISH_DELTA
            }
        };
        if changed {
            self.candidates
                .insert(candidate.candidate_id.clone(), candidate);
            self.revision += 1;
        } else if let Some(entry) = self.candidates.get_mut(&advertisement.peripheral_id) {
            entry.last_seen_ms = now;
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use app_facade_api::pod::{BLE_FLAG_COMMISSIONABLE, BLE_KIND_HOST, BLE_KIND_POD};

    fn advert(name: Option<&str>, kind: u8, flags: u8, rssi: i16) -> Advertisement {
        Advertisement {
            peripheral_id: "peripheral-1".into(),
            local_name: name.map(str::to_string),
            manufacturer_data: Some(
                BleAdvertisement {
                    kind,
                    flags,
                    short_id: [1, 2, 3, 4],
                }
                .encode()
                .to_vec(),
            ),
            service_uuids: Vec::new(),
            rssi: Some(rssi),
        }
    }

    #[test]
    fn merges_names_and_ignores_small_rssi_changes() {
        let mut discovery = Discovery::new();
        assert!(discovery.observe(
            &advert(
                Some("MeowPod S1"),
                BLE_KIND_POD,
                BLE_FLAG_COMMISSIONABLE,
                -60
            ),
            1_000
        ));
        assert!(!discovery.observe(
            &advert(None, BLE_KIND_POD, BLE_FLAG_COMMISSIONABLE, -63),
            2_000
        ));
        let candidate = &discovery.snapshot().candidates[0];
        assert_eq!(candidate.name, "MeowPod S1");
        assert_eq!(candidate.model.as_deref(), Some("S1"));
        assert_eq!(candidate.short_id, "01020304");
        assert_eq!(candidate.last_seen_ms, 2_000);
        assert!(discovery.observe(
            &advert(None, BLE_KIND_POD, BLE_FLAG_COMMISSIONABLE, -70),
            3_000
        ));
        assert!(discovery.observe(&advert(None, BLE_KIND_POD, 0, -70), 3_500));
        assert!(!discovery.snapshot().candidates[0].commissionable);

        assert!(discovery.observe(
            &advert(
                Some("Kitchen Host"),
                BLE_KIND_HOST,
                BLE_FLAG_COMMISSIONABLE,
                -70
            ),
            4_000
        ));
        let host = &discovery.snapshot().candidates[0];
        assert_eq!((host.kind, host.model.as_deref()), (DeviceKind::Host, None));

        let mut unknown = advert(None, 9, 0, -50);
        unknown.peripheral_id = "other".into();
        assert!(!discovery.observe(&unknown, 4_000));
        unknown.manufacturer_data = None;
        assert!(!discovery.observe(&unknown, 4_000));
    }

    #[test]
    fn prunes_stale_candidates_and_clears_them_when_the_adapter_stops() {
        let mut discovery = Discovery::new();
        discovery.observe(&advert(None, BLE_KIND_POD, 1, -60), 1_000);
        assert!(!discovery.prune(1_000 + CANDIDATE_TTL_MS));
        assert!(discovery.prune(1_001 + CANDIDATE_TTL_MS));
        discovery.observe(&advert(None, BLE_KIND_POD, 1, -60), 20_000);
        discovery.set_scanning(true);
        assert!(discovery.set_adapter(AdapterStatus {
            state: AdapterState::PoweredOff,
            message: None,
        }));
        let snapshot = discovery.snapshot();
        assert!(snapshot.candidates.is_empty() && !snapshot.scanning);
    }
}
