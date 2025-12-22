// Console Logger
//
// Logger that writes to console.

use super::logger::{Logger, LogLevel};

/// Console logger.
pub struct ConsoleLogger {
    level: LogLevel,
}

impl ConsoleLogger {
    pub fn new(level: LogLevel) -> Self {
        Self { level }
    }
}

impl Logger for ConsoleLogger {
    fn level(&self) -> LogLevel {
        self.level
    }
    
    fn debug(&self, msg: &str) {
        if self.is_enabled(LogLevel::Debug) {
            eprintln!("[DEBUG] {}", msg);
        }
    }
    
    fn info(&self, msg: &str) {
        if self.is_enabled(LogLevel::Info) {
            println!("[INFO] {}", msg);
        }
    }
    
    fn warn(&self, msg: &str) {
        if self.is_enabled(LogLevel::Warn) {
            eprintln!("[WARN] {}", msg);
        }
    }
    
    fn error(&self, msg: &str) {
        if self.is_enabled(LogLevel::Error) {
            eprintln!("[ERROR] {}", msg);
        }
    }
}
