#![forbid(unsafe_code)]
#![doc = "Core library for the FurrumX data platform."]

pub mod checkpoint;
pub mod cli;
pub mod compute;
pub mod config;
pub mod control;
pub mod core;
pub mod dag;
pub mod discovery;
pub mod error;
pub mod history;
pub mod plugins;
pub mod runtime;
pub mod serving;
pub mod storage;
pub mod transport;

/// Human-readable project name.
pub const PROJECT_NAME: &str = "FurrumX";
