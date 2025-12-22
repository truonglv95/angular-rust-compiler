// Update Declaration
//
// Update component declarations for HMR.

/// Update declaration result.
#[derive(Debug, Clone)]
pub struct UpdateResult {
    pub updated_source: String,
    pub success: bool,
}

/// Update a component declaration for HMR.
pub fn update_declaration(_source: &str, _component: &str) -> UpdateResult {
    UpdateResult {
        updated_source: String::new(),
        success: true,
    }
}
