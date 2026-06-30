//! Module manifest — JSON metadata file for every module.
//!
//! The manifest defines a module's identity, version, dependencies,
//! and required permissions. It is used by tooling for scaffolding,
//! documentation generation, and dependency analysis.

use serde::{Deserialize, Serialize};

use crate::error::KernelError;

/// Module manifest metadata.
///
/// Every module in OZ-POS must have a `manifest.json` at its root.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleManifest {
    /// Stable unique identifier for this module (e.g. `"sales"`, `"inventory"`).
    pub id: String,

    /// Human-readable display name (e.g. `"Sales"`, `"Inventory"`).
    pub name: String,

    /// Semantic version string (e.g. `"1.0.0"`).
    pub version: String,

    /// Module IDs that this module depends on.
    #[serde(default)]
    pub dependencies: Vec<String>,

    /// Permission strings required by this module (e.g. `["sales:void"]`).
    #[serde(default)]
    pub permissions: Vec<String>,

    /// Human-readable description of what this module provides.
    #[serde(default)]
    pub description: String,
}

impl ModuleManifest {
    /// Parse a `ModuleManifest` from a JSON string.
    ///
    /// # Errors
    ///
    /// Returns [`KernelError::ManifestParseError`] if the JSON is invalid
    /// or a required field is missing.
    pub fn from_json(json: &str) -> Result<Self, KernelError> {
        serde_json::from_str::<ModuleManifest>(json).map_err(|e| KernelError::ManifestParseError {
            module: "<unknown>".into(),
            message: e.to_string(),
        })
    }

    /// Serialize this manifest to a pretty-printed JSON string.
    pub fn to_json_pretty(&self) -> Result<String, KernelError> {
        serde_json::to_string_pretty(self)
            .map_err(|e| KernelError::Internal(format!("failed to serialize manifest: {e}")))
    }

    /// Validate that all required fields are present and well-formed.
    ///
    /// Returns `Ok(())` if the manifest is valid, or a descriptive error.
    pub fn validate(&self) -> Result<(), KernelError> {
        if self.id.is_empty() {
            return Err(KernelError::ManifestParseError {
                module: self.id.clone(),
                message: "manifest id must not be empty".into(),
            });
        }
        if self.name.is_empty() {
            return Err(KernelError::ManifestParseError {
                module: self.id.clone(),
                message: "manifest name must not be empty".into(),
            });
        }
        if self.version.is_empty() {
            return Err(KernelError::ManifestParseError {
                module: self.id.clone(),
                message: "manifest version must not be empty".into(),
            });
        }
        // Validate semver format (basic check: X.Y.Z)
        let parts: Vec<&str> = self.version.split('.').collect();
        if parts.len() != 3 || parts.iter().any(|p| p.parse::<u64>().is_err()) {
            return Err(KernelError::ManifestParseError {
                module: self.id.clone(),
                message: format!(
                    "invalid version '{}': expected semver format (X.Y.Z)",
                    self.version
                ),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_manifest() {
        let json = r#"{
            "id": "sales",
            "name": "Sales",
            "version": "1.0.0",
            "dependencies": ["inventory"],
            "permissions": ["sales:void"],
            "description": "Core sales module"
        }"#;

        let manifest = ModuleManifest::from_json(json).unwrap();
        assert_eq!(manifest.id, "sales");
        assert_eq!(manifest.name, "Sales");
        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(manifest.dependencies, vec!["inventory"]);
        assert_eq!(manifest.permissions, vec!["sales:void"]);
        assert_eq!(manifest.description, "Core sales module");
    }

    #[test]
    fn parse_minimal_manifest() {
        let json = r#"{
            "id": "inventory",
            "name": "Inventory",
            "version": "1.0.0"
        }"#;

        let manifest = ModuleManifest::from_json(json).unwrap();
        assert_eq!(manifest.id, "inventory");
        assert!(manifest.dependencies.is_empty());
        assert!(manifest.permissions.is_empty());
        assert!(manifest.description.is_empty());
    }

    #[test]
    fn parse_invalid_json() {
        let result = ModuleManifest::from_json("{invalid}");
        assert!(result.is_err());
        match result.unwrap_err() {
            KernelError::ManifestParseError { .. } => {} // expected
            other => panic!("expected ManifestParseError, got {other:?}"),
        }
    }

    #[test]
    fn to_json_pretty_roundtrip() {
        let manifest = ModuleManifest {
            id: "test".into(),
            name: "Test Module".into(),
            version: "1.2.3".into(),
            dependencies: vec!["core".into()],
            permissions: vec!["test:read".into()],
            description: "A test".into(),
        };

        let json = manifest.to_json_pretty().unwrap();
        let parsed = ModuleManifest::from_json(&json).unwrap();
        assert_eq!(parsed.id, manifest.id);
        assert_eq!(parsed.name, manifest.name);
        assert_eq!(parsed.version, manifest.version);
        assert_eq!(parsed.dependencies, manifest.dependencies);
        assert_eq!(parsed.permissions, manifest.permissions);
    }

    #[test]
    fn validate_valid_manifest() {
        let manifest = ModuleManifest {
            id: "test".into(),
            name: "Test".into(),
            version: "1.0.0".into(),
            dependencies: vec![],
            permissions: vec![],
            description: String::new(),
        };
        assert!(manifest.validate().is_ok());
    }

    #[test]
    fn validate_empty_id() {
        let manifest = ModuleManifest {
            id: "".into(),
            name: "Test".into(),
            version: "1.0.0".into(),
            dependencies: vec![],
            permissions: vec![],
            description: String::new(),
        };
        assert!(manifest.validate().is_err());
    }

    #[test]
    fn validate_empty_name() {
        let manifest = ModuleManifest {
            id: "test".into(),
            name: "".into(),
            version: "1.0.0".into(),
            dependencies: vec![],
            permissions: vec![],
            description: String::new(),
        };
        assert!(manifest.validate().is_err());
    }

    #[test]
    fn validate_invalid_version() {
        let manifest = ModuleManifest {
            id: "test".into(),
            name: "Test".into(),
            version: "abc".into(),
            dependencies: vec![],
            permissions: vec![],
            description: String::new(),
        };
        assert!(manifest.validate().is_err());
    }

    #[test]
    fn validate_missing_version_is_empty() {
        let manifest = ModuleManifest {
            id: "test".into(),
            name: "Test".into(),
            version: "".into(),
            dependencies: vec![],
            permissions: vec![],
            description: String::new(),
        };
        assert!(manifest.validate().is_err());
    }

    #[test]
    fn validate_version_too_many_parts() {
        let manifest = ModuleManifest {
            id: "test".into(),
            name: "Test".into(),
            version: "1.0.0.0".into(),
            dependencies: vec![],
            permissions: vec![],
            description: String::new(),
        };
        assert!(manifest.validate().is_err());
    }

    #[test]
    fn serde_roundtrip_with_defaults() {
        let json = r#"{"id":"x","name":"X","version":"1.0.0"}"#;
        let manifest = ModuleManifest::from_json(json).unwrap();
        assert_eq!(manifest.dependencies.len(), 0);
        assert_eq!(manifest.permissions.len(), 0);
    }
}
