use std::collections::HashMap;
use crate::linker::ast::{AstHost, AstNode};

#[derive(Clone)]
pub struct AstValue<'a, TExpression: AstNode> {
    pub node: TExpression,
    pub host: &'a dyn AstHost<TExpression>,
}

impl<'a, TExpression: AstNode> AstValue<'a, TExpression> {
    pub fn new(node: TExpression, host: &'a dyn AstHost<TExpression>) -> Self {
        Self { node, host }
    }

    pub fn is_string(&self) -> bool {
        self.host.is_string_literal(&self.node)
    }

    pub fn get_string(&self) -> Result<String, String> {
        self.host.parse_string_literal(&self.node)
    }

    pub fn is_number(&self) -> bool {
        self.host.is_numeric_literal(&self.node)
    }

    pub fn get_number(&self) -> Result<f64, String> {
        self.host.parse_numeric_literal(&self.node)
    }

    pub fn is_boolean(&self) -> bool {
        self.host.is_boolean_literal(&self.node)
    }

    pub fn get_boolean(&self) -> Result<bool, String> {
        self.host.parse_boolean_literal(&self.node)
    }

    pub fn is_array(&self) -> bool {
        self.host.is_array_literal(&self.node)
    }

    pub fn get_array(&self) -> Result<Vec<AstValue<'a, TExpression>>, String> {
        let items = self.host.parse_array_literal(&self.node)?;
        Ok(items.into_iter().map(|n| AstValue::new(n, self.host)).collect())
    }

    pub fn is_object(&self) -> bool {
        self.host.is_object_literal(&self.node)
    }

    pub fn get_object(&self) -> Result<AstObject<'a, TExpression>, String> {
        if !self.is_object() {
            return Err("Expected object literal".to_string());
        }
        let map = self.host.parse_object_literal(&self.node)?;
        Ok(AstObject { map, host: self.host })
    }
    
    pub fn is_null(&self) -> bool {
        self.host.is_null(&self.node)
    }

    pub fn print(&self) -> String {
        self.host.print_node(&self.node)
    }
}

pub struct AstObject<'a, TExpression: AstNode> {
    map: HashMap<String, TExpression>,
    pub host: &'a dyn AstHost<TExpression>,
}

impl<'a, TExpression: AstNode> AstObject<'a, TExpression> {
    pub fn has(&self, key: &str) -> bool {
        self.map.contains_key(key)
    }

    pub fn get_value(&self, key: &str) -> Result<AstValue<'a, TExpression>, String> {
        if let Some(node) = self.map.get(key) {
            Ok(AstValue::new(node.clone(), self.host))
        } else {
            Err(format!("Property '{}' not found", key))
        }
    }

    pub fn get_string(&self, key: &str) -> Result<String, String> {
        let val = self.get_value(key)?;
        val.get_string()
    }
    
    pub fn get_bool(&self, key: &str) -> Result<bool, String> {
        let val = self.get_value(key)?;
        val.get_boolean()
    }
    
    pub fn get_number(&self, key: &str) -> Result<f64, String> {
        let val = self.get_value(key)?;
        val.get_number()
    }

    pub fn get_array(&self, key: &str) -> Result<Vec<AstValue<'a, TExpression>>, String> {
        let val = self.get_value(key)?;
        val.get_array()
    }

    pub fn get_object(&self, key: &str) -> Result<AstObject<'a, TExpression>, String> {
        let val = self.get_value(key)?;
        val.get_object()
    }
    
    /// Returns the raw map for advanced usage
    pub fn to_map(&self) -> &HashMap<String, TExpression> {
        &self.map
    }
}
