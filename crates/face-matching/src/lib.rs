//! Pure matching shared by the gallery authority and its consumers.
use person_api::{GalleryEntry, MatchPolicy};
#[derive(Debug, Clone, PartialEq)]
pub enum Match {
    Known {
        person_id: String,
        cluster_id: String,
        score: f32,
    },
    UnknownCluster {
        cluster_id: String,
        score: f32,
    },
    Unmatched,
    Ambiguous,
}
pub fn validate_vector(vector: &[f32], dimensions: usize) -> bool {
    vector.len() == dimensions
        && dimensions > 0
        && vector.iter().all(|v| v.is_finite())
        && (vector.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() - 1.0).abs() < 0.01
}
pub fn cosine(a: &[f32], b: &[f32]) -> Option<f32> {
    if a.is_empty() || a.len() != b.len() || !a.iter().chain(b).all(|v| v.is_finite()) {
        return None;
    }
    let dot: f64 = a.iter().zip(b).map(|(x, y)| *x as f64 * *y as f64).sum();
    let norm = (a.iter().map(|x| (*x as f64).powi(2)).sum::<f64>()
        * b.iter().map(|x| (*x as f64).powi(2)).sum::<f64>())
    .sqrt();
    (norm > 1e-12).then(|| (dot / norm).clamp(-1.0, 1.0) as f32)
}
pub fn score(vector: &[f32], entry: &GalleryEntry) -> Option<f32> {
    let centroid = cosine(vector, &entry.centroid)?;
    let exemplar = entry
        .exemplars
        .iter()
        .filter_map(|e| cosine(vector, e))
        .max_by(f32::total_cmp);
    Some(exemplar.map_or(centroid, |e| centroid * 0.6 + e * 0.4))
}
pub fn identify(vector: &[f32], entries: &[GalleryEntry], policy: &MatchPolicy) -> Match {
    if policy.id != person_api::MATCH_POLICY {
        return Match::Unmatched;
    }
    let mut ranked: Vec<_> = entries
        .iter()
        .filter_map(|entry| score(vector, entry).map(|s| (entry, s)))
        .collect();
    ranked.sort_by(|(a, x), (b, y)| y.total_cmp(x).then_with(|| a.cluster_id.cmp(&b.cluster_id)));
    let Some((best, value)) = ranked.iter().find(|(entry, value)| {
        *value
            >= if entry.person_id.is_some() {
                policy.known_threshold
            } else {
                policy.unknown_threshold
            }
    }) else {
        return Match::Unmatched;
    };
    // Multiple pose clusters of the same named person are one identity, not competing people.
    if ranked.iter().any(|(other, s)| {
        let same_person = best.person_id.is_some() && best.person_id == other.person_id;
        other.cluster_id != best.cluster_id && !same_person && *value - *s < policy.ambiguity_margin
    }) {
        return Match::Ambiguous;
    }
    match &best.person_id {
        Some(id) => Match::Known {
            person_id: id.clone(),
            cluster_id: best.cluster_id.clone(),
            score: *value,
        },
        None => Match::UnknownCluster {
            cluster_id: best.cluster_id.clone(),
            score: *value,
        },
    }
}
pub fn centroid(vectors: &[Vec<f32>]) -> Option<Vec<f32>> {
    let dimensions = vectors.first()?.len();
    if vectors.iter().any(|v| !validate_vector(v, dimensions)) {
        return None;
    }
    let mut sum = vec![0.0f64; dimensions];
    for v in vectors {
        for (s, x) in sum.iter_mut().zip(v) {
            *s += *x as f64;
        }
    }
    let norm = sum.iter().map(|v| v * v).sum::<f64>().sqrt();
    (norm > 1e-12).then(|| sum.into_iter().map(|v| (v / norm) as f32).collect())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn entry(cluster: &str, person: Option<&str>) -> GalleryEntry {
        GalleryEntry {
            cluster_id: cluster.into(),
            person_id: person.map(str::to_string),
            centroid: vec![1., 0.],
            exemplars: vec![],
        }
    }
    #[test]
    fn multiple_clusters_for_one_person_do_not_create_ambiguity() {
        assert!(matches!(
            identify(
                &[1., 0.],
                &[entry("a", Some("p")), entry("b", Some("p"))],
                &MatchPolicy::default()
            ),
            Match::Known { .. }
        ));
        assert_eq!(
            identify(
                &[1., 0.],
                &[entry("a", Some("p")), entry("b", Some("q"))],
                &MatchPolicy::default()
            ),
            Match::Ambiguous
        );
    }
    #[test]
    fn invalid_vectors_are_rejected() {
        assert!(!validate_vector(&[f32::NAN, 0.], 2));
        assert!(!validate_vector(&[0., 0.], 2));
        assert_eq!(cosine(&[1.], &[1., 0.]), None);
    }
}

#[cfg(test)]
mod threshold_tests {
    use super::*;
    #[test]
    fn different_acceptance_thresholds_do_not_hide_an_ambiguous_candidate() {
        let entry = |id: &str, person: Option<&str>, cos: f32| GalleryEntry {
            cluster_id: id.into(),
            person_id: person.map(str::to_string),
            centroid: vec![cos, (1. - cos * cos).sqrt()],
            exemplars: vec![],
        };
        assert_eq!(
            identify(
                &[1., 0.],
                &[
                    entry("known", Some("p"), 0.61),
                    entry("unknown", None, 0.60)
                ],
                &MatchPolicy::default()
            ),
            Match::Ambiguous
        );
    }
}
