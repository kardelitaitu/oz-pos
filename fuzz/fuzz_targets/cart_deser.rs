//! Fuzz target for Cart + Sale JSON deserialization — feeds arbitrary
//! JSON byte sequences to `serde_json::from_str` and verifies
//! no panics during deserialization.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Attempt to deserialize a Cart from arbitrary JSON.
        // This must never panic — only return Err or Ok.
        let _result: Result<foundation::cart::Cart, _> = serde_json::from_str(s);

        // Also try Sale deserialization (oz-core domain type).
        let _sale: Result<oz_core::Sale, _> = serde_json::from_str(s);
    }
});
