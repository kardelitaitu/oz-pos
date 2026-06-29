//! Exchange rate types.

use serde::{Deserialize, Serialize};

/// A row from the `exchange_rates` table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExchangeRateRow {
    /// Internal row id (UUID v4).
    pub id: String,
    /// ISO-4217 currency code to convert from.
    pub from_currency: String,
    /// ISO-4217 currency code to convert to.
    pub to_currency: String,
    /// Conversion rate (multiply `from` amount by this to get `to` amount).
    pub rate: f64,
    /// Source of the rate (e.g. "manual", "ECB", "OANDA").
    pub source: String,
    /// ISO-8601 date this rate is effective from.
    pub effective_date: String,
    /// ISO-8601 row creation timestamp.
    pub created_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exchange_rate_row_construction() {
        let row = ExchangeRateRow {
            id: "rate-1".into(),
            from_currency: "USD".into(),
            to_currency: "EUR".into(),
            rate: 0.92,
            source: "ECB".into(),
            effective_date: "2026-06-01".into(),
            created_at: "2026-06-01T12:00:00.000Z".into(),
        };
        assert_eq!(row.id, "rate-1");
        assert_eq!(row.from_currency, "USD");
        assert_eq!(row.to_currency, "EUR");
        assert!((row.rate - 0.92).abs() < f64::EPSILON);
        assert_eq!(row.source, "ECB");
        assert_eq!(row.effective_date, "2026-06-01");
    }

    #[test]
    fn exchange_rate_row_serde_roundtrip() {
        let row = ExchangeRateRow {
            id: "rate-2".into(),
            from_currency: "GBP".into(),
            to_currency: "JPY".into(),
            rate: 185.42,
            source: "manual".into(),
            effective_date: "2026-06-15".into(),
            created_at: "2026-06-15T08:00:00.000Z".into(),
        };
        let json = serde_json::to_string(&row).unwrap();
        let deserialized: ExchangeRateRow = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, row.id);
        assert_eq!(deserialized.from_currency, row.from_currency);
        assert_eq!(deserialized.to_currency, row.to_currency);
        assert!((deserialized.rate - row.rate).abs() < f64::EPSILON);
        assert_eq!(deserialized.source, row.source);
        assert_eq!(deserialized.effective_date, row.effective_date);
        assert_eq!(deserialized.created_at, row.created_at);
    }

    #[test]
    fn exchange_rate_row_json_field_names() {
        let row = ExchangeRateRow {
            id: "r1".into(),
            from_currency: "USD".into(),
            to_currency: "EUR".into(),
            rate: 0.85,
            source: "ECB".into(),
            effective_date: "2026-07-01".into(),
            created_at: "2026-07-01T00:00:00.000Z".into(),
        };
        let json = serde_json::to_value(&row).unwrap();
        assert_eq!(json["id"], "r1");
        assert_eq!(json["from_currency"], "USD");
        assert_eq!(json["to_currency"], "EUR");
        assert!((json["rate"].as_f64().unwrap() - 0.85).abs() < f64::EPSILON);
        assert_eq!(json["source"], "ECB");
        assert_eq!(json["effective_date"], "2026-07-01");
        assert_eq!(json["created_at"], "2026-07-01T00:00:00.000Z");
    }

    #[test]
    fn exchange_rate_row_partial_eq() {
        let a = ExchangeRateRow {
            id: "r1".into(),
            from_currency: "USD".into(),
            to_currency: "EUR".into(),
            rate: 0.92,
            source: "ECB".into(),
            effective_date: "2026-06-01".into(),
            created_at: "2026-06-01T12:00:00.000Z".into(),
        };
        let b = ExchangeRateRow {
            id: "r1".into(),
            from_currency: "USD".into(),
            to_currency: "EUR".into(),
            rate: 0.92,
            source: "ECB".into(),
            effective_date: "2026-06-01".into(),
            created_at: "2026-06-01T12:00:00.000Z".into(),
        };
        assert_eq!(a, b);
        let c = ExchangeRateRow {
            rate: 0.91,
            ..a.clone()
        };
        assert_ne!(a, c);
    }

    #[test]
    fn exchange_rate_row_zero_rate() {
        let row = ExchangeRateRow {
            id: "r-zero".into(),
            from_currency: "USD".into(),
            to_currency: "BTC".into(),
            rate: 0.0,
            source: "manual".into(),
            effective_date: "2026-01-01".into(),
            created_at: "2026-01-01T00:00:00.000Z".into(),
        };
        assert_eq!(row.rate, 0.0);
    }

    #[test]
    fn exchange_rate_row_large_rate() {
        let row = ExchangeRateRow {
            id: "r-big".into(),
            from_currency: "BTC".into(),
            to_currency: "USD".into(),
            rate: 102345.67,
            source: "market".into(),
            effective_date: "2026-06-20".into(),
            created_at: "2026-06-20T00:00:00.000Z".into(),
        };
        assert!((row.rate - 102345.67).abs() < f64::EPSILON);
    }
}

