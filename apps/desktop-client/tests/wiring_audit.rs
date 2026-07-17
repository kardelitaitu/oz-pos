//! Tauri v2 wiring audit — exposes duplicate command registrations
//! in the `generate_handler!` macro that would cause a runtime panic.
//!
//! Bug: `commands::staff::list_staff` and `commands::sync::sync_pull`
//! were each listed twice in `tauri::generate_handler!`. Tauri v2 panics
//! at runtime when duplicate command paths appear in the macro.
//!
//! This test parses the `lib.rs` source and asserts no duplicate entries
//! exist, preventing future regressions.

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

/// Extract all command paths from the `generate_handler![...]` block
/// in a lib.rs file. Returns them in order of appearance.
fn extract_handler_commands(src: &str) -> Vec<String> {
    let start_marker = "generate_handler![";
    let start = match src.find(start_marker) {
        Some(idx) => idx + start_marker.len(),
        None => return Vec::new(),
    };

    // Find the matching closing `]` by counting brackets.
    let rest = &src[start..];
    let mut depth = 1;
    let mut end = 0;
    for (i, ch) in rest.chars().enumerate() {
        match ch {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    end = i;
                    break;
                }
            }
            _ => {}
        }
    }

    let block = &rest[..end];

    // Each line in the block is either a command path (e.g. `commands::staff::list_staff,`)
    // or a comment. Extract command paths by looking for lines containing `commands::`.
    block
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with("//") || trimmed.is_empty() {
                return None;
            }
            // Remove trailing comma and whitespace
            let path = trimmed.trim_end_matches(',').trim();
            if path.starts_with("commands::") {
                Some(path.to_string())
            } else {
                None
            }
        })
        .collect()
}

fn find_lib_rs(app_dir: &str) -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let candidates = [
        PathBuf::from(manifest_dir).join(app_dir).join("src/lib.rs"),
        PathBuf::from(manifest_dir)
            .join("..")
            .join(app_dir)
            .join("src/lib.rs"),
    ];
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    candidates[0].clone()
}

#[test]
fn desktop_client_no_duplicate_handler_commands() {
    let lib_rs = find_lib_rs(".");
    let src = fs::read_to_string(&lib_rs)
        .unwrap_or_else(|e| panic!("failed to read {:?}: {}", lib_rs, e));

    let commands = extract_handler_commands(&src);
    assert!(
        !commands.is_empty(),
        "no generate_handler commands found in {:?}",
        lib_rs
    );

    let mut seen = HashSet::new();
    let mut duplicates = Vec::new();
    for cmd in &commands {
        if !seen.insert(cmd.clone()) {
            duplicates.push(cmd.clone());
        }
    }

    assert!(
        duplicates.is_empty(),
        "Duplicate command(s) found in desktop-client generate_handler!: {:?}. \
         Tauri v2 panics at runtime when the same command path appears twice.",
        duplicates
    );
}

#[test]
fn tablet_client_no_duplicate_handler_commands() {
    let lib_rs = find_lib_rs("../tablet-client");
    let src = fs::read_to_string(&lib_rs)
        .unwrap_or_else(|e| panic!("failed to read {:?}: {}", lib_rs, e));

    let commands = extract_handler_commands(&src);
    assert!(
        !commands.is_empty(),
        "no generate_handler commands found in {:?}",
        lib_rs
    );

    let mut seen = HashSet::new();
    let mut duplicates = Vec::new();
    for cmd in &commands {
        if !seen.insert(cmd.clone()) {
            duplicates.push(cmd.clone());
        }
    }

    assert!(
        duplicates.is_empty(),
        "Duplicate command(s) found in tablet-client generate_handler!: {:?}. \
         Tauri v2 panics at runtime when the same command path appears twice.",
        duplicates
    );
}

/// Verify that `list_staff` and `sync_pull` specifically are registered
/// exactly once each (the two commands that were duplicated before the fix).
#[test]
fn desktop_client_specific_commands_registered_once() {
    let lib_rs = find_lib_rs(".");
    let src = fs::read_to_string(&lib_rs).expect("failed to read lib.rs");

    let commands = extract_handler_commands(&src);

    let list_staff_count = commands
        .iter()
        .filter(|c| *c == "commands::staff::list_staff")
        .count();
    let sync_pull_count = commands
        .iter()
        .filter(|c| *c == "commands::sync::sync_pull")
        .count();

    assert_eq!(
        list_staff_count, 1,
        "commands::staff::list_staff should be registered exactly once, found {}",
        list_staff_count
    );
    assert_eq!(
        sync_pull_count, 1,
        "commands::sync::sync_pull should be registered exactly once, found {}",
        sync_pull_count
    );
}
