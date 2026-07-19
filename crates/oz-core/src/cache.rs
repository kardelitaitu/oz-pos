//! Caching layer for frequently-accessed POS data.
//!
//! Provides a [`Cache`] trait, a [`NoopCache`] fallback, and an optional
//! [`RedisCache`] implementation behind the `cache-redis` feature flag.

use std::sync::Arc;

use crate::db::ProductWithDetails;

/// A key-value cache for frequently-accessed POS data.
pub trait Cache: Send + Sync {
    /// Get a cached product by SKU. Returns `None` on miss.
    fn get_product(&self, sku: &str) -> Option<ProductWithDetails>;
    /// Set a cached product with the configured TTL.
    fn set_product(&self, sku: &str, product: &ProductWithDetails);
    /// Invalidate a cached product.
    fn invalidate_product(&self, sku: &str);

    /// Get cached inventory quantity for a product.
    fn get_inventory(&self, product_id: &str) -> Option<i64>;
    /// Set a cached inventory quantity.
    fn set_inventory(&self, product_id: &str, qty: i64);
    /// Invalidate cached inventory for a product.
    fn invalidate_inventory(&self, product_id: &str);

    /// Returns `true` when the cache backend is connected and healthy.
    fn is_healthy(&self) -> bool;

    /// Start a background listener for inventory change notifications.
    ///
    /// `terminal_id` identifies this terminal for pub/sub filtering —
    /// the subscriber will skip messages tagged with its own terminal_id.
    /// Pass `None` if terminal identity is unknown (all messages will
    /// be processed).
    ///
    /// Returns a shutdown sender that can be used to stop the listener.
    /// Returns `None` when the backend does not support pub/sub (e.g.
    /// no-op cache). The `_cache` Arc is passed through so the spawned
    /// thread can hold a reference to the cache for invalidation.
    fn start_inventory_pubsub(
        &self,
        _cache: Arc<dyn Cache>,
        _terminal_id: Option<String>,
    ) -> Option<std::sync::mpsc::Sender<()>> {
        let _ = (_cache, _terminal_id);
        None
    }

    /// Publish an inventory change notification.
    ///
    /// `terminal_id` identifies this terminal so other subscribers can
    /// skip their own messages. Called after stock adjustments.
    /// Default impl is a no-op; `RedisCache` overrides this to publish
    /// to `inventory:updates`.
    fn publish_inventory_change(
        &self,
        _product_id: &str,
        _sku: &str,
        _new_qty: i64,
        _terminal_id: Option<&str>,
    ) {
    }

    /// Publish a `stock.negative` warning event (ADR-18 §4).
    ///
    /// Called when `allow_negative_stock` is enabled on a location and
    /// a stock adjustment results in negative quantity. The payload
    /// includes `{ product_id, sku, location_id, delta, current_qty, terminal_id, timestamp }`.
    /// Default impl is a no-op; `RedisCache` overrides this to publish
    /// to `stock:negative`.
    fn publish_negative_stock_event(
        &self,
        _product_id: &str,
        _sku: &str,
        _location_id: &str,
        _delta: i64,
        _current_qty: i64,
        _terminal_id: Option<&str>,
    ) {
    }
}

/// No-op cache that always misses.
///
/// Used as the default fallback when Redis is unavailable or the
/// `cache-redis` feature is disabled.
pub struct NoopCache;

impl Cache for NoopCache {
    fn get_product(&self, _sku: &str) -> Option<ProductWithDetails> {
        None
    }
    fn set_product(&self, _sku: &str, _product: &ProductWithDetails) {}
    fn invalidate_product(&self, _sku: &str) {}
    fn get_inventory(&self, _product_id: &str) -> Option<i64> {
        None
    }
    fn set_inventory(&self, _product_id: &str, _qty: i64) {}
    fn invalidate_inventory(&self, _product_id: &str) {}
    fn is_healthy(&self) -> bool {
        false
    }
    fn start_inventory_pubsub(
        &self,
        _cache: Arc<dyn Cache>,
        _terminal_id: Option<String>,
    ) -> Option<std::sync::mpsc::Sender<()>> {
        let _ = (_cache, _terminal_id);
        None
    }

    fn publish_negative_stock_event(
        &self,
        _product_id: &str,
        _sku: &str,
        _location_id: &str,
        _delta: i64,
        _current_qty: i64,
        _terminal_id: Option<&str>,
    ) {
    }
}

/// Redis-backed cache implementation.
///
/// Connects to a Redis instance and stores values as JSON strings with
/// a configurable TTL. Only available when the `cache-redis` feature is
/// enabled.
#[cfg(feature = "cache-redis")]
pub mod redis_cache {
    use std::sync::{Arc, Mutex};

    use super::Cache;
    use crate::db::ProductWithDetails;

    /// Redis-backed cache.
    pub struct RedisCache {
        #[allow(dead_code)]
        client: redis::Client,
        conn: Mutex<redis::Connection>,
        ttl_seconds: u64,
    }

    impl RedisCache {
        /// Connect to a Redis instance at the given URL.
        ///
        /// # Errors
        ///
        /// Returns a `RedisError` when the URL is invalid or the
        /// connection cannot be established.
        pub fn connect(url: &str, ttl_seconds: u64) -> Result<Self, redis::RedisError> {
            let client = redis::Client::open(url)?;
            let conn = client.get_connection()?;
            Ok(Self {
                client,
                conn: Mutex::new(conn),
                ttl_seconds,
            })
        }

        /// Subscribe to the inventory change channel and invalidate local cache
        /// entries when remote updates arrive.
        ///
        /// Spawns a background task that listens on `inventory:updates` and
        /// calls `invalidate_inventory` for each received notification.
        /// The returned `std::sync::mpsc::Sender` can be used to stop the
        /// subscription (drop it or send a value).
        ///
        /// A read timeout of 5 seconds is set on the underlying TCP connection
        /// so the shutdown signal is checked at least every 5 seconds even when
        /// no messages are being published.
        fn subscribe_inventory_changes(
            client: redis::Client,
            cache: Arc<dyn Cache>,
            terminal_id: Option<String>,
        ) -> Result<std::sync::mpsc::Sender<()>, redis::RedisError> {
            let (tx, rx) = std::sync::mpsc::channel::<()>();

            // Spawn a blocking task since `redis` crate connections are synchronous.
            // `redis::Client` is `Clone` (wraps an Arc internally), so we can cheaply
            // share it with the spawned thread.
            std::thread::spawn(move || {
                // Connect with a 5-second timeout so the shutdown signal can be
                // checked regularly even when no messages arrive.
                let mut conn =
                    match client.get_connection_with_timeout(std::time::Duration::from_secs(5)) {
                        Ok(c) => c,
                        Err(e) => {
                            tracing::error!(
                                error = %e,
                                "failed to connect for inventory pub/sub"
                            );
                            return;
                        }
                    };

                // Set read timeout on the TCP stream so `get_message()` unblocks.
                let _ = conn.set_read_timeout(Some(std::time::Duration::from_secs(5)));

                let mut pubsub = conn.as_pubsub();

                if let Err(e) = pubsub.subscribe("inventory:updates") {
                    tracing::error!(error = %e, "failed to subscribe to inventory:updates");
                    return;
                }

                tracing::info!("subscribed to inventory:updates channel");

                let own_id = terminal_id.unwrap_or_default();
                loop {
                    // Check if we should stop (non-blocking check).
                    if rx.try_recv().is_ok() {
                        tracing::info!("inventory pub/sub shutting down");
                        let _ = pubsub.unsubscribe("inventory:updates");
                        return;
                    }

                    match pubsub.get_message() {
                        Ok(msg) => {
                            let payload: String = msg.get_payload().unwrap_or_default();
                            if let Ok(notification) =
                                serde_json::from_str::<serde_json::Value>(&payload)
                            {
                                let msg_terminal_id =
                                    notification["terminal_id"].as_str().unwrap_or("");
                                // Skip own messages.
                                if own_id == msg_terminal_id {
                                    continue;
                                }
                                if let Some(pid) = notification["product_id"].as_str() {
                                    cache.invalidate_inventory(pid);
                                    tracing::debug!(
                                        product_id = pid,
                                        "invalidated inventory cache from pub/sub"
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            // Timeouts are expected when no messages arrive.
                            // Other errors (connection lost) terminate the loop.
                            let err_str = e.to_string();
                            if err_str.contains("timed out") || err_str.contains("timeout") {
                                continue;
                            }
                            tracing::warn!(error = %e, "inventory pub/sub error");
                            break;
                        }
                    }
                }
            });

            Ok(tx)
        }
    }

    impl Cache for RedisCache {
        fn get_product(&self, sku: &str) -> Option<ProductWithDetails> {
            let key = format!("product:{sku}");
            let mut conn = self.conn.lock().ok()?;
            let data: Option<String> = redis::cmd("GET").arg(&key).query(&mut *conn).ok()?;
            data.and_then(|s| serde_json::from_str(&s).ok())
        }

        fn set_product(&self, sku: &str, product: &ProductWithDetails) {
            let key = format!("product:{sku}");
            let Ok(data) = serde_json::to_string(product) else {
                return;
            };
            let Ok(mut conn) = self.conn.lock() else {
                return;
            };
            let _: Result<(), _> = redis::cmd("SETEX")
                .arg(&key)
                .arg(self.ttl_seconds)
                .arg(&data)
                .query(&mut *conn);
        }

        fn invalidate_product(&self, sku: &str) {
            let key = format!("product:{sku}");
            let Ok(mut conn) = self.conn.lock() else {
                return;
            };
            let _: Result<(), _> = redis::cmd("DEL").arg(&key).query(&mut *conn);
        }

        fn get_inventory(&self, product_id: &str) -> Option<i64> {
            let key = format!("inventory:{product_id}");
            let mut conn = self.conn.lock().ok()?;
            redis::cmd("GET").arg(&key).query(&mut *conn).ok()
        }

        fn set_inventory(&self, product_id: &str, qty: i64) {
            let key = format!("inventory:{product_id}");
            let Ok(mut conn) = self.conn.lock() else {
                return;
            };
            let _: Result<(), _> = redis::cmd("SETEX")
                .arg(&key)
                .arg(self.ttl_seconds)
                .arg(qty)
                .query(&mut *conn);
        }

        fn invalidate_inventory(&self, product_id: &str) {
            let key = format!("inventory:{product_id}");
            let Ok(mut conn) = self.conn.lock() else {
                return;
            };
            let _: Result<(), _> = redis::cmd("DEL").arg(&key).query(&mut *conn);
        }

        fn is_healthy(&self) -> bool {
            let Ok(mut conn) = self.conn.lock() else {
                return false;
            };
            redis::cmd("PING").query::<String>(&mut *conn).is_ok()
        }

        fn start_inventory_pubsub(
            &self,
            cache: Arc<dyn Cache>,
            terminal_id: Option<String>,
        ) -> Option<std::sync::mpsc::Sender<()>> {
            let client = self.client.clone();
            match Self::subscribe_inventory_changes(client, cache, terminal_id) {
                Ok(tx) => Some(tx),
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "failed to start inventory pub/sub subscription"
                    );
                    None
                }
            }
        }

        fn publish_inventory_change(
            &self,
            product_id: &str,
            sku: &str,
            new_qty: i64,
            terminal_id: Option<&str>,
        ) {
            let key = "inventory:updates";
            let payload = serde_json::json!({
                "product_id": product_id,
                "sku": sku,
                "new_qty": new_qty,
                "terminal_id": terminal_id.unwrap_or(""),
                "timestamp": chrono::Utc::now().to_rfc3339_opts(
                    chrono::SecondsFormat::Millis, true,
                ),
            });
            let Ok(msg) = serde_json::to_string(&payload) else {
                return;
            };
            let Ok(mut conn) = self.conn.lock() else {
                return;
            };
            let _: Result<(), _> = redis::cmd("PUBLISH").arg(key).arg(&msg).query(&mut *conn);
        }

        fn publish_negative_stock_event(
            &self,
            product_id: &str,
            sku: &str,
            location_id: &str,
            delta: i64,
            current_qty: i64,
            terminal_id: Option<&str>,
        ) {
            let key = "stock:negative";
            let payload = serde_json::json!({
                "product_id": product_id,
                "sku": sku,
                "location_id": location_id,
                "delta": delta,
                "current_qty": current_qty,
                "terminal_id": terminal_id.unwrap_or(""),
                "timestamp": chrono::Utc::now().to_rfc3339_opts(
                    chrono::SecondsFormat::Millis, true,
                ),
            });
            let Ok(msg) = serde_json::to_string(&payload) else {
                return;
            };
            let Ok(mut conn) = self.conn.lock() else {
                return;
            };
            let _: Result<(), _> = redis::cmd("PUBLISH").arg(key).arg(&msg).query(&mut *conn);
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::Money;

        fn usd() -> crate::Currency {
            "USD".parse().unwrap()
        }

        fn product_with_details() -> ProductWithDetails {
            ProductWithDetails {
                product: crate::Product::new(
                    "TEST-SKU",
                    "Test Product",
                    Money {
                        minor_units: 1000,
                        currency: usd(),
                    },
                ),
                category_name: Some("Test Category".into()),
                stock_qty: Some(42),
            }
        }

        #[test]
        #[ignore = "requires a Redis server on localhost:6379"]
        fn redis_cache_set_get_product() {
            let cache = RedisCache::connect("redis://127.0.0.1/", 300).unwrap();
            let p = product_with_details();
            cache.set_product("TEST-SKU", &p);
            let cached = cache.get_product("TEST-SKU").unwrap();
            assert_eq!(cached, p);
        }

        #[test]
        #[ignore = "requires a Redis server on localhost:6379"]
        fn redis_cache_invalidate_product() {
            let cache = RedisCache::connect("redis://127.0.0.1/", 300).unwrap();
            let p = product_with_details();
            cache.set_product("TEST-INV", &p);
            assert!(cache.get_product("TEST-INV").is_some());
            cache.invalidate_product("TEST-INV");
            assert!(cache.get_product("TEST-INV").is_none());
        }

        #[test]
        #[ignore = "requires a Redis server on localhost:6379"]
        fn redis_cache_set_get_inventory() {
            let cache = RedisCache::connect("redis://127.0.0.1/", 300).unwrap();
            cache.set_inventory("prod-1", 50);
            assert_eq!(cache.get_inventory("prod-1"), Some(50));
        }

        #[test]
        #[ignore = "requires a Redis server on localhost:6379"]
        fn redis_cache_invalidate_inventory() {
            let cache = RedisCache::connect("redis://127.0.0.1/", 300).unwrap();
            cache.set_inventory("prod-inv", 10);
            assert_eq!(cache.get_inventory("prod-inv"), Some(10));
            cache.invalidate_inventory("prod-inv");
            assert!(cache.get_inventory("prod-inv").is_none());
        }

        #[test]
        #[ignore = "requires a Redis server on localhost:6379"]
        fn redis_cache_is_healthy() {
            let cache = RedisCache::connect("redis://127.0.0.1/", 300).unwrap();
            assert!(cache.is_healthy());
        }
    }
}

/// Create a cache, attempting Redis first and falling back to no-op.
///
/// When the `cache-redis` feature is enabled, tries to connect to the
/// given Redis URL. On success, returns a [`RedisCache`]; on failure
/// logs a warning and returns [`NoopCache`]. When the feature is
/// disabled, always returns [`NoopCache`].
#[cfg_attr(not(feature = "cache-redis"), allow(unused_variables))]
pub fn create_cache(redis_url: &str, ttl_seconds: u64) -> Arc<dyn Cache> {
    #[cfg(feature = "cache-redis")]
    {
        match redis_cache::RedisCache::connect(redis_url, ttl_seconds) {
            Ok(cache) => return Arc::new(cache),
            Err(e) => tracing::warn!(error = %e, "Redis unavailable, using noop cache"),
        }
    }
    Arc::new(NoopCache)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Money;

    fn usd() -> crate::Currency {
        "USD".parse().unwrap()
    }

    #[test]
    fn noop_cache_get_product_returns_none() {
        let cache = NoopCache;
        assert!(cache.get_product("anything").is_none());
    }

    #[test]
    fn noop_cache_get_inventory_returns_none() {
        let cache = NoopCache;
        assert!(cache.get_inventory("any-id").is_none());
    }

    #[test]
    fn noop_cache_set_product_is_noop() {
        let cache = NoopCache;
        let p = ProductWithDetails {
            product: crate::Product::new(
                "SKU",
                "N",
                Money {
                    minor_units: 100,
                    currency: usd(),
                },
            ),
            category_name: None,
            stock_qty: None,
        };
        cache.set_product("sku", &p);
        assert!(cache.get_product("sku").is_none());
    }

    #[test]
    fn noop_cache_set_inventory_is_noop() {
        let cache = NoopCache;
        cache.set_inventory("p", 50);
        assert!(cache.get_inventory("p").is_none());
    }

    #[test]
    fn noop_cache_invalidation_is_noop() {
        let cache = NoopCache;
        cache.invalidate_product("sku");
        cache.invalidate_inventory("p");
    }

    #[test]
    fn noop_cache_is_not_healthy() {
        let cache = NoopCache;
        assert!(!cache.is_healthy());
    }

    #[test]
    fn noop_cache_start_inventory_pubsub_returns_none() {
        let cache = NoopCache;
        let arc_cache: Arc<dyn Cache> = Arc::new(NoopCache);
        assert!(cache.start_inventory_pubsub(arc_cache, None).is_none());
    }

    #[test]
    fn create_cache_falls_back_to_noop() {
        let cache = create_cache("redis://127.0.0.1:1/", 300);
        assert!(!cache.is_healthy());
    }
}
