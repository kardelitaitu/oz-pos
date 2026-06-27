# oz-payment

Payment processor abstraction for OZ-POS. A single trait with vendor-specific implementations for Stripe, Square, and EMV terminals. The cashier's flow uses the trait; switching processors is a config change, not a code change.

## Public API

- [`PaymentError`](src/error.rs) — `thiserror`-based error for processor calls (declined, timeout, network, invalid response).

## Planned surface

- `PaymentProcessor` trait with `authorize`, `capture`, `void`, `refund`.
- Adapters for Stripe (`stripe-rust`), Square (`square-rust-sdk`), and IDTech/ViVOtech EMV terminals.
- A `MockProcessor` for tests, following the same pattern as `oz-hal`'s mandatory mocks.
- Idempotency keys for every `authorize` and `capture` to make retries safe.

## Status

Scaffold only. The trait and adapters land in a follow-up alongside the `oz-hal` `PaymentTerminal` driver.
