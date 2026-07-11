//! Shared visitor for formatting `tracing` event fields into plain text.
//!
//! [`MessageVisitor`] collects all fields from a `tracing::Event` into
//! a plain-text string suitable for output channels that don't support
//! structured data (syslog, Windows Event Log).

/// Collects event fields into a plain-text message string.
///
/// The `message` field is formatted without quotes. Other fields are
/// appended as `name=value` pairs.
pub struct MessageVisitor<'a>(
    /// Output buffer — formatted event fields are appended into this string.
    pub &'a mut String,
);

impl<'a> tracing::field::Visit for MessageVisitor<'a> {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.0.push_str(value);
        } else {
            self.0.push_str(&format!(" {}={}", field.name(), value));
        }
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            // Use debug formatting for the message field.
            self.0.push_str(&format!("{:?}", value));
        } else {
            self.0.push_str(&format!(" {}={:?}", field.name(), value));
        }
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.0.push_str(&format!(" {}={}", field.name(), value));
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.0.push_str(&format!(" {}={}", field.name(), value));
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.0.push_str(&format!(" {}={}", field.name(), value));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tracing::{Level, event};
    use tracing_subscriber::layer::{Context, Layer};
    use tracing_subscriber::prelude::*;

    /// A custom layer that intercepts events and feeds them to our MessageVisitor.
    struct CaptureLayer(Arc<Mutex<String>>);

    impl<S: tracing::Subscriber> Layer<S> for CaptureLayer {
        fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
            let mut buf = self.0.lock().unwrap();
            let mut visitor = MessageVisitor(&mut buf);
            event.record(&mut visitor);
        }
    }

    /// Run a closure with the capture layer and return the formatted string.
    fn capture_event(f: impl FnOnce()) -> String {
        let buf = Arc::new(Mutex::new(String::new()));
        let layer = CaptureLayer(buf.clone());
        let subscriber = tracing_subscriber::registry().with(layer);

        tracing::subscriber::with_default(subscriber, f);

        let result = buf.lock().unwrap();
        result.clone()
    }

    #[test]
    fn message_field_without_prefix() {
        let output = capture_event(|| {
            event!(Level::INFO, message = "hello world");
        });
        assert_eq!(output, "hello world");
    }

    #[test]
    fn str_fields_use_name_equals_value() {
        let output = capture_event(|| {
            event!(Level::INFO, status = "ok");
        });
        assert_eq!(output, " status=ok");
    }

    #[test]
    fn multiple_fields_are_appended() {
        let output = capture_event(|| {
            event!(
                Level::INFO,
                message = "transaction complete",
                id = 42_u64,
                success = true,
            );
        });
        assert!(output.contains("transaction complete"));
        assert!(output.contains(" id=42"));
        assert!(output.contains(" success=true"));
    }

    #[test]
    fn record_i64() {
        let output = capture_event(|| {
            event!(Level::INFO, delta = -15_i64);
        });
        assert_eq!(output, " delta=-15");
    }

    #[test]
    fn record_u64() {
        let output = capture_event(|| {
            event!(Level::INFO, count = 100_u64);
        });
        assert_eq!(output, " count=100");
    }

    #[test]
    fn record_bool_true() {
        let output = capture_event(|| {
            event!(Level::INFO, verbose = true);
        });
        assert_eq!(output, " verbose=true");
    }

    #[test]
    fn record_bool_false() {
        let output = capture_event(|| {
            event!(Level::INFO, verbose = false);
        });
        assert_eq!(output, " verbose=false");
    }

    #[test]
    fn debug_uses_debug_repr() {
        let output = capture_event(|| {
            event!(Level::INFO, debug_info = ?("quoted"));
        });
        assert_eq!(output, " debug_info=\"quoted\"");
    }
}
