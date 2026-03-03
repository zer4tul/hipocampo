//! Hybrid search utilities

use crate::memory::MemoryEntry;
use std::collections::HashMap;

/// Merge vector and keyword results with weighted scores
pub fn merge_hybrid_results(
    vector_results: Vec<MemoryEntry>,
    keyword_results: Vec<MemoryEntry>,
    vector_weight: f32,
    keyword_weight: f32,
) -> Vec<MemoryEntry> {
    let mut by_id: HashMap<String, MemoryEntry> = HashMap::new();

    for r in vector_results {
        let score = r.score.unwrap_or(0.0) * vector_weight as f64;
        by_id.insert(
            r.id.clone(),
            MemoryEntry {
                id: r.id,
                key: r.key,
                content: r.content,
                category: r.category,
                timestamp: r.timestamp,
                session_id: r.session_id,
                score: Some(score),
                embedding: None,
            },
        );
    }

    for r in keyword_results {
        let entry = by_id.entry(r.id.clone()).or_insert(MemoryEntry {
            id: r.id,
            key: r.key,
            content: r.content,
            category: r.category,
            timestamp: r.timestamp,
            session_id: r.session_id,
            score: Some(0.0),
            embedding: None,
        });
        entry.score = Some(
            entry.score.unwrap_or(0.0) + r.score.unwrap_or(0.0) * keyword_weight as f64,
        );
    }

    let mut merged: Vec<_> = by_id.into_values().collect();
    merged.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    merged
}

/// Reciprocal Rank Fusion (RRF) for reranking
pub fn reciprocal_rank_fusion(
    rankings: &[Vec<MemoryEntry>],
    k: usize,
) -> Vec<MemoryEntry> {
    let mut scores: HashMap<String, f64> = HashMap::new();
    let mut entries: HashMap<String, MemoryEntry> = HashMap::new();

    for ranking in rankings {
        for (rank, entry) in ranking.iter().enumerate() {
            let rrf_score = 1.0 / (k as f64 + rank as f64 + 1.0);
            *scores.entry(entry.id.clone()).or_insert(0.0) += rrf_score;
            entries.entry(entry.id.clone()).or_insert_with(|| entry.clone());
        }
    }

    let mut results: Vec<_> = entries.into_values().collect();
    for entry in &mut results {
        entry.score = scores.get(&entry.id).copied();
    }

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::MemoryCategory;

    #[test]
    fn merge_combines_scores_correctly() {
        let vector_results = vec![MemoryEntry {
            id: "1".into(),
            key: "test".into(),
            content: "content".into(),
            category: MemoryCategory::Core,
            timestamp: "2026-01-01T00:00:00Z".into(),
            session_id: None,
            score: Some(0.9),
            embedding: None,
        }];

        let keyword_results = vec![MemoryEntry {
            id: "1".into(),
            key: "test".into(),
            content: "content".into(),
            category: MemoryCategory::Core,
            timestamp: "2026-01-01T00:00:00Z".into(),
            session_id: None,
            score: Some(0.8),
            embedding: None,
        }];

        let merged = merge_hybrid_results(vector_results, keyword_results, 0.7, 0.3);

        assert_eq!(merged.len(), 1);
        let expected = 0.9 * 0.7 + 0.8 * 0.3;
        assert!((merged[0].score.unwrap() - expected).abs() < 0.001);
    }
}
