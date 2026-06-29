//! Linux Secret Service (libsecret / DBus) implementation of [`Keyring`].
//!
//! Talks to the `org.freedesktop.secrets` DBus service to store and
//! retrieve secrets.

use crate::error::SecurityError;
use crate::Keyring;
use std::collections::HashMap;
use tokio::runtime::Runtime;
use zbus::Connection;
use zvariant::{OwnedObjectPath, Value};

const SECRET_SERVICE: &str = "org.freedesktop.secrets";
const SECRET_PATH: &str = "/org/freedesktop/secrets";
const SECRET_IFACE: &str = "org.freedesktop.Secret.Service";
const COLLECTION_IFACE: &str = "org.freedesktop.Secret.Collection";
const ITEM_IFACE: &str = "org.freedesktop.Secret.Item";

/// Linux Secret Service keyring.
///
/// Stores secrets in the FreeDesktop Secret Service using the
/// `org.freedesktop.secrets` DBus API.
pub struct LibSecretKeyring {
    rt: Runtime,
    conn: Connection,
}

impl LibSecretKeyring {
    /// Create a new libsecret keyring instance.
    pub fn new() -> Result<Self, SecurityError> {
        let rt = Runtime::new().map_err(|e| {
            SecurityError::KeyUnavailable(format!(
                "failed to create tokio runtime: {e}"
            ))
        })?;
        let conn = rt.block_on(Connection::session()).map_err(|e| {
            SecurityError::KeyUnavailable(format!(
                "failed to connect to session D-Bus: {e}"
            ))
        })?;
        Ok(Self { rt, conn })
    }

    async fn open_session(&self) -> Result<OwnedObjectPath, SecurityError> {
        let result: (Value<'_>, OwnedObjectPath) = self
            .conn
            .call_method(
                Some(SECRET_SERVICE),
                SECRET_PATH,
                Some(SECRET_IFACE),
                "OpenSession",
                &("plain", Value::new("")),
            )
            .await
            .map_err(|e| {
                SecurityError::KeyUnavailable(format!(
                    "OpenSession failed: {e}"
                ))
            })?
            .body()
            .map_err(|e| {
                SecurityError::KeyUnavailable(format!(
                    "OpenSession body deserialize failed: {e}"
                ))
            })?;
        Ok(result.1)
    }

    async fn default_collection(&self) -> Result<OwnedObjectPath, SecurityError> {
        let path: OwnedObjectPath = self
            .conn
            .call_method(
                Some(SECRET_SERVICE),
                SECRET_PATH,
                Some(SECRET_IFACE),
                "ReadAlias",
                &("default",),
            )
            .await
            .map_err(|e| {
                SecurityError::KeyUnavailable(format!(
                    "ReadAlias failed: {e}"
                ))
            })?
            .body()
            .map_err(|e| {
                SecurityError::KeyUnavailable(format!(
                    "ReadAlias body deserialize failed: {e}"
                ))
            })?;
        Ok(path)
    }

    async fn search_items(
        &self,
        attrs: &HashMap<String, String>,
    ) -> Result<Vec<OwnedObjectPath>, SecurityError> {
        let (unlocked, _locked): (Vec<OwnedObjectPath>, Vec<OwnedObjectPath>) = self
            .conn
            .call_method(
                Some(SECRET_SERVICE),
                SECRET_PATH,
                Some(SECRET_IFACE),
                "SearchItems",
                &(attrs,),
            )
            .await
            .map_err(|e| {
                SecurityError::KeyUnavailable(format!(
                    "SearchItems failed: {e}"
                ))
            })?
            .body()
            .map_err(|e| {
                SecurityError::KeyUnavailable(format!(
                    "SearchItems body deserialize failed: {e}"
                ))
            })?;
        Ok(unlocked)
    }
}

impl Keyring for LibSecretKeyring {
    fn get_secret(&self, name: &str) -> Result<Option<String>, SecurityError> {
        let attrs = attributes(name);
        let items = self.rt.block_on(self.search_items(&attrs))?;

        if items.is_empty() {
            return Ok(None);
        }

        let session = self.rt.block_on(self.open_session())?;

        let secret: (OwnedObjectPath, Vec<u8>, String) = self
            .rt
            .block_on(
                self.conn.call_method(
                    Some(SECRET_SERVICE),
                    &items[0],
                    Some(ITEM_IFACE),
                    "GetSecret",
                    &(&session,),
                ),
            )
            .map_err(|e| {
                SecurityError::KeyUnavailable(format!(
                    "GetSecret failed: {e}"
                ))
            })?
            .body()
            .map_err(|e| {
                SecurityError::KeyUnavailable(format!(
                    "GetSecret body deserialize failed: {e}"
                ))
            })?;

        let s = String::from_utf8(secret.1).map_err(|e| {
            SecurityError::KeyUnavailable(format!(
                "secret is not valid UTF-8: {e}"
            ))
        })?;

        Ok(Some(s))
    }

    fn set_secret(&self, name: &str, value: &str) -> Result<(), SecurityError> {
        let attrs = attributes(name);

        let session = self.rt.block_on(self.open_session())?;
        let collection = self.rt.block_on(self.default_collection())?;

        let items = self.rt.block_on(self.search_items(&attrs))?;
        for item_path in &items {
            let _: Result<(), _> = self
                .rt
                .block_on(
                    self.conn.call_method::<(), _, ()>(
                        Some(SECRET_SERVICE),
                        item_path,
                        Some(ITEM_IFACE),
                        "Delete",
                        &(),
                    ),
                )
                .map_err(|e| {
                    SecurityError::KeyUnavailable(format!(
                        "Delete failed: {e}"
                    ))
                });
        }

        let properties = HashMap::from([
            (
                "org.freedesktop.Secret.Item.Label".to_string(),
                Value::new(name.to_string()),
            ),
            (
                "org.freedesktop.Secret.Item.Attributes".to_string(),
                Value::new(attrs.clone()),
            ),
        ]);

        let secret: (OwnedObjectPath, Vec<u8>, String) =
            (session, value.as_bytes().to_vec(), "text/plain".to_string());

        let (_item, _created): (OwnedObjectPath, bool) = self
            .rt
            .block_on(
                self.conn.call_method(
                    Some(SECRET_SERVICE),
                    &collection,
                    Some(COLLECTION_IFACE),
                    "CreateItem",
                    &(properties, secret, true),
                ),
            )
            .map_err(|e| {
                SecurityError::KeyUnavailable(format!(
                    "CreateItem failed: {e}"
                ))
            })?
            .body()
            .map_err(|e| {
                SecurityError::KeyUnavailable(format!(
                    "CreateItem body deserialize failed: {e}"
                ))
            })?;

        Ok(())
    }

    fn delete_secret(&self, name: &str) -> Result<bool, SecurityError> {
        let attrs = attributes(name);
        let items = self.rt.block_on(self.search_items(&attrs))?;

        if items.is_empty() {
            return Ok(false);
        }

        for item_path in &items {
            self.rt
                .block_on(
                    self.conn.call_method::<(), _, ()>(
                        Some(SECRET_SERVICE),
                        item_path,
                        Some(ITEM_IFACE),
                        "Delete",
                        &(),
                    ),
                )
                .map_err(|e| {
                    SecurityError::KeyUnavailable(format!(
                        "Delete failed: {e}"
                    ))
                })?;
        }

        Ok(true)
    }
}

fn attributes(name: &str) -> HashMap<String, String> {
    HashMap::from([
        ("application".to_string(), "oz-pos".to_string()),
        ("oz-pos-name".to_string(), name.to_string()),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_keyring() -> LibSecretKeyring {
        LibSecretKeyring::new().expect("failed to create keyring")
    }

    #[test]
    fn linux_roundtrip() {
        let k = test_keyring();
        let name = "oz-pos-test-linux-roundtrip";
        let _ = k.delete_secret(name);

        assert_eq!(k.get_secret(name).unwrap(), None);

        k.set_secret(name, "linux-secret-42").unwrap();
        assert_eq!(
            k.get_secret(name).unwrap(),
            Some("linux-secret-42".into())
        );

        assert!(k.delete_secret(name).unwrap());
        assert_eq!(k.get_secret(name).unwrap(), None);
    }

    #[test]
    fn linux_delete_nonexistent_returns_false() {
        let k = test_keyring();
        assert!(!k
            .delete_secret("oz-pos-test-nonexistent-linux")
            .unwrap());
    }

    #[test]
    fn linux_overwrite_existing() {
        let k = test_keyring();
        let name = "oz-pos-test-overwrite-linux";
        let _ = k.delete_secret(name);

        k.set_secret(name, "original").unwrap();
        k.set_secret(name, "replacement").unwrap();
        assert_eq!(
            k.get_secret(name).unwrap(),
            Some("replacement".into())
        );

        k.delete_secret(name).unwrap();
    }
}
