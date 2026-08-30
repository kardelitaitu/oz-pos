use super::*;

#[test]
fn roundtrip_minimal() {
    let payload = OzpkgPayload {
        products: vec![],
        categories: vec![],
        sales: None,
        customers: None,
        users: None,
        settings: None,
    };

    let mut features = HashMap::new();
    features.insert("simple-retail".into(), "1".into());

    let exported = export_ozpkg(
        "test-password-123",
        "My Store",
        "0.0.1",
        vec!["products".into()],
        features.clone(),
        &payload,
    )
    .unwrap();

    let (header, imported) = import_ozpkg(&exported, "test-password-123").unwrap();

    assert_eq!(header.version, FORMAT_VERSION);
    assert_eq!(header.store_name, "My Store");
    assert_eq!(header.app_version, "0.0.1");
    assert_eq!(header.data_types, vec!["products"]);
    assert_eq!(header.features, features);
    assert!(imported.products.is_empty());
    assert!(imported.categories.is_empty());
    assert!(imported.sales.is_none());
}

#[test]
fn roundtrip_with_data() {
    let payload = OzpkgPayload {
        products: vec![serde_json::json!({"sku": "LATTE", "name": "Latte", "price": 450})],
        categories: vec![
            serde_json::json!({"id": "cat-drinks", "name": "Drinks", "colour": "#06b6d4"}),
        ],
        sales: Some(vec![]),
        customers: Some(vec![serde_json::json!({"id": "cust-1", "name": "Alice"})]),
        users: None,
        settings: Some(vec![
            serde_json::json!({"key": "store.name", "value": "My Store"}),
        ]),
    };

    let exported = export_ozpkg(
        "strong-password-here!",
        "My Store",
        "0.0.1",
        vec!["products".into(), "categories".into(), "customers".into()],
        HashMap::new(),
        &payload,
    )
    .unwrap();

    let (_header, imported) = import_ozpkg(&exported, "strong-password-here!").unwrap();

    assert_eq!(imported.products.len(), 1);
    assert_eq!(imported.categories.len(), 1);
    assert_eq!(imported.customers.as_ref().unwrap().len(), 1);
    assert_eq!(imported.settings.as_ref().unwrap().len(), 1);
    assert!(imported.users.is_none());
}

#[test]
fn wrong_password_fails() {
    let payload = OzpkgPayload {
        products: vec![],
        categories: vec![],
        sales: None,
        customers: None,
        users: None,
        settings: None,
    };

    let exported = export_ozpkg(
        "correct-password",
        "Store",
        "0.0.1",
        vec![],
        HashMap::new(),
        &payload,
    )
    .unwrap();

    let result = import_ozpkg(&exported, "wrong-password");
    assert!(
        result.is_err(),
        "decryption should fail with wrong password"
    );
}

#[test]
fn corrupted_data_fails() {
    let payload = OzpkgPayload {
        products: vec![],
        categories: vec![],
        sales: None,
        customers: None,
        users: None,
        settings: None,
    };

    let mut exported = export_ozpkg(
        "password",
        "Store",
        "0.0.1",
        vec![],
        HashMap::new(),
        &payload,
    )
    .unwrap();

    // Corrupt a byte in the ciphertext.
    let last = exported.len() - 1;
    exported[last] ^= 0x01;

    let result = import_ozpkg(&exported, "password");
    assert!(
        result.is_err(),
        "decryption should fail with corrupted data"
    );
}

#[test]
fn empty_password_allowed() {
    let payload = OzpkgPayload {
        products: vec![],
        categories: vec![],
        sales: None,
        customers: None,
        users: None,
        settings: None,
    };

    let exported = export_ozpkg("", "Store", "0.0.1", vec![], HashMap::new(), &payload).unwrap();

    let result = import_ozpkg(&exported, "");
    assert!(
        result.is_ok(),
        "empty password should work (though not recommended)"
    );
}

#[test]
fn header_metadata_preserved() {
    let payload = OzpkgPayload {
        products: vec![],
        categories: vec![],
        sales: None,
        customers: None,
        users: None,
        settings: None,
    };

    let mut features = HashMap::new();
    features.insert("cash-payment".into(), "1".into());
    features.insert("barcode-scanning".into(), "1".into());

    let exported = export_ozpkg(
        "password",
        "Test Store",
        "0.1.0",
        vec!["products".into(), "settings".into()],
        features.clone(),
        &payload,
    )
    .unwrap();

    let (header, _) = import_ozpkg(&exported, "password").unwrap();

    assert_eq!(header.store_name, "Test Store");
    assert_eq!(header.app_version, "0.1.0");
    assert_eq!(header.data_types, vec!["products", "settings"]);
    assert_eq!(header.features, features);
    assert!(!header.created_at.is_empty());
    assert!(!header.salt.is_empty());
    assert!(!header.nonce.is_empty());
}

#[test]
fn large_payload_roundtrip() {
    let products: Vec<serde_json::Value> = (0..100)
        .map(|i| serde_json::json!({"sku": format!("SKU-{i:04}"), "name": format!("Product {i}"), "price": 100 + i}))
        .collect();

    let payload = OzpkgPayload {
        products: products.clone(),
        categories: vec![],
        sales: None,
        customers: None,
        users: None,
        settings: None,
    };

    let exported = export_ozpkg(
        "large-payload-password",
        "Big Store",
        "0.0.1",
        vec!["products".into()],
        HashMap::new(),
        &payload,
    )
    .unwrap();

    let (_header, imported) = import_ozpkg(&exported, "large-payload-password").unwrap();

    assert_eq!(imported.products.len(), 100);
    assert_eq!(imported.products[0]["sku"], "SKU-0000");
    assert_eq!(imported.products[99]["sku"], "SKU-0099");
}

// ── Header budget (B46) ───────────────────────────────────────────────
//
// The header is written into a fixed HEADER_LEN block by padding with
// spaces. Until the fix, an oversized header was silently TRUNCATED, so
// export_ozpkg() returned Ok and produced a file whose header block was
// invalid JSON — permanently unopenable. Export must fail loudly
// instead.
//
// There is deliberately no byte-exact boundary test: created_at uses
// to_rfc3339(), whose fractional-second digits vary in length, so a
// test pinned to the exact limit would be flaky by construction.

fn empty_payload() -> OzpkgPayload {
    OzpkgPayload {
        products: vec![],
        categories: vec![],
        sales: None,
        customers: None,
        users: None,
        settings: None,
    }
}

#[test]
fn export_with_typical_enabled_flags_errors_instead_of_truncating() {
    // Realistic production shape: an ordinary store name plus the set of
    // enabled feature flags a mid-size store carries. to_settings_rows()
    // emits EVERY enabled flag as "feature.<key>" -> "1" (~26 bytes
    // each) and the registry holds ~39 flags, so this is not crafted
    // input.
    let mut features = HashMap::new();
    for i in 0..16 {
        features.insert(format!("feature.pos.module_{i:02}"), "1".to_string());
    }
    let err = export_ozpkg(
        "password",
        "Kopi Senja",
        "0.0.33",
        vec!["products".to_string(), "categories".to_string()],
        features,
        &empty_payload(),
    )
    .expect_err("a header past HEADER_LEN must not be written as a truncated archive");
    let msg = err.to_string();
    assert!(
        msg.contains("512") && msg.contains("header"),
        "error should name the limit so the operator can act: {msg}"
    );
}

#[test]
fn export_with_oversized_store_name_errors_instead_of_truncating() {
    let long_name = "Toko ".repeat(100); // ~600 chars, well past HEADER_LEN
    let err = export_ozpkg(
        "password",
        &long_name,
        "0.0.33",
        vec!["products".to_string()],
        HashMap::new(),
        &empty_payload(),
    )
    .expect_err("a store name past HEADER_LEN must fail, not truncate");
    assert!(
        err.to_string().contains("512"),
        "error should name the limit: {err}"
    );
}

#[test]
fn export_with_header_under_the_budget_still_roundtrips() {
    // Guard against an over-eager check: a long-but-fitting header must
    // still export and import cleanly. Sized with a wide margin (~450
    // bytes) so the varying created_at length cannot straddle the limit.
    let name = "a".repeat(200);
    let exported = export_ozpkg(
        "password",
        &name,
        "0.0.33",
        vec!["products".to_string()],
        HashMap::new(),
        &empty_payload(),
    )
    .expect("a header comfortably under HEADER_LEN must export");
    let (header, _) = import_ozpkg(&exported, "password").unwrap();
    assert_eq!(header.store_name, name);
}
