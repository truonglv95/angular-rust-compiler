
#[derive(Debug, Clone, Copy)]
pub struct Context {
    pub is_statement: bool,
}

impl Context {
    pub fn new(is_statement: bool) -> Self {
        Self { is_statement }
    }

    pub fn with_expression_mode(&self) -> Self {
        if self.is_statement {
            Self::new(false)
        } else {
            *self
        }
    }

    pub fn with_statement_mode(&self) -> Self {
        if !self.is_statement {
            Self::new(true)
        } else {
            *self
        }
    }
}
