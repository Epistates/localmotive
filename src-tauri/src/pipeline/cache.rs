use std::collections::HashMap;

use crate::analyzer::CodeAnalysis;

/// Content-hash keyed cache for analysis results.
/// Avoids re-parsing files when re-running generation with different settings.
#[derive(Default)]
pub struct AnalysisCache {
    entries: HashMap<u64, CodeAnalysis>,
}

impl AnalysisCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get a cached analysis by file content hash.
    pub fn get(&self, content_hash: u64) -> Option<&CodeAnalysis> {
        self.entries.get(&content_hash)
    }

    /// Store an analysis result keyed by content hash.
    pub fn insert(&mut self, content_hash: u64, analysis: CodeAnalysis) {
        self.entries.insert(content_hash, analysis);
    }

    /// Number of cached entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear all cached entries.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}
