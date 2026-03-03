//! Temporal decay for memory relevance

use crate::memory::MemoryEntry;

/// Temporal decay configuration
pub struct TemporalDecayConfig {
    pub enabled: bool,
    pub half_life_days: f64,
}

impl Default for TemporalDecayConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            half_life_days: 30.0,
        }
    }
}

/// Apply temporal decay to memory scores
pub fn apply_temporal_decay(results: Vec<MemoryEntry>, config: &TemporalDecayConfig) -> Vec<MemoryEntry> {
    if !config.enabled {
        return results;
    }

    let lambda = std::f64::consts::LN_2 / config.half_life_days;

    results
        .into_iter()
        .map(|mut entry| {
            let age_days = estimate_age_days(&entry.timestamp);
            let decay = (-lambda * age_days).exp();

            entry.score = entry.score.map(|s| s * decay);
            entry
        })
        .collect()
}

/// Estimate age in days from timestamp
fn estimate_age_days(timestamp: &str) -> f64 {
    let parsed = chrono::DateTime::parse_from_rfc3339(timestamp);
    match parsed {
        Ok(dt) => {
            let now = chrono::Utc::now();
            (now - dt.with_timezone(&chrono::Utc)).num_days() as f64
        }
        Err(_) => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decay_reduces_old_scores() {
        let config = TemporalDecayConfig {
            enabled: true,
            half_life_days: 30.0,
        };

        let results = vec![MemoryEntry {
            id: "1".into(),
            key: "test".into(),
            content: "test".into(),
            category: crate::memory::MemoryCategory::Core,
            timestamp: "2026-01-01T00:00:00Z".into(),
            session_id: None,
            score: Some(1.0),
            embedding: None,
        }];

        let decayed = apply_temporal_decay(results, &config);

        assert!(decayed[0].score.unwrap() < 1.0);
    }
}
