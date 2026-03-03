//! MMR (Maximal Marginal Relevance) re-ranking

use crate::memory::MemoryEntry;

/// MMR configuration
pub struct MMRConfig {
    pub enabled: bool,
    pub lambda: f32, // 0.0 = max diversity, 1.0 = max relevance
}

impl Default for MMRConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            lambda: 0.7,
        }
    }
}

/// Apply MMR to re-rank results for diversity
pub fn apply_mmr(results: Vec<MemoryEntry>, config: &MMRConfig, top_k: usize) -> Vec<MemoryEntry> {
    if !config.enabled || results.len() <= top_k {
        return results;
    }

    let mut selected: Vec<MemoryEntry> = Vec::new();
    let mut remaining = results;

    while selected.len() < top_k && !remaining.is_empty() {
        let mut best_score = f64::NEG_INFINITY;
        let mut best_idx = 0;

        for (i, candidate) in remaining.iter().enumerate() {
            let relevance = candidate.score.unwrap_or(0.0);

            let diversity = if selected.is_empty() {
                0.0_f64
            } else {
                selected
                    .iter()
                    .map(|s| jaccard_similarity(&s.content, &candidate.content))
                    .fold(0.0_f64, |a: f64, b: f64| a.max(b))
            };

            let score = config.lambda as f64 * relevance - (1.0 - config.lambda) as f64 * diversity;

            if score > best_score {
                best_score = score;
                best_idx = i;
            }
        }

        selected.push(remaining.remove(best_idx));
    }

    selected
}

/// Jaccard similarity between two strings
fn jaccard_similarity(a: &str, b: &str) -> f64 {
    let a_tokens: std::collections::HashSet<&str> = a.split_whitespace().collect();
    let b_tokens: std::collections::HashSet<&str> = b.split_whitespace().collect();

    if a_tokens.is_empty() || b_tokens.is_empty() {
        return 0.0;
    }

    let intersection = a_tokens.intersection(&b_tokens).count();
    let union = a_tokens.union(&b_tokens).count();

    intersection as f64 / union as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jaccard_similarity_works() {
        let sim = jaccard_similarity("hello world", "hello rust");
        assert!((sim - 0.333).abs() < 0.01);
    }
}
