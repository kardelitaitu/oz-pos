//! Shared test fixtures for oz-payment integration tests.

use oz_payment::drivers::stripe::StripePaymentProcessor;

/// Default test secret key for Stripe processor.
pub const TEST_SECRET_KEY: &str = "sk_test_shared_fixture";

/// Creates a StripePaymentProcessor pointing at the given endpoint.
pub fn stripe_processor(uri: &str, card_present: bool) -> StripePaymentProcessor {
    StripePaymentProcessor::new_with_endpoint(TEST_SECRET_KEY, uri, card_present)
}
