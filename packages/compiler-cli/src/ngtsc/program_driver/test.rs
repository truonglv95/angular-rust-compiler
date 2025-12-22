// Program Driver Tests
//
// Tests for the program driver module.

#[cfg(test)]
mod tests {
    use crate::ngtsc::program_driver::*;
    
    mod program_tests {
        use super::*;
        
        #[test]
        fn should_create_program() {
            let program = Program::new(vec!["main.ts".to_string()]);
            
            assert_eq!(program.source_files().len(), 1);
            assert!(program.source_files().contains(&"main.ts".to_string()));
        }
        
        #[test]
        fn should_track_multiple_files() {
            let program = Program::new(vec![
                "main.ts".to_string(),
                "app.module.ts".to_string(),
                "app.component.ts".to_string(),
            ]);
            
            assert_eq!(program.source_files().len(), 3);
        }
    }
    
    mod simple_program_driver_tests {
        use super::*;
        
        #[test]
        fn should_create_driver() {
            let driver = SimpleProgramDriver::new();
            assert!(driver.get_program().is_none());
        }
        
        #[test]
        fn should_update_program() {
            let mut driver = SimpleProgramDriver::new();
            
            let program = Program::new(vec!["test.ts".to_string()]);
            driver.update_program(program);
            
            assert!(driver.get_program().is_some());
        }
        
        #[test]
        fn should_get_source_files() {
            let mut driver = SimpleProgramDriver::new();
            
            let program = Program::new(vec!["a.ts".to_string(), "b.ts".to_string()]);
            driver.update_program(program);
            
            let files = driver.get_source_files();
            assert_eq!(files.len(), 2);
        }
    }
}
