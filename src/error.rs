//! Application error types.

use thiserror::Error;

/// Top-level typed error for bootstrap-level failures.
#[derive(Debug, Error)]
pub enum FurrumxError {
    /// Telemetry subscriber initialization failed.
    #[error("telemetry initialization failed: {message}")]
    TelemetryInitialization {
        /// Actionable initialization context.
        message: String,
    },
}
