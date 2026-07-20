//! Fuzz target for Cart JSON deserialization — feeds arbitrary JSON
//! byte sequences to `serde_json::from_str::<Cart>()` and verifies
//! no panics during deserialization.

#![no_main]

use libfuzzer_sys::fuzz_target;
use foundation::cart::Cart;
use foundation::money::Currency;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Attempt to deserialize a Cart from arbitrary JSON.
        // This must never panic — only return Err or Ok.
        let _result: Result<Cart, _> = serde_json::from_str(s);

        // Also try PaymentRequest deserialization.
        let _pr: Result<oz_core::PaymentRequest, _> = serde_json::from_str(s);
    }
});
