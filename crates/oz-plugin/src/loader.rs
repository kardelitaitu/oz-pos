use std::path::{Path, PathBuf};

use crate::error::PluginError;
use crate::manifest::PluginManifest;

/// A loaded plugin with its manifest and script paths.
#[derive(Debug, Clone)]
pub struct LoadedPlugin {
    pub manifest: PluginManifest,
    pub directory: PathBuf,
    pub scripts: Vec<PathBuf>,
}

/// A registry of all loaded plugins.
#[derive(Debug, Default)]
pub struct PluginRegistry {
    pub plugins: Vec<LoadedPlugin>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self { plugins: vec![] }
    }

    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }

    pub fn len(&self) -> usize {
        self.plugins.len()
    }
}

/// Scan a directory for plugin manifests and load them.
pub fn load_plugins(plugins_dir: &Path) -> Result<PluginRegistry, PluginError> {
    let mut registry = PluginRegistry::new();

    if !plugins_dir.exists() {
        return Ok(registry);
    }

    for entry in std::fs::read_dir(plugins_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let manifest_path = path.join("plugin.toml");
        if !manifest_path.exists() {
            continue;
        }

        match PluginManifest::load(&manifest_path) {
            Ok(manifest) => {
                let scripts: Vec<PathBuf> = manifest
                    .capabilities
                    .scripts
                    .iter()
                    .map(|s| path.join(s))
                    .filter(|p| p.exists())
                    .collect();

                let plugin = LoadedPlugin {
                    manifest,
                    directory: path,
                    scripts,
                };
                tracing::info!(name = %plugin.manifest.plugin.name, "plugin loaded");
                registry.plugins.push(plugin);
            }
            Err(e) => {
                tracing::warn!(dir = %path.display(), error = %e, "failed to load plugin");
            }
        }
    }

    Ok(registry)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_empty_directory() {
        let dir = tempfile::tempdir().unwrap();
        let registry = load_plugins(dir.path()).unwrap();
        assert!(registry.is_empty());
    }

    #[test]
    fn load_single_plugin() {
        let dir = tempfile::tempdir().unwrap();
        let plugin_dir = dir.path().join("my-plugin");
        std::fs::create_dir(&plugin_dir).unwrap();

        let manifest = r#"
[plugin]
name = "my-plugin"
version = "1.0.0"

[capabilities]
scripts = ["test.lua"]
"#;
        std::fs::write(plugin_dir.join("plugin.toml"), manifest).unwrap();
        std::fs::write(plugin_dir.join("test.lua"), "-- test script").unwrap();

        let registry = load_plugins(dir.path()).unwrap();
        assert_eq!(registry.len(), 1);
        assert_eq!(registry.plugins[0].manifest.plugin.name, "my-plugin");
        assert_eq!(registry.plugins[0].scripts.len(), 1);
    }

    #[test]
    fn skip_directories_without_manifest() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("no-manifest")).unwrap();
        let registry = load_plugins(dir.path()).unwrap();
        assert!(registry.is_empty());
    }

    // ── PluginRegistry struct tests ──────────────────────────────────

    #[test]
    fn registry_default_is_empty() {
        let reg = PluginRegistry::default();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
    }

    #[test]
    fn registry_new_equals_default() {
        let new = PluginRegistry::new();
        let default = PluginRegistry::default();
        assert_eq!(new.len(), default.len());
        assert!(new.is_empty());
        assert!(default.is_empty());
    }

    #[test]
    fn registry_len_reflects_plugins() {
        let dir = tempfile::tempdir().unwrap();
        for i in 1..=3 {
            let plugin_dir = dir.path().join(format!("plugin-{i}"));
            std::fs::create_dir(&plugin_dir).unwrap();
            std::fs::write(
                plugin_dir.join("plugin.toml"),
                format!("[plugin]\nname = \"plugin-{i}\"\nversion = \"1.0.0\"\n\n[capabilities]\nscripts = []\n"),
            )
            .unwrap();
        }
        let registry = load_plugins(dir.path()).unwrap();
        assert_eq!(registry.len(), 3);
        assert!(!registry.is_empty());
    }

    // ── LoadedPlugin struct tests ────────────────────────────────────

    #[test]
    fn loaded_plugin_debug() {
        let dir = tempfile::tempdir().unwrap();
        let plugin_dir = dir.path().join("debug-plugin");
        std::fs::create_dir(&plugin_dir).unwrap();
        std::fs::write(
            plugin_dir.join("plugin.toml"),
            "[plugin]\nname = \"debug-plugin\"\nversion = \"1.0.0\"\n",
        )
        .unwrap();
        let registry = load_plugins(dir.path()).unwrap();
        let debug = format!("{:?}", registry.plugins[0]);
        assert!(debug.contains("debug-plugin"));
    }

    #[test]
    fn plugin_with_missing_scripts_dir() {
        let dir = tempfile::tempdir().unwrap();
        let plugin_dir = dir.path().join("no-scripts");
        std::fs::create_dir(&plugin_dir).unwrap();
        std::fs::write(
            plugin_dir.join("plugin.toml"),
            "[plugin]\nname = \"no-scripts\"\nversion = \"1.0.0\"\n\n[capabilities]\nscripts = [\"missing.lua\"]\n",
        )
        .unwrap();
        let registry = load_plugins(dir.path()).unwrap();
        assert_eq!(registry.len(), 1);
        assert!(registry.plugins[0].scripts.is_empty());
    }

    #[test]
    fn load_nonexistent_directory() {
        let registry = load_plugins(std::path::Path::new("/nonexistent/path/for/plugins")).unwrap();
        assert!(registry.is_empty());
    }
}
