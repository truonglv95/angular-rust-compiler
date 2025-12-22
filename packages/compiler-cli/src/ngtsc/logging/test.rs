// Logging Tests
//
// Tests for the logging module.

#[cfg(test)]
mod tests {
    use crate::ngtsc::logging::*;
    
    mod log_level_tests {
        use super::*;
        
        #[test]
        fn should_have_correct_ordering() {
            assert!((LogLevel::Debug as u8) < (LogLevel::Info as u8));
            assert!((LogLevel::Info as u8) < (LogLevel::Warn as u8));
            assert!((LogLevel::Warn as u8) < (LogLevel::Error as u8));
        }
    }
    
    mod console_logger_tests {
        use super::*;
        
        #[test]
        fn should_create_with_level() {
            let logger = ConsoleLogger::new(LogLevel::Warn);
            assert_eq!(logger.level(), LogLevel::Warn);
        }
        
        #[test]
        fn should_check_level_enabled() {
            let logger = ConsoleLogger::new(LogLevel::Warn);
            
            assert!(!logger.is_enabled(LogLevel::Debug));
            assert!(!logger.is_enabled(LogLevel::Info));
            assert!(logger.is_enabled(LogLevel::Warn));
            assert!(logger.is_enabled(LogLevel::Error));
        }
    }
    
    mod null_logger_tests {
        use super::*;
        
        #[test]
        fn should_not_log_anything() {
            let logger = NullLogger::new();
            
            // These should not panic
            logger.debug("debug message");
            logger.info("info message");
            logger.warn("warn message");
            logger.error("error message");
        }
    }
}
