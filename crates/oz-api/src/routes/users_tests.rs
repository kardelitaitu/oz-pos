use super::*;

// ── CreateUserRequest deserialization ───────────────────────

#[test]
fn create_user_request_minimal() {
    let json = r#"{"username":"alice","pin_hash":"hash123","display_name":"Alice","role_id":"role-staff"}"#;
    let req: CreateUserRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.username, "alice");
    assert_eq!(req.pin_hash, "hash123");
    assert_eq!(req.display_name, "Alice");
    assert_eq!(req.role_id, "role-staff");
}

#[test]
fn create_user_request_owner_role() {
    let json =
        r#"{"username":"owner","pin_hash":"abc","display_name":"Owner","role_id":"role-owner"}"#;
    let req: CreateUserRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.username, "owner");
    assert_eq!(req.role_id, "role-owner");
}
