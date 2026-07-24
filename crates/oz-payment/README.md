<!-- Audit stamp: 2026-07-22 · Hermes-Agent · status: ACCURATE (2 noted findings, doc-staleness) · F1: "35 unit tests pass" -> actual 122 #[test]/#[tokio::test] in crates/oz-payment/src · F2: "Next: real adapters (Stripe, Square, EMV terminal)" -> Stripe, Square, AND QRIS adapters already exist (drivers/stripe.rs, square.rs, qris.rs); EMV terminal not present · verified accurate: PaymentProcessor trait in processor.rs:37 with authorize/capture/refund/void/sale lifecycle; MockPaymentProcessor in drivers/mock.rs -->

# oz-payment

Payment processor abstraction for OZ-POS.

## Status

✅ `PaymentProcessor` trait defined with the full lifecycle:
`authorize → capture → refund → void`. Includes a `sale()`
default implementation (authorize + capture in one call).

✅ `MockPaymentProcessor` — programmable test double with call
counters, one-shot decline/timeout simulation. 35 unit tests pass.

Next: real adapters (Stripe, Square, EMV terminal).

> last audited 30-06-26 by docs-auditor
