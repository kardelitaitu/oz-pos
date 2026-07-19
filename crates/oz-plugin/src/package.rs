//! `.ozpkg` archive reader.
//!
//! An `.ozpkg` file is a zip archive containing:
//!
//! - `manifest.json` — required, validates against module manifest schema
//! - `*.lua` files — Lua scripts
//! - `*.sql` files — SQLite migration scripts
//!
//! # Example
//!
//! ```no_run
//! # use oz_plugin::package::OzpkArchive;
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let archive = OzpkArchive::open("path/to/plugin.ozpkg")?;
//! let manifest = archive.manifest();
//! let scripts = archive.scripts();
//! # Ok(())
//! # }

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::error::PluginError;

/// The recognised entry types inside an `.ozpkg` archive.
#[derive(Debug, Clone, PartialEq)]
pub enum OzpkEntry {
    /// The parsed `manifest.json` value.
    Manifest(Value),
    /// A Lua script — filename stored as canonical path inside archive.
    Script(String),
    /// A SQL migration script.
    Migration(String),
    /// Any other file not recognised as script or migration.
    Other(String),
}

impl OzpkEntry {
    /// The filename (last component) of this entry.
    pub fn filename(&self) -> &str {
        match self {
            OzpkEntry::Manifest(_) => "manifest.json",
            OzpkEntry::Script(name) | OzpkEntry::Migration(name) | OzpkEntry::Other(name) => name,
        }
    }

    /// Returns `true` if this entry is a Lua script.
    pub fn is_script(&self) -> bool {
        matches!(self, OzpkEntry::Script(_))
    }

    /// Returns `true` if this entry is a migration.
    pub fn is_migration(&self) -> bool {
        matches!(self, OzpkEntry::Migration(_))
    }
}

/// An opened `.ozpkg` archive.
#[derive(Debug, Clone)]
pub struct OzpkArchive {
    path: PathBuf,
    parsed_manifest: Option<Value>,
    entries: Vec<(String, OzpkEntry)>,
    entry_contents: HashMap<String, Vec<u8>>,
}

impl OzpkArchive {
    /// Open and parse an `.ozpkg` archive from a file path.
    ///
    /// Reads the entire archive into memory, validates that `manifest.json`
    /// exists and is valid JSON, and classifies all entries by extension.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, PluginError> {
        let path = path.as_ref().to_path_buf();
        let file = std::fs::File::open(&path)?;
        let mut reader = std::io::BufReader::new(file);
        Self::from_reader(&mut reader, path)
    }

    /// Open an `.ozpkg` archive from an in-memory byte buffer.
    ///
    /// Used primarily in tests and when loading from a network source.
    pub fn from_bytes(bytes: &[u8], name: impl Into<PathBuf>) -> Result<Self, PluginError> {
        let path: PathBuf = name.into();
        let mut reader = std::io::Cursor::new(bytes);
        Self::from_reader(&mut reader, path)
    }

    /// Shared constructor from any `Read + Seek` source.
    fn from_reader<R>(reader: &mut R, path: PathBuf) -> Result<Self, PluginError>
    where
        R: std::io::Read + std::io::Seek,
    {
        let mut archive =
            zip::ZipArchive::new(reader).map_err(|e| PluginError::Archive(e.to_string()))?;

        let mut parsed_manifest: Option<Value> = None;
        let mut entries: Vec<(String, OzpkEntry)> = Vec::new();
        let mut entry_contents: HashMap<String, Vec<u8>> = HashMap::new();
        let mut manifest_found = false;

        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|e| PluginError::Archive(e.to_string()))?;

            // Skip directories
            if file.is_dir() {
                continue;
            }

            let name = file.name().to_string();
            let mut data = Vec::new();
            file.read_to_end(&mut data)
                .map_err(|e| PluginError::Archive(e.to_string()))?;

            // Normalise path separators to forward-slash for consistent matching
            let normalised = name.replace('\\', "/");
            let filename = normalised
                .rsplit('/')
                .next()
                .unwrap_or(&normalised)
                .to_string();

            let entry = if filename == "manifest.json" {
                manifest_found = true;
                let value: Value = serde_json::from_slice(&data)
                    .map_err(|e| PluginError::Archive(format!("invalid manifest.json: {e}")))?;
                parsed_manifest = Some(value.clone());
                OzpkEntry::Manifest(value)
            } else if filename.ends_with(".lua") {
                OzpkEntry::Script(filename.clone())
            } else if filename.ends_with(".sql") {
                OzpkEntry::Migration(filename.clone())
            } else {
                OzpkEntry::Other(filename.clone())
            };

            entries.push((name.clone(), entry));
            entry_contents.insert(name, data);
        }

        if !manifest_found {
            return Err(PluginError::Archive(
                "missing manifest.json in .ozpkg archive".into(),
            ));
        }

        Ok(Self {
            path,
            parsed_manifest,
            entries,
            entry_contents,
        })
    }

    /// The file path this archive was opened from.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns a reference to the parsed `manifest.json`, if it was found.
    pub fn manifest(&self) -> Option<&Value> {
        self.parsed_manifest.as_ref()
    }

    /// Returns the names of all Lua script entries in the archive.
    pub fn scripts(&self) -> Vec<&str> {
        self.entries
            .iter()
            .filter_map(|(_, e)| match e {
                OzpkEntry::Script(name) => Some(name.as_str()),
                _ => None,
            })
            .collect()
    }

    /// Returns the names of all SQL migration entries in the archive.
    pub fn migrations(&self) -> Vec<&str> {
        self.entries
            .iter()
            .filter_map(|(_, e)| match e {
                OzpkEntry::Migration(name) => Some(name.as_str()),
                _ => None,
            })
            .collect()
    }

    /// Returns all entries in insertion order.
    pub fn entries(&self) -> &[(String, OzpkEntry)] {
        &self.entries
    }

    /// The total number of entries in the archive.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` if the archive is empty (no entries at all).
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Read the raw bytes of an entry by its filename (last path component).
    ///
    /// Returns `None` if no entry matches exactly.
    pub fn read_entry(&self, filename: &str) -> Option<&[u8]> {
        // Try exact match first
        if let Some(data) = self.entry_contents.get(filename) {
            return Some(data.as_slice());
        }
        // Fall back to matching on the last path component
        let normalised = filename.replace('\\', "/");
        let target = normalised
            .rsplit('/')
            .next()
            .unwrap_or(&normalised)
            .to_string();
        for (stored_name, data) in &self.entry_contents {
            let stored_normalised = stored_name.replace('\\', "/");
            let stored_file = stored_normalised
                .rsplit('/')
                .next()
                .unwrap_or(&stored_normalised);
            if stored_file == target {
                return Some(data.as_slice());
            }
        }
        None
    }

    /// Read the raw bytes of an entry by its exact path inside the archive.
    ///
    /// Unlike `read_entry`, this does not fall back to filename matching.
    pub fn read_entry_exact(&self, exact_path: &str) -> Option<&[u8]> {
        self.entry_contents.get(exact_path).map(Vec::as_slice)
    }

    /// Extract all entries from the archive into a destination directory.
    ///
    /// Creates the destination directory if it doesn't exist. Maintains the
    /// directory structure from inside the archive for paths with `/` or `\`.
    pub fn extract_to(&self, dest: impl AsRef<Path>) -> Result<(), PluginError> {
        let dest = dest.as_ref();
        std::fs::create_dir_all(dest)?;

        for (name, data) in &self.entry_contents {
            let target_path = dest.join(name);
            if let Some(parent) = target_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&target_path, data)?;
        }

        Ok(())
    }

    /// Extract only the Lua scripts and SQL migrations (not other files) into
    /// subdirectories under `dest`.
    ///
    /// Creates `dest/scripts/` and `dest/migrations/` and writes the respective
    /// files there, flattening any directory structure.
    pub fn extract_scripts_and_migrations(
        &self,
        dest: impl AsRef<Path>,
    ) -> Result<(), PluginError> {
        let dest = dest.as_ref();

        let scripts_dir = dest.join("scripts");
        let migrations_dir = dest.join("migrations");

        for (_, entry) in &self.entries {
            match entry {
                OzpkEntry::Script(name) => {
                    if let Some(data) = self.read_entry(name) {
                        std::fs::create_dir_all(&scripts_dir)?;
                        std::fs::write(scripts_dir.join(name), data)?;
                    }
                }
                OzpkEntry::Migration(name) => {
                    if let Some(data) = self.read_entry(name) {
                        std::fs::create_dir_all(&migrations_dir)?;
                        std::fs::write(migrations_dir.join(name), data)?;
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Returns `true` if the archive contains any Lua script entries.
    pub fn has_scripts(&self) -> bool {
        self.entries.iter().any(|(_, e)| e.is_script())
    }

    /// Returns `true` if the archive contains any SQL migration entries.
    pub fn has_migrations(&self) -> bool {
        self.entries.iter().any(|(_, e)| e.is_migration())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Helper: build an in-memory `.ozpkg` zip archive from a list of
    /// (path_in_archive, content) pairs.
    fn build_ozpkg(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut buf = std::io::Cursor::new(Vec::new());
        let mut zip = zip::ZipWriter::new(&mut buf);

        for (name, data) in files {
            zip.start_file::<&str, ()>(*name, zip::write::FileOptions::default())
                .unwrap();
            zip.write_all(data).unwrap();
        }

        zip.finish().unwrap();
        buf.into_inner()
    }

    #[test]
    fn open_valid_ozpkg_with_manifest() {
        let manifest = br#"{"id": "my-plugin", "name": "My Plugin", "version": "1.0.0"}"#;
        let lua = b"-- hello.lua\nfunction run() end";
        let sql = b"CREATE TABLE test (id INTEGER);";

        let bytes = build_ozpkg(&[
            ("manifest.json", manifest),
            ("hello.lua", lua),
            ("init.sql", sql),
        ]);

        let archive = OzpkArchive::from_bytes(&bytes, "test.ozpkg").unwrap();
        assert!(archive.manifest().is_some());
        assert_eq!(archive.scripts(), vec!["hello.lua"]);
        assert_eq!(archive.migrations(), vec!["init.sql"]);
        assert_eq!(archive.len(), 3);
    }

    #[test]
    fn open_ozpkg_missing_manifest_fails() {
        let lua = b"-- orphan.lua";
        let bytes = build_ozpkg(&[("orphan.lua", lua)]);
        let result = OzpkArchive::from_bytes(&bytes, "bad.ozpkg");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("missing manifest.json"), "got: {err}");
    }

    #[test]
    fn open_ozpkg_invalid_manifest_json_fails() {
        let bytes = build_ozpkg(&[("manifest.json", b"not valid json")]);
        let result = OzpkArchive::from_bytes(&bytes, "bad.ozpkg");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("invalid manifest.json"), "got: {err}");
    }

    #[test]
    fn open_ozpkg_invalid_zip_fails() {
        let result = OzpkArchive::from_bytes(b"not a zip file at all", "bad.ozpkg");
        assert!(result.is_err());
    }

    #[test]
    fn open_ozpkg_empty_zip_with_manifest() {
        let manifest = br#"{"id": "empty", "name": "Empty", "version": "0.1.0"}"#;
        let bytes = build_ozpkg(&[("manifest.json", manifest)]);
        let archive = OzpkArchive::from_bytes(&bytes, "empty.ozpkg").unwrap();
        assert!(archive.manifest().is_some());
        assert!(archive.scripts().is_empty());
        assert!(archive.migrations().is_empty());
        assert!(!archive.has_scripts());
        assert!(!archive.has_migrations());
        assert_eq!(archive.len(), 1);
    }

    #[test]
    fn archive_with_subdirectories() {
        let manifest = br#"{"id": "subdirs", "name": "Subdirs", "version": "1.0.0"}"#;
        let lua = b"-- sub/helper.lua";
        let sql = b"CREATE TABLE x (id);";

        let bytes = build_ozpkg(&[
            ("manifest.json", manifest),
            ("scripts/helper.lua", lua),
            ("migrations/001_init.sql", sql),
        ]);

        let archive = OzpkArchive::from_bytes(&bytes, "subdirs.ozpkg").unwrap();
        // scripts() returns filenames only (last component)
        assert_eq!(archive.scripts(), vec!["helper.lua"]);
        assert_eq!(archive.migrations(), vec!["001_init.sql"]);
        assert_eq!(archive.len(), 3);

        // Can read by filename (falls back to last component)
        assert!(archive.read_entry("helper.lua").is_some());
        assert!(archive.read_entry("001_init.sql").is_some());

        // Can read by exact path
        assert!(archive.read_entry_exact("scripts/helper.lua").is_some());
    }

    #[test]
    fn read_entry_exact_vs_fallback() {
        let manifest = br#"{"id": "test", "name": "Test", "version": "1.0.0"}"#;
        let bytes = build_ozpkg(&[("manifest.json", manifest), ("scripts/foo.lua", b"-- foo")]);

        let archive = OzpkArchive::from_bytes(&bytes, "test.ozpkg").unwrap();

        // read_entry with filename works (fallback)
        assert_eq!(archive.read_entry("foo.lua"), Some(&b"-- foo"[..]));

        // read_entry with exact path works
        assert_eq!(
            archive.read_entry_exact("scripts/foo.lua"),
            Some(&b"-- foo"[..])
        );

        // read_entry_exact with just filename does NOT work (no fallback)
        assert!(archive.read_entry_exact("foo.lua").is_none());
    }

    #[test]
    fn extract_to_directory() {
        let manifest = br#"{"id": "extract", "name": "Extract", "version": "1.0.0"}"#;
        let lua = b"-- extracted.lua";
        let sql = b"CREATE TABLE t (id);";

        let bytes = build_ozpkg(&[
            ("manifest.json", manifest),
            ("extracted.lua", lua),
            ("init.sql", sql),
        ]);

        let archive = OzpkArchive::from_bytes(&bytes, "extract.ozpkg").unwrap();
        let dest = tempfile::tempdir().unwrap();
        archive.extract_to(dest.path()).unwrap();

        // All files should be written
        assert!(dest.path().join("manifest.json").exists());
        assert!(dest.path().join("extracted.lua").exists());
        assert!(dest.path().join("init.sql").exists());

        // Contents match
        assert_eq!(
            std::fs::read(dest.path().join("extracted.lua")).unwrap(),
            lua
        );
        assert_eq!(std::fs::read(dest.path().join("init.sql")).unwrap(), sql);
    }

    #[test]
    fn extract_to_with_subdirectories() {
        let manifest = br#"{"id": "sub", "name": "Sub", "version": "1.0.0"}"#;
        let bytes = build_ozpkg(&[
            ("manifest.json", manifest),
            ("scripts/a.lua", b"-- a"),
            ("migrations/b.sql", b"-- b"),
        ]);

        let archive = OzpkArchive::from_bytes(&bytes, "sub.ozpkg").unwrap();
        let dest = tempfile::tempdir().unwrap();
        archive.extract_to(dest.path()).unwrap();

        assert!(dest.path().join("manifest.json").exists());
        assert!(dest.path().join("scripts/a.lua").exists());
        assert!(dest.path().join("migrations/b.sql").exists());
    }

    #[test]
    fn extract_scripts_and_migrations_only() {
        let manifest = br#"{"id": "x", "name": "X", "version": "1.0.0"}"#;
        let lua = b"-- script.lua";
        let sql = b"CREATE TABLE t (id);";
        let extra = b"some extra data";

        let bytes = build_ozpkg(&[
            ("manifest.json", manifest),
            ("script.lua", lua),
            ("001_create.sql", sql),
            ("readme.txt", extra),
        ]);

        let archive = OzpkArchive::from_bytes(&bytes, "x.ozpkg").unwrap();
        let dest = tempfile::tempdir().unwrap();
        archive.extract_scripts_and_migrations(dest.path()).unwrap();

        // Scripts and migrations extracted
        assert!(dest.path().join("scripts/script.lua").exists());
        assert!(dest.path().join("migrations/001_create.sql").exists());

        // Other files NOT extracted
        assert!(!dest.path().join("readme.txt").exists());
        assert!(!dest.path().join("manifest.json").exists());
    }

    #[test]
    fn read_entry_nonexistent() {
        let manifest = br#"{"id": "x", "name": "X", "version": "1.0.0"}"#;
        let bytes = build_ozpkg(&[("manifest.json", manifest)]);
        let archive = OzpkArchive::from_bytes(&bytes, "x.ozpkg").unwrap();
        assert!(archive.read_entry("nonexistent.lua").is_none());
    }

    #[test]
    fn from_bytes_path_preserved() {
        let manifest = br#"{"id": "p", "name": "P", "version": "1.0.0"}"#;
        let bytes = build_ozpkg(&[("manifest.json", manifest)]);
        let archive =
            OzpkArchive::from_bytes(&bytes, PathBuf::from("/custom/path/plugin.ozpkg")).unwrap();
        assert_eq!(archive.path(), PathBuf::from("/custom/path/plugin.ozpkg"));
    }

    #[test]
    fn archive_with_multiple_scripts_and_migrations() {
        let manifest = br#"{"id": "multi", "name": "Multi", "version": "1.0.0"}"#;
        let bytes = build_ozpkg(&[
            ("manifest.json", manifest),
            ("a.lua", b"-- a"),
            ("b.lua", b"-- b"),
            ("x.sql", b"-- x"),
            ("y.sql", b"-- y"),
            ("z.sql", b"-- z"),
        ]);

        let archive = OzpkArchive::from_bytes(&bytes, "multi.ozpkg").unwrap();

        let mut scripts = archive.scripts();
        scripts.sort();
        assert_eq!(scripts, vec!["a.lua", "b.lua"]);

        let mut migrations = archive.migrations();
        migrations.sort();
        assert_eq!(migrations, vec!["x.sql", "y.sql", "z.sql"]);

        assert_eq!(archive.len(), 6);
        assert!(archive.has_scripts());
        assert!(archive.has_migrations());
    }

    #[test]
    fn other_entry_types() {
        let manifest = br#"{"id": "o", "name": "O", "version": "1.0.0"}"#;
        let bytes = build_ozpkg(&[
            ("manifest.json", manifest),
            ("data.json", br#"{"key": "value"}"#),
            ("config.yaml", b"key: value"),
            ("README.md", b"# Plugin"),
        ]);

        let archive = OzpkArchive::from_bytes(&bytes, "other.ozpkg").unwrap();
        // Scripts and migrations should be empty
        assert!(archive.scripts().is_empty());
        assert!(archive.migrations().is_empty());
        // But entries count includes manifest + other files
        assert_eq!(archive.len(), 4);
    }

    #[test]
    fn archive_is_empty_with_only_manifest() {
        let manifest = br#"{"id": "e", "name": "E", "version": "0.0.1"}"#;
        let bytes = build_ozpkg(&[("manifest.json", manifest)]);
        let archive = OzpkArchive::from_bytes(&bytes, "e.ozpkg").unwrap();
        assert!(!archive.is_empty());
        assert_eq!(archive.len(), 1);
    }

    #[test]
    fn archive_debug_output() {
        let manifest = br#"{"id": "debug-me", "name": "Debug", "version": "1.0.0"}"#;
        let bytes = build_ozpkg(&[("manifest.json", manifest)]);
        let archive = OzpkArchive::from_bytes(&bytes, "debug.ozpkg").unwrap();
        let debug = format!("{archive:?}");
        assert!(debug.contains("debug.ozpkg"), "got: {debug}");
    }

    #[test]
    fn ozpk_entry_variant_tests() {
        let manifest_val: Value =
            serde_json::from_str(r#"{"id": "t", "name": "T", "version": "1.0.0"}"#).unwrap();

        let m = OzpkEntry::Manifest(manifest_val.clone());
        let s = OzpkEntry::Script("test.lua".into());
        let mig = OzpkEntry::Migration("001.sql".into());
        let o = OzpkEntry::Other("data.txt".into());

        assert_eq!(m.filename(), "manifest.json");
        assert_eq!(s.filename(), "test.lua");
        assert_eq!(mig.filename(), "001.sql");
        assert_eq!(o.filename(), "data.txt");

        assert!(s.is_script());
        assert!(!m.is_script());
        assert!(mig.is_migration());
        assert!(!o.is_migration());

        // Debug output
        let s_debug = format!("{s:?}");
        assert!(s_debug.contains("test.lua"));
    }

    #[test]
    fn open_from_file() {
        let manifest = br#"{"id": "file-test", "name": "FileTest", "version": "1.0.0"}"#;
        let bytes = build_ozpkg(&[("manifest.json", manifest)]);

        let dir = tempfile::tempdir().unwrap();
        let ozpkg_path = dir.path().join("test.ozpkg");
        std::fs::write(&ozpkg_path, &bytes).unwrap();

        let archive = OzpkArchive::open(&ozpkg_path).unwrap();
        assert_eq!(archive.path(), ozpkg_path);
        assert!(archive.manifest().is_some());
    }

    #[test]
    fn open_nonexistent_file_fails() {
        let result = OzpkArchive::open(Path::new("/does/not/exist/plugin.ozpkg"));
        assert!(result.is_err());
    }

    #[test]
    fn extract_to_creates_dest_dir() {
        let manifest = br#"{"id": "c", "name": "C", "version": "1.0.0"}"#;
        let bytes = build_ozpkg(&[("manifest.json", manifest)]);

        let archive = OzpkArchive::from_bytes(&bytes, "c.ozpkg").unwrap();
        let dest = tempfile::tempdir().unwrap();
        let sub_dir = dest.path().join("nested/dir");
        archive.extract_to(&sub_dir).unwrap();
        assert!(sub_dir.join("manifest.json").exists());
    }

    #[test]
    fn extract_scripts_and_migrations_creates_dirs() {
        let manifest = br#"{"id": "m", "name": "M", "version": "1.0.0"}"#;
        let lua = b"-- test.lua";
        let sql = b"CREATE TABLE t (id);";

        let bytes = build_ozpkg(&[
            ("manifest.json", manifest),
            ("test.lua", lua),
            ("create.sql", sql),
        ]);

        let archive = OzpkArchive::from_bytes(&bytes, "m.ozpkg").unwrap();
        let dest = tempfile::tempdir().unwrap();
        let sub = dest.path().join("extracted");
        archive.extract_scripts_and_migrations(&sub).unwrap();

        assert!(sub.join("scripts/test.lua").exists());
        assert!(sub.join("migrations/create.sql").exists());
    }

    #[test]
    fn has_scripts_and_has_migrations_edge_cases() {
        let manifest = br#"{"id": "e", "name": "E", "version": "1.0.0"}"#;

        // Only manifest
        let bytes = build_ozpkg(&[("manifest.json", manifest)]);
        let archive = OzpkArchive::from_bytes(&bytes, "e.ozpkg").unwrap();
        assert!(!archive.has_scripts());
        assert!(!archive.has_migrations());

        // Only scripts
        let bytes = build_ozpkg(&[("manifest.json", manifest), ("s.lua", b"-- s")]);
        let archive = OzpkArchive::from_bytes(&bytes, "e.ozpkg").unwrap();
        assert!(archive.has_scripts());
        assert!(!archive.has_migrations());

        // Only migrations
        let bytes = build_ozpkg(&[("manifest.json", manifest), ("m.sql", b"-- m")]);
        let archive = OzpkArchive::from_bytes(&bytes, "e.ozpkg").unwrap();
        assert!(!archive.has_scripts());
        assert!(archive.has_migrations());
    }
}
