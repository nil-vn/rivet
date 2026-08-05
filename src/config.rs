//! Runtime configuration contracts.

/// Initial execution profiles used to select safe resource defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionProfile {
    /// Conservative defaults for 2-4 core and 4-8 GiB nodes.
    Low,
    /// Balanced defaults for a single throughput-oriented node.
    Standard,
    /// Distributed controller or executor defaults.
    Distributed,
}
