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
}
