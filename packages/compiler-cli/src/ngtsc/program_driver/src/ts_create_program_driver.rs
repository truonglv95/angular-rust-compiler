// TypeScript Create Program Driver
//
// Driver for creating TypeScript programs.

use super::api::{Program, ProgramDriver};

/// TypeScript program driver.
#[derive(Default)]
pub struct TsCreateProgramDriver {
    program: Option<Program>,
    root_files: Vec<String>,
}

impl TsCreateProgramDriver {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn set_root_files(&mut self, files: Vec<String>) {
        self.root_files = files;
    }
    
    pub fn create_program(&mut self) {
        self.program = Some(Program::new(self.root_files.clone()));
    }
}

impl ProgramDriver for TsCreateProgramDriver {
    fn get_program(&self) -> Option<&Program> {
        self.program.as_ref()
    }
    
    fn update_program(&mut self, program: Program) {
        self.program = Some(program);
    }
    
    fn get_source_files(&self) -> Vec<String> {
        self.program
            .as_ref()
            .map(|p| p.source_files().to_vec())
            .unwrap_or_default()
    }
}
