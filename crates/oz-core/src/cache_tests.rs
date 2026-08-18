
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
        popularity_score: 0.0,
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
