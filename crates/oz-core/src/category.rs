//! Category domain type — product grouping for the POS.
//!
//! A [`Category`] is a lightweight grouping: an id, a display name,
//! a colour for the POS UI, and an icon identifier for the pill rendering.
//! Categories are stored in the `categories` table (migrations `002_products.sql`
//! and `039_category_icon.sql`).

use serde::{Deserialize, Serialize};

/// A product category with display colour and icon.
///
/// # Schema mapping
///
/// Maps 1:1 to the `categories` table. The `colour` field is a hex
/// string like `"#6366f1"`. The `icon` field is a short identifier
/// like `"dots-1"` / `"dots-2"` / `"dots-3"` that the front-end maps
/// to an inline SVG; an empty string means "no icon".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Category {
    /// Internal row id.
    pub id: String,

    /// Display name (unique across all categories).
    pub name: String,

    /// Hex colour string, e.g. `"#06b6d4"`.
    pub colour: String,

    /// Icon identifier, e.g. `"dots-1"`. Empty string = no icon.
    pub icon: String,
}

impl Category {
    /// Create a new category with the given id, name, colour, and icon.
    ///
    /// # Panics
    ///
    /// Panics if `name` is empty after trimming.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        colour: impl Into<String>,
        icon: impl Into<String>,
    ) -> Self {
        let name = name.into().trim().to_owned();
        assert!(!name.is_empty(), "category name must not be empty");
        Self {
            id: id.into(),
            name,
            colour: colour.into(),
            icon: icon.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Construction ─────────────────────────────────────────────

    #[test]
    fn new_category() {
        let c = Category::new("cat-1", "Drinks", "#06b6d4", "dots-1");
        assert_eq!(c.id, "cat-1");
        assert_eq!(c.name, "Drinks");
        assert_eq!(c.colour, "#06b6d4");
        assert_eq!(c.icon, "dots-1");
    }

    #[test]
    #[should_panic(expected = "category name must not be empty")]
    fn new_panics_on_empty_name() {
        Category::new("cat-1", "   ", "#000", "");
    }

    #[test]
    fn new_category_empty_icon() {
        let c = Category::new("cat-2", "Food", "#ef4444", "");
        assert_eq!(c.icon, "");
    }

    #[test]
    fn new_category_long_name() {
        let long_name = "Freshly Brewed Artisanal Coffee".repeat(5);
        let c = Category::new("cat-long", &long_name, "#8b5cf6", "");
        assert_eq!(c.name, long_name);
    }

    // ── Colour formats ───────────────────────────────────────────

    #[test]
    fn category_colour_variations() {
        let colours = [
            "#06b6d4", "#ef4444", "#10b981", "#f59e0b", "#6366f1", "#ec4899", "#000000", "#ffffff",
        ];
        for colour in colours {
            let c = Category::new("cat", "Test", colour, "");
            assert_eq!(c.colour, colour, "should accept colour {colour}");
        }
    }

    #[test]
    fn category_colour_uppercase_hex() {
        let c = Category::new("cat-1", "Test", "#FF6600", "");
        assert_eq!(c.colour, "#FF6600");
    }

    #[test]
    fn category_colour_shorthand() {
        let c = Category::new("cat-1", "Test", "#fff", "");
        assert_eq!(c.colour, "#fff");
    }

    #[test]
    fn category_colour_no_hash() {
        let c = Category::new("cat-1", "Test", "ff6600", "");
        assert_eq!(c.colour, "ff6600");
    }

    #[test]
    fn category_colour_empty() {
        let c = Category::new("cat-1", "Test", "", "");
        assert_eq!(c.colour, "");
    }

    // ── Icon values ──────────────────────────────────────────────

    #[test]
    fn category_icon_different_values() {
        let icons = ["dots-1", "dots-2", "dots-3", "square", "circle", "star"];
        for icon in icons {
            let c = Category::new("cat", "Test", "#000", icon);
            assert_eq!(c.icon, icon, "should accept icon {icon}");
        }
    }

    #[test]
    fn category_icon_long_value() {
        let icon = "custom-category-icon-name";
        let c = Category::new("cat-1", "Test", "#000", icon);
        assert_eq!(c.icon, icon);
    }

    // ── ID values ────────────────────────────────────────────────

    #[test]
    fn category_id_variations() {
        let c = Category::new("", "Empty ID", "#000", "");
        assert_eq!(c.id, "");

        let c = Category::new("cat-with-dashes", "Dasher", "#06b6d4", "");
        assert_eq!(c.id, "cat-with-dashes");
    }

    // ── Serde ────────────────────────────────────────────────────

    #[test]
    fn serde_roundtrip() {
        let c = Category::new("cat-1", "Drinks", "#06b6d4", "dots-2");
        let json = serde_json::to_string(&c).unwrap();
        let back: Category = serde_json::from_str(&json).unwrap();
        assert_eq!(back, c);
    }

    #[test]
    fn serde_roundtrip_all_variations() {
        let c = Category::new("cat-99", "Specials", "#f97316", "star");
        let json = serde_json::to_string(&c).unwrap();
        let back: Category = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "cat-99");
        assert_eq!(back.name, "Specials");
        assert_eq!(back.colour, "#f97316");
        assert_eq!(back.icon, "star");
    }

    #[test]
    fn serde_json_field_names() {
        let c = Category::new("cat-1", "Drinks", "#06b6d4", "dots-1");
        let json = serde_json::to_value(&c).unwrap();
        assert_eq!(json["id"], "cat-1");
        assert_eq!(json["name"], "Drinks");
        assert_eq!(json["colour"], "#06b6d4");
        assert_eq!(json["icon"], "dots-1");
    }

    // ── Clone + equality ─────────────────────────────────────────

    #[test]
    fn category_clone_eq() {
        let a = Category::new("cat-1", "Drinks", "#06b6d4", "dots-1");
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn category_neq_when_field_differs() {
        let a = Category::new("cat-1", "Drinks", "#06b6d4", "dots-1");
        let b = Category::new("cat-1", "Food", "#ef4444", "dots-2");
        assert_ne!(a, b);
    }

    #[test]
    fn category_debug_output() {
        let c = Category::new("cat-1", "Drinks", "#06b6d4", "dots-1");
        let debug = format!("{:?}", c);
        assert!(debug.contains("Drinks"));
        assert!(debug.contains("#06b6d4"));
    }
}
