use super::*;
use crate::migrations;

fn seed_user(conn: &rusqlite::Connection) {
    conn.execute_batch(
        "INSERT INTO roles (id, name, permissions) VALUES
             ('role-staff', 'staff', '[\"sales:view\"]');
         INSERT INTO users (id, username, pin_hash, display_name, role_id, is_active, created_at, updated_at)
         VALUES ('u1', 'u1', 'h', 'U1', 'role-staff', 1,
                 '2026-01-01T00:00:00.000Z', '2026-01-01T00:00:00.000Z');",
    )
    .unwrap();
}

fn insert_assignment(
    conn: &rusqlite::Connection,
    mode: &str,
    branch_scope: &str,
    workspace_scope: &str,
) {
    conn.execute(
        "INSERT INTO assignments (user_id, role_id, scope_mode, branch_scope, workspace_scope)
         VALUES ('u1', 'role-staff', ?1, ?2, ?3)",
        params![mode, branch_scope, workspace_scope],
    )
    .unwrap();
}

#[test]
fn write_assignment_scope_joins_an_open_transaction() {
    let conn = migrations::fresh_db();
    seed_user(&conn);

    // The in-tx writer must not open a nested transaction: when called
    // inside a caller's transaction, the statements join it and a
    // subsequent rollback undoes the assignment write too.
    let tx = conn.unchecked_transaction().unwrap();
    let in_tx = Store::new(&tx);
    in_tx
        .write_assignment_scope(
            "u1",
            "role-staff",
            &AssignmentSpec {
                scope_mode: ScopeMode::Scoped,
                branches_all: false,
                branches: vec!["store-a".into()],
                workspaces_all: false,
                workspaces: vec!["retail-pos".into()],
            },
        )
        .unwrap();
    tx.rollback().unwrap();

    // Rolled back: no assignment row survives.
    let store = Store::new(&conn);
    assert!(store.assignment_for_user("u1").unwrap().is_none());
}

#[test]
fn assignment_for_user_loads_global_assignment() {
    let conn = migrations::fresh_db();
    seed_user(&conn);
    insert_assignment(&conn, "global", "all", "all");
    let store = Store::new(&conn);

    let a = store
        .assignment_for_user("u1")
        .unwrap()
        .expect("assignment");
    assert_eq!(a.user_id, "u1");
    assert_eq!(a.role_id, "role-staff");
    assert_eq!(a.scope_mode, ScopeMode::Global);
    assert!(a.branches.is_empty() && a.workspaces.is_empty());
}

#[test]
fn assignment_for_user_returns_none_when_absent() {
    let conn = migrations::fresh_db();
    seed_user(&conn);
    let store = Store::new(&conn);

    assert!(store.assignment_for_user("u1").unwrap().is_none());
    assert!(store.assignment_for_user("no-such-user").unwrap().is_none());
}

#[test]
fn assignment_for_user_loads_scoped_lists() {
    let conn = migrations::fresh_db();
    seed_user(&conn);
    insert_assignment(&conn, "scoped", "list", "list");
    conn.execute_batch(
        "INSERT INTO assignment_branches (assignment_user_id, branch_id) VALUES
             ('u1', 'store-a'), ('u1', 'store-b');
         INSERT INTO assignment_workspaces (assignment_user_id, workspace_key)
         VALUES ('u1', 'retail-pos');",
    )
    .unwrap();
    let store = Store::new(&conn);

    let a = store
        .assignment_for_user("u1")
        .unwrap()
        .expect("assignment");
    assert_eq!(a.scope_mode, ScopeMode::Scoped);
    assert!(!a.branches_all && !a.workspaces_all);
    assert_eq!(a.branches, vec!["store-a", "store-b"]);
    assert_eq!(a.workspaces, vec!["retail-pos"]);
}

#[test]
fn set_assignment_writes_scoped_dimensions() {
    let conn = migrations::fresh_db();
    seed_user(&conn);
    let store = Store::new(&conn);

    store
        .set_assignment(
            "u1",
            "role-staff",
            &AssignmentSpec {
                scope_mode: ScopeMode::Scoped,
                branches_all: false,
                branches: vec!["store-a".into(), "store-b".into()],
                workspaces_all: false,
                workspaces: vec!["retail-pos".into()],
            },
        )
        .unwrap();

    let a = store
        .assignment_for_user("u1")
        .unwrap()
        .expect("assignment");
    assert_eq!(a.scope_mode, ScopeMode::Scoped);
    assert!(!a.branches_all && !a.workspaces_all);
    assert_eq!(a.branches, vec!["store-a", "store-b"]);
    assert_eq!(a.workspaces, vec!["retail-pos"]);
}

#[test]
fn set_assignment_replaces_existing_scope_and_clears_stale_rows() {
    let conn = migrations::fresh_db();
    seed_user(&conn);
    let store = Store::new(&conn);
    store
        .set_assignment(
            "u1",
            "role-staff",
            &AssignmentSpec {
                scope_mode: ScopeMode::Scoped,
                branches_all: false,
                branches: vec!["store-a".into()],
                workspaces_all: false,
                workspaces: vec!["retail-pos".into()],
            },
        )
        .unwrap();

    // Switch to global all/all: the previous dimension rows must not
    // survive as stale grants (ADR #35 D5: empty lists never mean "all",
    // and `all` must mean every branch/workspace).
    store
        .set_assignment(
            "u1",
            "role-staff",
            &AssignmentSpec {
                scope_mode: ScopeMode::Global,
                branches_all: true,
                branches: vec![],
                workspaces_all: true,
                workspaces: vec![],
            },
        )
        .unwrap();

    let a = store
        .assignment_for_user("u1")
        .unwrap()
        .expect("assignment");
    assert_eq!(a.scope_mode, ScopeMode::Global);
    assert!(a.branches_all && a.workspaces_all);
    assert!(a.branches.is_empty() && a.workspaces.is_empty());
    // The dimension tables carry no stale rows either.
    let branches_left: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM assignment_branches WHERE assignment_user_id = 'u1'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let workspaces_left: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM assignment_workspaces WHERE assignment_user_id = 'u1'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(branches_left, 0);
    assert_eq!(workspaces_left, 0);
}

#[test]
fn matches_scope_global_ignores_dimensions() {
    let a = Assignment {
        user_id: "u1".into(),
        role_id: "role-staff".into(),
        scope_mode: ScopeMode::Global,
        branches_all: true,
        branches: vec![],
        workspaces_all: true,
        workspaces: vec![],
    };
    assert!(a.matches_scope(None, None));
    assert!(a.matches_scope(Some("store-a"), Some("retail-pos")));
    assert!(a.matches_scope(Some("anything"), Some("anything-else")));
}

#[test]
fn matches_scope_explicit_all_matches_any_context() {
    let a = Assignment {
        user_id: "u1".into(),
        role_id: "role-staff".into(),
        scope_mode: ScopeMode::Scoped,
        branches_all: true,
        branches: vec![],
        workspaces_all: true,
        workspaces: vec![],
    };
    assert!(a.matches_scope(None, None));
    assert!(a.matches_scope(Some("store-z"), Some("kds")));
}

#[test]
fn matches_scope_branch_list_requires_branch_in_scope() {
    let a = Assignment {
        user_id: "u1".into(),
        role_id: "role-staff".into(),
        scope_mode: ScopeMode::Scoped,
        branches_all: false,
        branches: vec!["store-a".into()],
        workspaces_all: true,
        workspaces: vec![],
    };
    assert!(a.matches_scope(Some("store-a"), None));
    assert!(!a.matches_scope(Some("store-b"), None));
    // No branch context on a list dimension denies (fail closed).
    assert!(!a.matches_scope(None, None));
}

#[test]
fn matches_scope_workspace_list_requires_workspace_in_scope() {
    let a = Assignment {
        user_id: "u1".into(),
        role_id: "role-staff".into(),
        scope_mode: ScopeMode::Scoped,
        branches_all: true,
        branches: vec![],
        workspaces_all: false,
        workspaces: vec!["retail-pos".into()],
    };
    assert!(a.matches_scope(None, Some("retail-pos")));
    assert!(!a.matches_scope(None, Some("kds")));
    assert!(!a.matches_scope(None, None));
}

#[test]
fn matches_scope_both_lists_require_combination() {
    let a = Assignment {
        user_id: "u1".into(),
        role_id: "role-staff".into(),
        scope_mode: ScopeMode::Scoped,
        branches_all: false,
        branches: vec!["store-a".into()],
        workspaces_all: false,
        workspaces: vec!["retail-pos".into()],
    };
    assert!(a.matches_scope(Some("store-a"), Some("retail-pos")));
    assert!(!a.matches_scope(Some("store-a"), Some("kds")));
    assert!(!a.matches_scope(Some("store-b"), Some("retail-pos")));
    assert!(!a.matches_scope(Some("store-b"), Some("kds")));
}

#[test]
fn matches_scope_empty_list_is_deny_not_all() {
    let a = Assignment {
        user_id: "u1".into(),
        role_id: "role-staff".into(),
        scope_mode: ScopeMode::Scoped,
        branches_all: false,
        branches: vec![],
        workspaces_all: true,
        workspaces: vec![],
    };
    // An empty list must never mean "all" — it denies everything.
    assert!(!a.matches_scope(Some("store-a"), None));
    assert!(!a.matches_scope(None, None));
}

#[test]
fn scope_mode_parse_roundtrips_and_unknown_is_none() {
    assert_eq!(ScopeMode::parse("global"), Some(ScopeMode::Global));
    assert_eq!(ScopeMode::parse("scoped"), Some(ScopeMode::Scoped));
    assert_eq!(ScopeMode::parse("bogus"), None);
    assert_eq!(ScopeMode::parse(""), None);
    assert_eq!(ScopeMode::Global.as_str(), "global");
    assert_eq!(ScopeMode::Scoped.as_str(), "scoped");
}
