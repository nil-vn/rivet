//! Runtime and telemetry bootstrap.

use tracing_subscriber::EnvFilter;

use crate::error::FurrumxError;

/// Initializes structured tracing from `RUST_LOG`, defaulting to `info`.
pub fn init_telemetry() -> Result<(), FurrumxError> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .try_init()
        .map_err(|error| FurrumxError::TelemetryInitialization {
            message: error.to_string(),
        })
}
