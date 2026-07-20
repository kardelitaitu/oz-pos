//! Fuzz target for SKU parsing — feeds arbitrary byte sequences to
//! `Sku::new()` and verifies no panics, no invalid states.

#![no_main]

use libfuzzer_sys::fuzz_target;
use foundation::sku::Sku;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Sku::new should never panic, regardless of input.
        let sku = Sku::new(s);

        // SKU length must be ≤ 100 characters.
        assert!(sku.as_str().len() <= 100, "SKU exceeds max length");

        // SKU must not contain control characters.
        let valid = sku.as_str().chars().all(|c| !c.is_control());
        assert!(valid, "SKU contains control character");
    }
});
