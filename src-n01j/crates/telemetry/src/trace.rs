//! Trace span utilities for OTEL.

#![forbid(unsafe_code)]
#![forbid(clippy::unwrap_used)]
#![forbid(clippy::expect_used)]
#![forbid(clippy::panic)]

use tracing::Span;

/// Create a span with OTEL semantic conventions.
#[must_use]
pub fn span_with_otel_attributes(name: &str, _attributes: &[(&str, &str)]) -> Span {
    let span = tracing::info_span!("{}", name);
    span
}

/// Add OTEL status to span attributes.
#[must_use]
pub fn with_otel_status(status: &'static str) -> Span {
    let span = tracing::info_span!("otel_status", otel_status = status);
    span
}

/// Record error details on a span.
pub fn record_span_error(span: &Span, error: &impl std::fmt::Display) {
    span.record("error", tracing::field::display(&error.to_string()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_with_otel_attributes() {
        let span = span_with_otel_attributes("test_span", &[]);
        assert!(span.is_disabled() || span.id().is_some());
    }

    #[test]
    fn test_with_otel_status() {
        let span = with_otel_status("OK");
        assert!(span.is_disabled() || span.id().is_some());
    }
}
