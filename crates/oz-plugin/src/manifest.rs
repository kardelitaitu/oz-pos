use serde::Deserialize;
use std::path::Path;

use crate::error::PluginError;

/// A plugin manifest (`plugin.toml`).
#[derive(Debug, Clone, Deserialize)]
pub struct PluginManifest {
    pub plugin: PluginMeta,
    #[serde(default)]
    pub capabilities: PluginCapabilities,
    #[serde(default)]
    pub permissions: PluginPermissions,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PluginMeta {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub license: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct PluginCapabilities {
    #[serde(default)]
    pub scripts: Vec<String>,
    #[serde(default)]
    pub drivers: Vec<String>,
    #[serde(default)]
    pub hooks: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct PluginPermissions {
    #[serde(default)]
    pub allow_network: bool,
    #[serde(default)]
    pub allow_filesystem: bool,
    #[serde(default)]
    pub allow_http: bool,
}

impl PluginManifest {
    /// Load a manifest from a `plugin.toml` file.
    pub fn load(path: &Path) -> Result<Self, PluginError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| PluginError::Manifest(format!("cannot read {path:?}: {e}")))?;
        toml::from_str(&content)
            .map_err(|e| PluginError::Manifest(format!("invalid manifest {path:?}: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_valid_manifest() {
        let toml = r#"
[plugin]
name = "test-plugin"
version = "1.0.0"
description = "A test plugin"

[capabilities]
scripts = ["test.lua"]

[permissions]
allow_network = false
allow_filesystem = false
"#;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("plugin.toml");
        std::fs::write(&path, toml).unwrap();
        let manifest = PluginManifest::load(&path).unwrap();
        assert_eq!(manifest.plugin.name, "test-plugin");
        assert_eq!(manifest.capabilities.scripts, vec!["test.lua"]);
        assert!(!manifest.permissions.allow_network);
    }

    #[test]
    fn load_invalid_manifest_fails() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("plugin.toml");
        std::fs::write(&path, "not: valid: toml").unwrap();
        assert!(PluginManifest::load(&path).is_err());
    }

    // ── Deserialization edge cases ───────────────────────────────────

    #[test]
    fn minimal_manifest_only_name_and_version() {
        let toml = "[plugin]\nname = \"minimal\"\nversion = \"0.1.0\"\n";
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("plugin.toml");
        std::fs::write(&path, toml).unwrap();
        let manifest = PluginManifest::load(&path).unwrap();
        assert_eq!(manifest.plugin.name, "minimal");
        assert_eq!(manifest.plugin.version, "0.1.0");
        assert!(manifest.plugin.description.is_none());
        assert!(manifest.plugin.author.is_none());
        assert!(manifest.plugin.license.is_none());
        assert!(manifest.capabilities.scripts.is_empty());
        assert!(manifest.capabilities.drivers.is_empty());
        assert!(manifest.capabilities.hooks.is_empty());
        assert!(!manifest.permissions.allow_network);
        assert!(!manifest.permissions.allow_filesystem);
        assert!(!manifest.permissions.allow_http);
    }

    #[test]
    fn manifest_with_all_optional_fields() {
        let toml = r#"
[plugin]
name = "full"
version = "2.0.0"
description = "Fully featured plugin"
author = "Alice"
license = "MIT"

[capabilities]
scripts = ["a.lua", "b.lua"]
drivers = ["printer.so"]
hooks = ["on_sale", "on_refund"]

[permissions]
allow_network = true
allow_filesystem = true
allow_http = false
"#;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("plugin.toml");
        std::fs::write(&path, toml).unwrap();
        let manifest = PluginManifest::load(&path).unwrap();
        assert_eq!(manifest.plugin.name, "full");
        assert_eq!(manifest.plugin.version, "2.0.0");
        assert_eq!(
            manifest.plugin.description.as_deref(),
            Some("Fully featured plugin")
        );
        assert_eq!(manifest.plugin.author.as_deref(), Some("Alice"));
        assert_eq!(manifest.plugin.license.as_deref(), Some("MIT"));
        assert_eq!(manifest.capabilities.scripts.len(), 2);
        assert_eq!(manifest.capabilities.drivers.len(), 1);
        assert_eq!(manifest.capabilities.hooks.len(), 2);
        assert!(manifest.permissions.allow_network);
        assert!(manifest.permissions.allow_filesystem);
        assert!(!manifest.permissions.allow_http);
    }

    #[test]
    fn manifest_all_permissions_true() {
        let toml = r#"
[plugin]
name = "networked"
version = "1.0.0"

[permissions]
allow_network = true
allow_filesystem = true
allow_http = true
"#;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("plugin.toml");
        std::fs::write(&path, toml).unwrap();
        let manifest = PluginManifest::load(&path).unwrap();
        assert!(manifest.permissions.allow_network);
        assert!(manifest.permissions.allow_filesystem);
        assert!(manifest.permissions.allow_http);
    }

    #[test]
    fn manifest_capabilities_default_to_empty() {
        let toml = "[plugin]\nname = \"no-caps\"\nversion = \"1.0.0\"\n";
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("plugin.toml");
        std::fs::write(&path, toml).unwrap();
        let manifest = PluginManifest::load(&path).unwrap();
        assert!(manifest.capabilities.scripts.is_empty());
        assert!(manifest.capabilities.drivers.is_empty());
        assert!(manifest.capabilities.hooks.is_empty());
    }

    #[test]
    fn manifest_permissions_default_to_false() {
        let toml = "[plugin]\nname = \"safe\"\nversion = \"1.0.0\"\n";
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("plugin.toml");
        std::fs::write(&path, toml).unwrap();
        let manifest = PluginManifest::load(&path).unwrap();
        assert!(!manifest.permissions.allow_network);
        assert!(!manifest.permissions.allow_filesystem);
        assert!(!manifest.permissions.allow_http);
    }

    // ── Struct Debug tests ───────────────────────────────────────────

    #[test]
    fn plugin_meta_debug() {
        let meta = PluginMeta {
            name: "test".into(),
            version: "1.0.0".into(),
            description: Some("desc".into()),
            author: None,
            license: Some("MIT".into()),
        };
        let debug = format!("{meta:?}");
        assert!(debug.contains("test"));
        assert!(debug.contains("desc"));
        assert!(debug.contains("MIT"));
    }

    #[test]
    fn plugin_capabilities_debug() {
        let caps = PluginCapabilities {
            scripts: vec!["s1.lua".into()],
            drivers: vec!["d1.so".into()],
            hooks: vec![],
        };
        let debug = format!("{caps:?}");
        assert!(debug.contains("s1.lua"));
        assert!(debug.contains("d1.so"));
    }

    #[test]
    fn plugin_permissions_debug() {
        let perms = PluginPermissions {
            allow_network: true,
            allow_filesystem: false,
            allow_http: true,
        };
        let debug = format!("{perms:?}");
        assert!(debug.contains("true"));
    }

    #[test]
    fn manifest_debug_output() {
        let toml = "[plugin]\nname = \"debug-manifest\"\nversion = \"1.0.0\"\n";
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("plugin.toml");
        std::fs::write(&path, toml).unwrap();
        let manifest = PluginManifest::load(&path).unwrap();
        let debug = format!("{manifest:?}");
        assert!(debug.contains("debug-manifest"));
    }
}
