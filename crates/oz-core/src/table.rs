use serde::{Deserialize, Serialize};

/// Status of a restaurant table in the floor plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TableStatus {
    /// Table is free to seat guests.
    Available,
    /// Table has an active order.
    Occupied,
    /// Table is booked for a future guest.
    Reserved,
    /// Table is being cleaned after release.
    Cleaning,
}

impl TableStatus {
    /// Return the serialised string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Occupied => "occupied",
            Self::Reserved => "reserved",
            Self::Cleaning => "cleaning",
        }
    }

    /// Parse a status from its string representation.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "available" => Some(Self::Available),
            "occupied" => Some(Self::Occupied),
            "reserved" => Some(Self::Reserved),
            "cleaning" => Some(Self::Cleaning),
            _ => None,
        }
    }
}

/// A restaurant table with position on the floor plan and current status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    /// Unique identifier (UUID v4).
    pub id: String,
    /// Human-readable name (e.g. "Table 1", "Patio A").
    pub name: String,
    /// Maximum number of guests.
    pub capacity: i64,
    /// X position on the floor plan (0-100 percentage).
    pub pos_x: f64,
    /// Y position on the floor plan (0-100 percentage).
    pub pos_y: f64,
    /// Shape: 'circle' or 'rectangle'.
    pub shape: String,
    /// Width in percentage (for rectangle tables).
    pub width: f64,
    /// Height in percentage (for rectangle tables).
    pub height: f64,
    /// Current status: 'available', 'occupied', 'reserved', 'cleaning'.
    pub status: String,
    /// Active sale ID when the table is occupied.
    pub active_sale_id: Option<String>,
    /// Section/area (e.g. "Indoor", "Patio", "Bar").
    pub section: String,
    /// Whether the table is active (soft-delete flag).
    pub active: bool,
    /// Ordinal position for sorted display.
    pub sort_order: i64,
    /// RFC-3339 creation timestamp.
    pub created_at: String,
    /// RFC-3339 last-update timestamp.
    pub updated_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── TableStatus as_str ─────────────────────────────────────────

    #[test]
    fn status_as_str_all_variants() {
        assert_eq!(TableStatus::Available.as_str(), "available");
        assert_eq!(TableStatus::Occupied.as_str(), "occupied");
        assert_eq!(TableStatus::Reserved.as_str(), "reserved");
        assert_eq!(TableStatus::Cleaning.as_str(), "cleaning");
    }

    // ── TableStatus from_str ───────────────────────────────────────

    #[test]
    fn status_from_str_all_variants() {
        assert_eq!(
            TableStatus::from_str("available"),
            Some(TableStatus::Available)
        );
        assert_eq!(
            TableStatus::from_str("occupied"),
            Some(TableStatus::Occupied)
        );
        assert_eq!(
            TableStatus::from_str("reserved"),
            Some(TableStatus::Reserved)
        );
        assert_eq!(
            TableStatus::from_str("cleaning"),
            Some(TableStatus::Cleaning)
        );
    }

    #[test]
    fn status_from_str_invalid() {
        assert_eq!(TableStatus::from_str("bogus"), None);
        assert_eq!(TableStatus::from_str(""), None);
        assert_eq!(TableStatus::from_str("AVAILABLE"), None);
    }

    #[test]
    fn status_from_str_roundtrip() {
        for s in &[
            TableStatus::Available,
            TableStatus::Occupied,
            TableStatus::Reserved,
            TableStatus::Cleaning,
        ] {
            assert_eq!(TableStatus::from_str(s.as_str()), Some(*s));
        }
    }

    // ── Serde roundtrips ───────────────────────────────────────────

    #[test]
    fn table_status_serde_roundtrip() {
        let status = TableStatus::Occupied;
        let json = serde_json::to_string(&status).unwrap();
        let back: TableStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(back, TableStatus::Occupied);
    }

    #[test]
    fn table_serde_roundtrip() {
        let table = Table {
            id: "t-1".into(),
            name: "Table 1".into(),
            capacity: 4,
            pos_x: 25.0,
            pos_y: 50.0,
            shape: "circle".into(),
            width: 10.0,
            height: 10.0,
            status: "available".into(),
            active_sale_id: None,
            section: "Main".into(),
            active: true,
            sort_order: 0,
            created_at: "2025-01-01T00:00:00.000Z".into(),
            updated_at: "2025-01-01T00:00:00.000Z".into(),
        };
        let json = serde_json::to_string(&table).unwrap();
        let back: Table = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, "t-1");
        assert_eq!(back.name, "Table 1");
        assert_eq!(back.capacity, 4);
        assert_eq!(back.pos_x, 25.0);
        assert_eq!(back.status, "available");
        assert_eq!(back.section, "Main");
        assert!(back.active);
    }

    #[test]
    fn table_with_active_sale() {
        let table = Table {
            id: "t-2".into(),
            name: "Patio A".into(),
            capacity: 6,
            pos_x: 80.0,
            pos_y: 20.0,
            shape: "rectangle".into(),
            width: 15.0,
            height: 8.0,
            status: "occupied".into(),
            active_sale_id: Some("sale-42".into()),
            section: "Patio".into(),
            active: true,
            sort_order: 1,
            created_at: String::new(),
            updated_at: String::new(),
        };
        assert_eq!(table.active_sale_id.as_deref(), Some("sale-42"));
        assert_eq!(table.status, "occupied");
        assert_eq!(table.shape, "rectangle");
    }

    #[test]
    fn table_inactive_soft_delete() {
        let table = Table {
            id: "t-3".into(),
            name: "Inactive".into(),
            capacity: 2,
            pos_x: 0.0,
            pos_y: 0.0,
            shape: "circle".into(),
            width: 0.0,
            height: 0.0,
            status: "cleaning".into(),
            active_sale_id: None,
            section: String::new(),
            active: false,
            sort_order: 99,
            created_at: String::new(),
            updated_at: String::new(),
        };
        assert!(!table.active);
        assert_eq!(table.status, "cleaning");
    }
}
