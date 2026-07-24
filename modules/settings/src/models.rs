//! Settings domain models.

use serde::{Deserialize, Serialize};

/// A key-value setting record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingItem {
    /// Settings key name.
    pub key: String,
    /// Settings value.
    pub value: String,
    /// ISO-8601 update timestamp.
    pub updated_at: String,
}

impl SettingItem {
    /// Create a new setting item.
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            updated_at: String::new(),
        }
    }
}
