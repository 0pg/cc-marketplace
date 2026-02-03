//! ADT-based state machine for Rust code analyzer.

/// State with associated data (ADT pattern).
pub enum State {
    /// Initial idle state.
    Idle,
    /// Loading with progress percentage.
    Loading { progress: u32 },
    /// Loaded with data.
    Loaded(String),
    /// Error with message.
    Error(String),
}

/// Event enum with mixed variants.
pub enum Event {
    Start,
    DataReceived { data: String },
    Progress(u32),
    Cancel,
}
