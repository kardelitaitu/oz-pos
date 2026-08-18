
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
        // ADR #36 popularity_score field (added in-flight by another
        // agent's products work — completing the initializer so the
        // crate compiles).
        popularity_score: 0.0,
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
    
