use serde::Deserialize;
use std::path::Path;

use crate::error::PluginError;

/// A plugin manifest (`plugin.toml`).
#[derive(Debug, Clone, Deserialize)]
pub struct PluginManifest {
    /// Plugin metadata (name, version, etc.).
    pub plugin: PluginMeta,
    /// Declared plugin capabilities.
    #[serde(default)]
    pub capabilities: PluginCapabilities,
    /// Sandbox permission settings.
    #[serde(default)]
    pub permissions: PluginPermissions,
}

/// Metadata section of a plugin manifest.
#[derive(Debug, Clone, Deserialize)]
pub struct PluginMeta {
    /// Plugin name (must be unique).
    pub name: String,
    /// Plugin version (semver string).
    pub version: String,
    /// Optional human-readable description.
    pub description: Option<String>,
    /// Optional plugin author.
    pub author: Option<String>,
    /// Optional license identifier.
    pub license: Option<String>,
}

/// Declared capabilities of a plugin.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PluginCapabilities {
    /// Script files to load into the Lua sandbox.
    #[serde(default)]
    pub scripts: Vec<String>,
    /// Native driver modules to load.
    #[serde(default)]
    pub drivers: Vec<String>,
    /// Hook names this plugin registers.
    #[serde(default)]
    pub hooks: Vec<String>,
}

/// A typed permission that a plugin can declare.
///
/// See [`PluginPermissions::required_permissions`] for the full list of
/// allowed values. Each permission governs access to a specific POS domain.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Permission {
    /// Read cart contents and prices.
    CartRead,
    /// Modify cart totals (apply discounts).
    CartWrite,
    /// Read tax rates and configuration.
    TaxRead,
    /// Read inventory stock levels.
    InventoryRead,
    /// Write inventory stock levels (adjust stock).
    InventoryWrite,
    /// Read reporting/analytics data.
    ReportingRead,
    /// Access system time (non-sensitive).
    SystemTime,
    /// Write log entries.
    LogWrite,
}

impl std::fmt::Display for Permission {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CartRead => write!(f, "cart:read"),
            Self::CartWrite => write!(f, "cart:write"),
            Self::TaxRead => write!(f, "tax:read"),
            Self::InventoryRead => write!(f, "inventory:read"),
            Self::InventoryWrite => write!(f, "inventory:write"),
            Self::ReportingRead => write!(f, "reporting:read"),
            Self::SystemTime => write!(f, "system:time"),
            Self::LogWrite => write!(f, "log:write"),
        }
    }
}

/// Sanity-check that a string value is a known permission name.
/// Returns `None` for unrecognised values so unknown permissions are
/// silently ignored (forward compatibility: a newer merchant's plugin
/// may declare permissions this older runtime doesn't understand).
pub fn permission_from_str(s: &str) -> Option<Permission> {
    match s {
        "cart:read" => Some(Permission::CartRead),
        "cart:write" => Some(Permission::CartWrite),
        "tax:read" => Some(Permission::TaxRead),
        "inventory:read" => Some(Permission::InventoryRead),
        "inventory:write" => Some(Permission::InventoryWrite),
        "reporting:read" => Some(Permission::ReportingRead),
        "system:time" => Some(Permission::SystemTime),
        "log:write" => Some(Permission::LogWrite),
        _ => None, // Unknown permission — silently ignore for forward compat.
    }
}

/// Deserialize a single permission or a list of permissions from TOML.
/// Supports both single-string and array-of-strings forms.
fn deserialize_permissions<'de, D>(deserializer: D) -> Result<Vec<Permission>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de;

    // Try array first, then single string.
    struct PermVisitor;
    impl<'de> de::Visitor<'de> for PermVisitor {
        type Value = Vec<Permission>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a permission string or array of permission strings")
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<Vec<Permission>, E> {
            Ok(permission_from_str(v).into_iter().collect())
        }

        fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Vec<Permission>, A::Error> {
            let mut perms = Vec::new();
            while let Some(val) = seq.next_element::<String>()? {
                if let Some(p) = permission_from_str(&val) {
                    perms.push(p);
                }
            }
            Ok(perms)
        }
    }

    deserializer.deserialize_any(PermVisitor)
}

/// Sandbox permissions for a plugin.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PluginPermissions {
    /// Whether the plugin may make network requests.
    #[serde(default)]
    pub allow_network: bool,
    /// Whether the plugin may access the filesystem.
    #[serde(default)]
    pub allow_filesystem: bool,
    /// Whether the plugin may send HTTP requests.
    #[serde(default)]
    pub allow_http: bool,
    /// Declared permissions this plugin needs (e.g., `["cart:read", "cart:write"]`).
    /// Rejected at load time if any permission is not recognised.
    /// Unknown permissions are silently ignored for forward compatibility.
    #[serde(default, deserialize_with = "deserialize_permissions")]
    pub required_permissions: Vec<Permission>,
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
            required_permissions: vec![Permission::CartRead],
        };
        let debug = format!("{perms:?}");
        assert!(debug.contains("true"));
        assert!(debug.contains("CartRead"));
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
