//! State machine example for Rust code analyzer.

/// State represents the possible states of the state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Idle,
    Loading,
    Loaded,
    Error,
}

/// Context for the state machine.
pub struct StateContext {
    pub state: State,
    pub data: Option<Box<dyn std::any::Any>>,
    pub error: Option<String>,
}

impl Default for StateContext {
    fn default() -> Self {
        Self {
            state: State::Idle,
            data: None,
            error: None,
        }
    }
}

/// Resource loader with explicit lifecycle methods.
pub struct ResourceLoader {
    context: StateContext,
}

impl ResourceLoader {
    /// Creates a new ResourceLoader.
    pub fn new() -> Self {
        Self {
            context: StateContext::default(),
        }
    }

    /// Initialize the loader.
    ///
    /// @lifecycle 1
    pub fn init(&mut self) {
        self.context = StateContext::default();
    }

    /// Start loading resources.
    ///
    /// @lifecycle 2
    pub fn start(&mut self) -> Result<(), &'static str> {
        if self.context.state != State::Idle {
            return Err("Can only start from Idle state");
        }
        self.context.state = State::Loading;
        Ok(())
    }

    /// Stop the loader.
    ///
    /// @lifecycle 3
    pub fn stop(&mut self) {
        self.context.state = State::Idle;
        self.context.data = None;
    }

    /// Clean up resources.
    ///
    /// @lifecycle 4
    pub fn destroy(&mut self) {
        self.stop();
    }

    /// State transition: Idle -> Loading
    pub fn load(&mut self) {
        self.context.state = State::Loading;
    }

    /// State transition: Loading -> Loaded
    pub fn on_success(&mut self, data: Box<dyn std::any::Any>) {
        self.context.state = State::Loaded;
        self.context.data = Some(data);
    }

    /// State transition: Loading -> Error
    pub fn on_error(&mut self, error: String) {
        self.context.state = State::Error;
        self.context.error = Some(error);
    }

    /// State transition: Error -> Idle (retry)
    pub fn retry(&mut self) {
        if self.context.state == State::Error {
            self.context.state = State::Idle;
            self.context.error = None;
        }
    }
}

impl Default for ResourceLoader {
    fn default() -> Self {
        Self::new()
    }
}
