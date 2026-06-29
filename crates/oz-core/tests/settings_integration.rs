//! Integration tests for the settings module — config persistence,
//! default values, updates, feature flags, batch operations, and
//! edge cases with data.
//!
//! Tests exercise the full persistence layer via [`oz_core::Settings`]
//! (which wraps `platform_core::settings::Settings`) and the Store API
//! against an in-memory SQLite database.

use oz_core::{Store, Settings, migrations, FeatureRegistry, Feature};
use rusqlite::Connection;

// ── Helpers ───────────────────────────────────────────────────────────

fn setup() -> Connection {
    let mut conn = Connection::open_in_memory().unwrap();
    conn.pragma_update(None, "foreign_keys", "ON").unwrap();
    migrations::run(&mut conn).unwrap();
    conn
}

fn store(conn: &Connection) -> Store<'_> {
    Store::new(conn)
}

// ── Typed store configuration ────────────────────────────────────────

#[test]
fn store_name_default_is_none() {
    let conn = setup();
    assert_eq!(Settings::get_store_name(&conn).unwrap(), None);
}

#[test]
fn set_and_get_store_name() {
    let conn = setup();
    Settings::set_store_name(&conn, "Acme POS").unwrap();
    assert_eq!(
        Settings::get_store_name(&conn).unwrap(),
        Some("Acme POS".into())
    );
}

#[test]
fn set_store_name_overwrites() {
    let conn = setup();
    Settings::set_store_name(&conn, "Old Name").unwrap();
    Settings::set_store_name(&conn, "New Name").unwrap();
    assert_eq!(Settings::get_store_name(&conn).unwrap(), Some("New Name".into()));
}

#[test]
fn store_address_default_is_none() {
    let conn = setup();
    assert_eq!(Settings::get_store_address(&conn).unwrap(), None);
}

#[test]
fn set_and_get_store_address() {
    let conn = setup();
    Settings::set_store_address(&conn, "123 Main St, City, State 12345").unwrap();
    assert_eq!(
        Settings::get_store_address(&conn).unwrap(),
        Some("123 Main St, City, State 12345".into())
    );
}

#[test]
fn store_tax_id_default_is_none() {
    let conn = setup();
    assert_eq!(Settings::get_store_tax_id(&conn).unwrap(), None);
}

#[test]
fn set_and_get_store_tax_id() {
    let conn = setup();
    Settings::set_store_tax_id(&conn, "12-3456789").unwrap();
    assert_eq!(
        Settings::get_store_tax_id(&conn).unwrap(),
        Some("12-3456789".into())
    );
}

#[test]
fn default_currency_default_is_none() {
    let conn = setup();
    assert_eq!(Settings::get_default_currency(&conn).unwrap(), None);
}

#[test]
fn set_and_get_default_currency() {
    let conn = setup();
    Settings::set_default_currency(&conn, "EUR").unwrap();
    assert_eq!(
        Settings::get_default_currency(&conn).unwrap(),
        Some("EUR".into())
    );
}

// ── All typed settings coexist ───────────────────────────────────────

#[test]
fn all_typed_settings_roundtrip() {
    let conn = setup();

    Settings::set_store_name(&conn, "My Store").unwrap();
    Settings::set_store_address(&conn, "456 Oak Ave").unwrap();
    Settings::set_store_tax_id(&conn, "TAX-999").unwrap();
    Settings::set_default_currency(&conn, "GBP").unwrap();

    assert_eq!(Settings::get_store_name(&conn).unwrap(), Some("My Store".into()));
    assert_eq!(
        Settings::get_store_address(&conn).unwrap(),
        Some("456 Oak Ave".into())
    );
    assert_eq!(Settings::get_store_tax_id(&conn).unwrap(), Some("TAX-999".into()));
    assert_eq!(Settings::get_default_currency(&conn).unwrap(), Some("GBP".into()));
}

// ── Receipt display settings ─────────────────────────────────────────

#[test]
fn receipt_show_currency_default_false() {
    let conn = setup();
    assert!(!Settings::get_receipt_show_currency(&conn).unwrap());
}

#[test]
fn receipt_show_currency_true_then_false() {
    let conn = setup();
    Settings::set_receipt_show_currency(&conn, true).unwrap();
    assert!(Settings::get_receipt_show_currency(&conn).unwrap());

    Settings::set_receipt_show_currency(&conn, false).unwrap();
    assert!(!Settings::get_receipt_show_currency(&conn).unwrap());
}

#[test]
fn receipt_decimal_separator_default_dot() {
    let conn = setup();
    assert_eq!(Settings::get_receipt_decimal_separator(&conn).unwrap(), "dot");
}

#[test]
fn receipt_decimal_separator_roundtrip() {
    let conn = setup();
    Settings::set_receipt_decimal_separator(&conn, "comma").unwrap();
    assert_eq!(Settings::get_receipt_decimal_separator(&conn).unwrap(), "comma");

    Settings::set_receipt_decimal_separator(&conn, "none").unwrap();
    assert_eq!(Settings::get_receipt_decimal_separator(&conn).unwrap(), "none");

    // Back to dot.
    Settings::set_receipt_decimal_separator(&conn, "dot").unwrap();
    assert_eq!(Settings::get_receipt_decimal_separator(&conn).unwrap(), "dot");
}

#[test]
fn receipt_show_tax_default_true() {
    let conn = setup();
    assert!(Settings::get_receipt_show_tax(&conn).unwrap());
}

#[test]
fn receipt_show_tax_true_then_false() {
    let conn = setup();
    assert!(Settings::get_receipt_show_tax(&conn).unwrap());

    Settings::set_receipt_show_tax(&conn, false).unwrap();
    assert!(!Settings::get_receipt_show_tax(&conn).unwrap());

    Settings::set_receipt_show_tax(&conn, true).unwrap();
    assert!(Settings::get_receipt_show_tax(&conn).unwrap());
}

#[test]
fn receipt_footer_default_empty() {
    let conn = setup();
    assert_eq!(Settings::get_receipt_footer(&conn).unwrap(), "");
}

#[test]
fn receipt_footer_roundtrip() {
    let conn = setup();
    Settings::set_receipt_footer(&conn, "Thank you for shopping!").unwrap();
    assert_eq!(
        Settings::get_receipt_footer(&conn).unwrap(),
        "Thank you for shopping!"
    );

    // Set to empty clears it.
    Settings::set_receipt_footer(&conn, "").unwrap();
    assert_eq!(Settings::get_receipt_footer(&conn).unwrap(), "");
}

#[test]
fn receipt_paper_width_default_standard() {
    let conn = setup();
    assert_eq!(Settings::get_receipt_paper_width(&conn).unwrap(), "standard");
}

#[test]
fn receipt_paper_width_narrow() {
    let conn = setup();
    Settings::set_receipt_paper_width(&conn, "narrow").unwrap();
    assert_eq!(Settings::get_receipt_paper_width(&conn).unwrap(), "narrow");
}

// ── Cloud sync settings ──────────────────────────────────────────────

#[test]
fn sync_server_url_default_none() {
    let conn = setup();
    assert_eq!(Settings::get_sync_server_url(&conn).unwrap(), None);
}

#[test]
fn sync_server_url_roundtrip() {
    let conn = setup();
    Settings::set_sync_server_url(&conn, "https://sync.example.com/api").unwrap();
    assert_eq!(
        Settings::get_sync_server_url(&conn).unwrap(),
        Some("https://sync.example.com/api".into())
    );
}

#[test]
fn sync_api_key_default_none() {
    let conn = setup();
    assert_eq!(Settings::get_sync_api_key(&conn).unwrap(), None);
}

#[test]
fn sync_api_key_roundtrip() {
    let conn = setup();
    Settings::set_sync_api_key(&conn, "sk_test_abc123_def456").unwrap();
    assert_eq!(
        Settings::get_sync_api_key(&conn).unwrap(),
        Some("sk_test_abc123_def456".into())
    );
}

#[test]
fn sync_enabled_default_false() {
    let conn = setup();
    assert!(!Settings::is_sync_enabled(&conn).unwrap());
}

#[test]
fn sync_enabled_toggle() {
    let conn = setup();
    Settings::set_sync_enabled(&conn, true).unwrap();
    assert!(Settings::is_sync_enabled(&conn).unwrap());

    Settings::set_sync_enabled(&conn, false).unwrap();
    assert!(!Settings::is_sync_enabled(&conn).unwrap());
}

#[test]
fn sync_settings_independent_of_store_settings() {
    let conn = setup();

    // Mix sync and store settings.
    Settings::set_store_name(&conn, "My Store").unwrap();
    Settings::set_sync_server_url(&conn, "https://sync.example.com").unwrap();
    Settings::set_sync_enabled(&conn, true).unwrap();
    Settings::set_default_currency(&conn, "USD").unwrap();
    Settings::set_sync_api_key(&conn, "sk-123").unwrap();

    assert_eq!(Settings::get_store_name(&conn).unwrap(), Some("My Store".into()));
    assert_eq!(
        Settings::get_sync_server_url(&conn).unwrap(),
        Some("https://sync.example.com".into())
    );
    assert!(Settings::is_sync_enabled(&conn).unwrap());
    assert_eq!(Settings::get_default_currency(&conn).unwrap(), Some("USD".into()));
    assert_eq!(Settings::get_sync_api_key(&conn).unwrap(), Some("sk-123".into()));
}

// ── Feature flags via Settings ───────────────────────────────────────

#[test]
fn load_features_empty_db_returns_empty() {
    let conn = setup();
    let reg = Settings::load_features(&conn).unwrap();
    assert_eq!(reg.count(), 0);
}

#[test]
fn save_and_load_features() {
    let conn = setup();
    let reg = FeatureRegistry::simple_retail();
    Settings::save_features(&conn, &reg).unwrap();
    let loaded = Settings::load_features(&conn).unwrap();
    assert_eq!(loaded, reg);
}

#[test]
fn save_features_replaces_previous() {
    let conn = setup();

    let simple = FeatureRegistry::simple_retail();
    Settings::save_features(&conn, &simple).unwrap();

    let full = FeatureRegistry::full_store();
    Settings::save_features(&conn, &full).unwrap();

    let loaded = Settings::load_features(&conn).unwrap();
    assert_eq!(loaded, full, "full store features should replace simple retail");
}

#[test]
fn save_features_with_full_store() {
    let conn = setup();
    let reg = FeatureRegistry::full_store();
    Settings::save_features(&conn, &reg).unwrap();
    let loaded = Settings::load_features(&conn).unwrap();

    // Full store has 24 features (from the preset).
    assert!(loaded.count() >= 20);
    assert!(loaded.is_enabled(Feature::SimpleRetail));
    assert!(loaded.is_enabled(Feature::CardPayment));
    assert!(loaded.is_enabled(Feature::StaffLogin));
    assert!(loaded.is_enabled(Feature::Analytics));
    assert!(loaded.is_enabled(Feature::Reporting));
}

#[test]
fn prune_stale_features_removes_old_flags() {
    let conn = setup();
    let reg = FeatureRegistry::simple_retail();
    Settings::save_features(&conn, &reg).unwrap();

    // Inject a stale feature flag.
    Settings::set(&conn, "feature.stale-flag", "1").unwrap();

    let removed = Settings::prune_stale_features(&conn, &reg).unwrap();
    assert_eq!(removed, 1, "should remove the stale flag");

    // The stale flag should be gone.
    let all = Settings::load_all(&conn).unwrap();
    assert!(
        !all.iter().any(|(k, _)| k == "feature.stale-flag"),
        "stale flag should have been removed"
    );

    // The current features should still be present.
    let loaded = Settings::load_features(&conn).unwrap();
    assert_eq!(loaded, reg);
}

#[test]
fn prune_stale_features_noop_when_no_stale() {
    let conn = setup();
    let reg = FeatureRegistry::full_store();
    Settings::save_features(&conn, &reg).unwrap();
    let removed = Settings::prune_stale_features(&conn, &reg).unwrap();
    assert_eq!(removed, 0, "no stale flags to remove");
}

#[test]
fn prune_stale_ignores_non_feature_settings() {
    let conn = setup();
    let reg = FeatureRegistry::simple_retail();
    Settings::save_features(&conn, &reg).unwrap();

    // Inject some non-feature settings and a stale feature flag.
    Settings::set_store_name(&conn, "Test Store").unwrap();
    Settings::set(&conn, "feature.old-flag", "1").unwrap();
    Settings::set_default_currency(&conn, "JPY").unwrap();

    let removed = Settings::prune_stale_features(&conn, &reg).unwrap();
    assert_eq!(removed, 1, "only the stale feature flag should be removed");

    // Non-feature settings should survive.
    assert_eq!(
        Settings::get_store_name(&conn).unwrap(),
        Some("Test Store".into())
    );
    assert_eq!(
        Settings::get_default_currency(&conn).unwrap(),
        Some("JPY".into())
    );
}

// ── Feature flags + store settings coexist ───────────────────────────

#[test]
fn feature_flags_and_store_settings_coexist_in_load_all() {
    let conn = setup();
    let reg = FeatureRegistry::simple_retail();
    Settings::save_features(&conn, &reg).unwrap();
    Settings::set_store_name(&conn, "Coexistence Store").unwrap();
    Settings::set_store_address(&conn, "789 Pine Rd").unwrap();
    Settings::set_sync_enabled(&conn, true).unwrap();

    let all = Settings::load_all(&conn).unwrap();

    // Should contain feature rows.
    assert!(
        all.iter().any(|(k, _)| k.starts_with("feature.")),
        "load_all should include feature flag rows"
    );

    // Should contain store settings.
    assert!(
        all.iter().any(|(k, _)| k == "store.name"),
        "load_all should include store.name"
    );
    assert!(
        all.iter().any(|(k, _)| k == "store.address"),
        "load_all should include store.address"
    );
    assert!(
        all.iter().any(|(k, _)| k == "sync_enabled"),
        "load_all should include sync_enabled"
    );
}

// ── Batch operations ─────────────────────────────────────────────────

#[test]
fn load_all_empty_db() {
    let conn = setup();
    let all = Settings::load_all(&conn).unwrap();
    assert!(all.is_empty(), "empty DB should return empty settings");
}

#[test]
fn set_batch_inserts_and_overwrites() {
    let conn = setup();
    let rows: Vec<(String, String)> = vec![
        ("key.a".into(), "value-a".into()),
        ("key.b".into(), "value-b".into()),
    ];
    Settings::set_batch(&conn, &rows).unwrap();

    assert_eq!(Settings::get(&conn, "key.a").unwrap(), Some("value-a".into()));
    assert_eq!(Settings::get(&conn, "key.b").unwrap(), Some("value-b".into()));

    // Overwrite one key and add a new one.
    let rows2: Vec<(String, String)> = vec![
        ("key.a".into(), "updated-a".into()),
        ("key.c".into(), "value-c".into()),
    ];
    Settings::set_batch(&conn, &rows2).unwrap();

    assert_eq!(Settings::get(&conn, "key.a").unwrap(), Some("updated-a".into()));
    assert_eq!(Settings::get(&conn, "key.b").unwrap(), Some("value-b".into()));
    assert_eq!(Settings::get(&conn, "key.c").unwrap(), Some("value-c".into()));
}

#[test]
fn load_all_ordered_by_key() {
    let conn = setup();
    let rows: Vec<(String, String)> = vec![
        ("z".into(), "last".into()),
        ("a".into(), "first".into()),
        ("m".into(), "middle".into()),
    ];
    Settings::set_batch(&conn, &rows).unwrap();

    let all = Settings::load_all(&conn).unwrap();
    assert_eq!(all.len(), 3);
    assert_eq!(all[0].0, "a");
    assert_eq!(all[1].0, "m");
    assert_eq!(all[2].0, "z");
}

#[test]
fn set_batch_empty_vec_is_noop() {
    let conn = setup();
    Settings::set_batch(&conn, &[]).unwrap();
    assert_eq!(Settings::load_all(&conn).unwrap().len(), 0);
}

#[test]
fn set_batch_large_number_of_settings() {
    let conn = setup();
    let rows: Vec<(String, String)> = (0..100)
        .map(|i| (format!("key.{i}"), format!("value.{i}")))
        .collect();
    Settings::set_batch(&conn, &rows).unwrap();

    let all = Settings::load_all(&conn).unwrap();
    assert_eq!(all.len(), 100, "all 100 settings should be stored");

    // Verify a few values.
    assert_eq!(Settings::get(&conn, "key.0").unwrap(), Some("value.0".into()));
    assert_eq!(Settings::get(&conn, "key.50").unwrap(), Some("value.50".into()));
    assert_eq!(Settings::get(&conn, "key.99").unwrap(), Some("value.99".into()));
}

// ── Large / edge-case values ─────────────────────────────────────────

#[test]
fn long_string_value_roundtrip() {
    let conn = setup();
    let long = "A".repeat(10_000);
    Settings::set(&conn, "long.value", &long).unwrap();
    let loaded = Settings::get(&conn, "long.value").unwrap().unwrap();
    assert_eq!(loaded.len(), 10_000);
    assert_eq!(loaded, long);
}

#[test]
fn unicode_value_roundtrip() {
    let conn = setup();
    let unicode = "日本語 Español العربية 🎉 你好 Привет";
    Settings::set(&conn, "unicode.test", unicode).unwrap();
    assert_eq!(
        Settings::get(&conn, "unicode.test").unwrap(),
        Some(unicode.into())
    );
}

#[test]
fn empty_value_roundtrip() {
    let conn = setup();
    Settings::set(&conn, "empty.val", "").unwrap();
    assert_eq!(
        Settings::get(&conn, "empty.val").unwrap(),
        Some("".into())
    );
}

#[test]
fn key_with_special_characters() {
    let conn = setup();
    Settings::set(&conn, "key.with.dots", "dots").unwrap();
    Settings::set(&conn, "key_with_underscores", "underscores").unwrap();
    Settings::set(&conn, "key-with-dashes", "dashes").unwrap();
    Settings::set(&conn, "key/with/slashes", "slashes").unwrap();

    assert_eq!(Settings::get(&conn, "key.with.dots").unwrap(), Some("dots".into()));
    assert_eq!(Settings::get(&conn, "key_with_underscores").unwrap(), Some("underscores".into()));
    assert_eq!(Settings::get(&conn, "key-with-dashes").unwrap(), Some("dashes".into()));
    assert_eq!(Settings::get(&conn, "key/with/slashes").unwrap(), Some("slashes".into()));
}

// ── Remove operations ────────────────────────────────────────────────

#[test]
fn remove_existing_setting() {
    let conn = setup();
    Settings::set(&conn, "temp.key", "temp-value").unwrap();
    assert!(Settings::remove(&conn, "temp.key").unwrap());
    assert_eq!(Settings::get(&conn, "temp.key").unwrap(), None);
}

#[test]
fn remove_nonexistent_returns_false() {
    let conn = setup();
    assert!(!Settings::remove(&conn, "nonexistent.key").unwrap());
}

#[test]
fn remove_typed_setting_restores_default() {
    let conn = setup();
    Settings::set_store_name(&conn, "Temp Name").unwrap();
    Settings::remove(&conn, "store.name").unwrap();
    assert_eq!(Settings::get_store_name(&conn).unwrap(), None);
}

#[test]
fn remove_idempotent() {
    let conn = setup();
    Settings::set(&conn, "k", "v").unwrap();
    assert!(Settings::remove(&conn, "k").unwrap());
    assert!(!Settings::remove(&conn, "k").unwrap());
}

// ── Overwrite operations ─────────────────────────────────────────────

#[test]
fn overwrite_with_same_value() {
    let conn = setup();
    Settings::set(&conn, "k", "same").unwrap();
    Settings::set(&conn, "k", "same").unwrap();
    assert_eq!(Settings::get(&conn, "k").unwrap(), Some("same".into()));
}

#[test]
fn overwrite_typed_setting_with_same_value() {
    let conn = setup();
    Settings::set_store_name(&conn, "Store").unwrap();
    Settings::set_store_name(&conn, "Store").unwrap();
    assert_eq!(Settings::get_store_name(&conn).unwrap(), Some("Store".into()));
}

// ── Store delegation (via Store API) ─────────────────────────────────

#[test]
fn store_get_set_setting() {
    let conn = setup();
    let s = store(&conn);
    assert_eq!(s.get_setting("my.key").unwrap(), None);
    s.set_setting("my.key", "hello").unwrap();
    assert_eq!(s.get_setting("my.key").unwrap(), Some("hello".into()));
}

#[test]
fn store_features_roundtrip() {
    let conn = setup();
    let s = store(&conn);
    let reg = FeatureRegistry::simple_retail();
    s.save_features(&reg).unwrap();
    let loaded = s.load_features().unwrap();
    assert_eq!(loaded, reg);
}

#[test]
fn store_name_via_store_api() {
    let conn = setup();
    let s = store(&conn);
    assert_eq!(s.get_store_name().unwrap(), None);
    s.set_store_name("Store API").unwrap();
    assert_eq!(s.get_store_name().unwrap(), Some("Store API".into()));
}

#[test]
fn store_namespace_isolation() {
    let conn = setup();

    // Settings with similar key prefixes should not interfere.
    Settings::set(&conn, "store.name", "Store A").unwrap();
    Settings::set(&conn, "store.name.version2", "Store B").unwrap();

    assert_eq!(
        Settings::get(&conn, "store.name").unwrap(),
        Some("Store A".into())
    );
    assert_eq!(
        Settings::get(&conn, "store.name.version2").unwrap(),
        Some("Store B".into())
    );
}
