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
