/// Configuration for flat mode
#[derive(Debug, Clone)]
pub struct FlatConfig {
    /// Maximum depth to expand (None = unlimited)
    pub depth: Option<usize>,
    /// Maximum array elements to display
    pub array_limit: usize,
}

impl FlatConfig {
    pub fn new(depth: Option<usize>, array_limit: usize) -> Self {
        Self { depth, array_limit }
    }
}

impl Default for FlatConfig {
    fn default() -> Self {
        Self {
            depth: None,
            array_limit: 3,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flat_config_default() {
        let config = FlatConfig::default();
        assert_eq!(config.depth, None);
        assert_eq!(config.array_limit, 3);
    }

    #[test]
    fn test_flat_config_with_depth() {
        let config = FlatConfig::new(Some(2), 5);
        assert_eq!(config.depth, Some(2));
        assert_eq!(config.array_limit, 5);
    }
}
