use serde_json::Value;
use std::collections::{HashMap, VecDeque};

/// A simple LRU cache for parsed JSON rows
#[derive(Debug)]
pub struct RowCache {
    /// Maximum number of entries
    capacity: usize,
    /// Cached values by row index
    entries: HashMap<usize, Value>,
    /// Access order (most recent at back)
    order: VecDeque<usize>,
}

impl RowCache {
    /// Create a new cache with the specified capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: HashMap::with_capacity(capacity),
            order: VecDeque::with_capacity(capacity),
        }
    }

    /// Create a cache with default capacity (1000 rows)
    pub fn default_capacity() -> Self {
        Self::new(1000)
    }

    /// Get a cached row, updating its access time
    pub fn get(&mut self, index: usize) -> Option<&Value> {
        if self.entries.contains_key(&index) {
            // Move to back (most recently used)
            self.order.retain(|&i| i != index);
            self.order.push_back(index);
            self.entries.get(&index)
        } else {
            None
        }
    }

    /// Insert a row into the cache
    pub fn insert(&mut self, index: usize, value: Value) {
        // If already present, update and move to back
        if self.entries.contains_key(&index) {
            self.entries.insert(index, value);
            self.order.retain(|&i| i != index);
            self.order.push_back(index);
            return;
        }

        // Evict if at capacity
        if self.entries.len() >= self.capacity {
            if let Some(oldest) = self.order.pop_front() {
                self.entries.remove(&oldest);
            }
        }

        self.entries.insert(index, value);
        self.order.push_back(index);
    }

    /// Check if a row is cached
    pub fn contains(&self, index: usize) -> bool {
        self.entries.contains_key(&index)
    }

    /// Get the number of cached entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear all cached entries
    pub fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
    }

    /// Get cache hit statistics (for debugging)
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_basic_insert_and_get() {
        let mut cache = RowCache::new(10);

        cache.insert(0, json!({"id": 0}));
        cache.insert(1, json!({"id": 1}));

        assert_eq!(cache.get(0).unwrap()["id"], 0);
        assert_eq!(cache.get(1).unwrap()["id"], 1);
        assert!(cache.get(2).is_none());
    }

    #[test]
    fn test_lru_eviction() {
        let mut cache = RowCache::new(3);

        // Fill cache
        cache.insert(0, json!({"id": 0}));
        cache.insert(1, json!({"id": 1}));
        cache.insert(2, json!({"id": 2}));

        assert_eq!(cache.len(), 3);

        // Access 0 to make it recently used
        cache.get(0);

        // Insert new item, should evict 1 (least recently used)
        cache.insert(3, json!({"id": 3}));

        assert_eq!(cache.len(), 3);
        assert!(cache.contains(0)); // Still there (accessed recently)
        assert!(!cache.contains(1)); // Evicted
        assert!(cache.contains(2));
        assert!(cache.contains(3));
    }

    #[test]
    fn test_update_existing() {
        let mut cache = RowCache::new(10);

        cache.insert(0, json!({"old": true}));
        cache.insert(0, json!({"new": true}));

        let value = cache.get(0).unwrap();
        assert!(value.get("new").is_some());
        assert!(value.get("old").is_none());
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_clear() {
        let mut cache = RowCache::new(10);

        cache.insert(0, json!({}));
        cache.insert(1, json!({}));
        cache.insert(2, json!({}));

        assert_eq!(cache.len(), 3);

        cache.clear();

        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_capacity_one() {
        let mut cache = RowCache::new(1);

        cache.insert(0, json!({"id": 0}));
        assert!(cache.contains(0));

        cache.insert(1, json!({"id": 1}));
        assert!(!cache.contains(0));
        assert!(cache.contains(1));
    }

    #[test]
    fn test_access_updates_order() {
        let mut cache = RowCache::new(3);

        cache.insert(0, json!({}));
        cache.insert(1, json!({}));
        cache.insert(2, json!({}));

        // Access in reverse order
        cache.get(2);
        cache.get(1);
        cache.get(0);

        // Now insert new items - should evict 2, then 1
        cache.insert(3, json!({}));
        assert!(!cache.contains(2)); // First to be evicted

        cache.insert(4, json!({}));
        assert!(!cache.contains(1)); // Second to be evicted

        assert!(cache.contains(0)); // Still there
        assert!(cache.contains(3));
        assert!(cache.contains(4));
    }

    #[test]
    fn test_default_capacity() {
        let cache = RowCache::default_capacity();
        assert_eq!(cache.capacity(), 1000);
    }
}
