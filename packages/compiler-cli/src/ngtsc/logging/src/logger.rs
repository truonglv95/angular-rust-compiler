// Logger Interface
//
// Logger trait definition.

/// Log level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Debug = 0,
    Info = 1,
    Warn = 2,
    Error = 3,
}

/// Logger trait.
pub trait Logger {
    fn level(&self) -> LogLevel;
    fn debug(&self, msg: &str);
    fn info(&self, msg: &str);
    fn warn(&self, msg: &str);
    fn error(&self, msg: &str);
    fn is_enabled(&self, level: LogLevel) -> bool {
        level >= self.level()
    }
}

/// Null logger (logs nothing).
pub struct NullLogger;

impl NullLogger {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NullLogger {
    fn default() -> Self {
        Self::new()
    }
}

impl Logger for NullLogger {
    fn level(&self) -> LogLevel { LogLevel::Error }
    fn debug(&self, _msg: &str) {}
    fn info(&self, _msg: &str) {}
    fn warn(&self, _msg: &str) {}
    fn error(&self, _msg: &str) {}
}
