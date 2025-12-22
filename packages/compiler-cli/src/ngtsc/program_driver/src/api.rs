// Program Driver API
//
// Program driver interface definitions.

/// Program representation.
#[derive(Debug, Clone)]
pub struct Program {
    source_files: Vec<String>,
}

impl Program {
    pub fn new(files: Vec<String>) -> Self {
        Self { source_files: files }
    }
    
    pub fn source_files(&self) -> &[String] {
        &self.source_files
    }
}

/// Program driver trait.
pub trait ProgramDriver {
    fn get_program(&self) -> Option<&Program>;
    fn update_program(&mut self, program: Program);
    fn get_source_files(&self) -> Vec<String>;
}

/// Simple program driver.
#[derive(Default)]
pub struct SimpleProgramDriver {
    program: Option<Program>,
}

impl SimpleProgramDriver {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ProgramDriver for SimpleProgramDriver {
    fn get_program(&self) -> Option<&Program> {
        self.program.as_ref()
    }
    
    fn update_program(&mut self, program: Program) {
        self.program = Some(program);
    }
    
    fn get_source_files(&self) -> Vec<String> {
        self.program
            .as_ref()
            .map(|p| p.source_files.clone())
            .unwrap_or_default()
    }
}
