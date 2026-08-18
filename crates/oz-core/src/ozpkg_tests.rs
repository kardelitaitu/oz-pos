
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

    let exported =
        export_ozpkg("", "Store", "0.0.1", vec![], HashMap::new(), &payload).unwrap();

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
