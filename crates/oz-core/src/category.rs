//! Category domain type — product grouping for the POS.
//!
//! A [`Category`] is a lightweight grouping: an id, a display name,
//! and a colour for the POS UI. Categories are stored in the
//! `categories` table (migration `002_products.sql`).

use serde::{Deserialize, Serialize};

/// A product category with display colour.
///
/// # Schema mapping
///
/// Maps 1:1 to the `categories` table. The `colour` field is a hex
/// string like `"#6366f1"` — the default set by the migration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Category {
    /// Internal row id.
    pub id: String,

    /// Display name (unique across all categories).
    pub name: String,

    /// Hex colour string, e.g. `"#06b6d4"`.
    pub colour: String,
}

impl Category {
    /// Create a new category with the given id, name, and colour.
    ///
    /// # Panics
    ///
    /// Panics if `name` is empty after trimming.
    pub fn new(id: impl Into<String>, name: impl Into<String>, colour: impl Into<String>) -> Self {
        let name = name.into().trim().to_owned();
        assert!(!name.is_empty(), "category name must not be empty");
        Self {
            id: id.into(),
            name,
            colour: colour.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_category() {
        let c = Category::new("cat-1", "Drinks", "#06b6d4");
        assert_eq!(c.id, "cat-1");
        assert_eq!(c.name, "Drinks");
        assert_eq!(c.colour, "#06b6d4");
    }

    #[test]
    #[should_panic(expected = "category name must not be empty")]
    fn new_panics_on_empty_name() {
        Category::new("cat-1", "   ", "#000");
    }

    #[test]
    fn serde_roundtrip() {
        let c = Category::new("cat-1", "Drinks", "#06b6d4");
        let json = serde_json::to_string(&c).unwrap();
        let back: Category = serde_json::from_str(&json).unwrap();
        assert_eq!(back, c);
    }
}
