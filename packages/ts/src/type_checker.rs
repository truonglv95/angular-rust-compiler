use crate::node::Node;
use std::fmt::Debug;

pub trait TypeChecker: Debug {
    fn get_type_at_location(&self, node: &dyn Node) -> Box<dyn Type>;
    fn get_symbol_at_location(&self, node: &dyn Node) -> Option<Box<dyn Symbol>>;
    fn type_to_string(&self, ty: &dyn Type, enclosing_declaration: Option<&dyn Node>, flags: Option<TypeFormatFlags>) -> String;
    fn symbol_to_string(&self, symbol: &dyn Symbol, enclosing_declaration: Option<&dyn Node>, meaning: Option<SymbolFlags>, flags: Option<SymbolFormatFlags>) -> String;
    
    // Add more methods as needed
    fn get_declared_type_of_symbol(&self, symbol: &dyn Symbol) -> Box<dyn Type>;
    fn get_exports_of_module(&self, symbol: &dyn Symbol) -> Vec<Box<dyn Symbol>>;
}

pub trait Symbol: Debug {
    fn name(&self) -> String;
    fn flags(&self) -> SymbolFlags;
    fn get_declarations(&self) -> Option<Vec<Box<dyn Node>>>;
}

pub trait Type: Debug {
    fn flags(&self) -> TypeFlags;
    fn symbol(&self) -> Option<Box<dyn Symbol>>;
    fn is_union(&self) -> bool;
    fn is_intersection(&self) -> bool;
    fn is_literal(&self) -> bool;
    fn is_string_literal(&self) -> bool;
    fn is_number_literal(&self) -> bool;
    fn is_class(&self) -> bool;
    fn is_interface(&self) -> bool;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeFlags(u32);

impl TypeFlags {
    pub const ANY: Self = Self(1);
    pub const UNKNOWN: Self = Self(2);
    pub const STRING: Self = Self(4);
    pub const NUMBER: Self = Self(8);
    pub const BOOLEAN: Self = Self(16);
    pub const ENUM: Self = Self(32);
    pub const BIGINT: Self = Self(64);
    pub const STRING_LITERAL: Self = Self(128);
    pub const NUMBER_LITERAL: Self = Self(256);
    pub const BOOLEAN_LITERAL: Self = Self(512);
    pub const ENUM_LITERAL: Self = Self(1024);
    pub const BIGINT_LITERAL: Self = Self(2048);
    pub const ESSYMBOL: Self = Self(4096);
    pub const UNIQUE_ESSYMBOL: Self = Self(8192);
    pub const VOID: Self = Self(16384);
    pub const UNDEFINED: Self = Self(32768);
    pub const NULL: Self = Self(65536);
    pub const NEVER: Self = Self(131072);
    pub const TYPE_PARAMETER: Self = Self(262144);
    pub const OBJECT: Self = Self(524288);
    pub const UNION: Self = Self(1048576);
    pub const INTERSECTION: Self = Self(2097152);
    pub const INDEX: Self = Self(4194304);
    pub const INDEXED_ACCESS: Self = Self(8388608);
    pub const CONDITIONAL: Self = Self(16777216);
    pub const SUBSTITUTION: Self = Self(33554432);
    pub const NON_PRIMITIVE: Self = Self(67108864);
    pub const TEMPLATE_LITERAL: Self = Self(134217728);
    pub const STRING_MAPPING: Self = Self(268435456);
    
    // Helper to check flags
    pub fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SymbolFlags(u32);

impl SymbolFlags {
    pub const NONE: Self = Self(0);
    pub const FUNCTION_SCOPED_VARIABLE: Self = Self(1);
    pub const BLOCK_SCOPED_VARIABLE: Self = Self(2);
    pub const PROPERTY: Self = Self(4);
    pub const ENUM_MEMBER: Self = Self(8);
    pub const FUNCTION: Self = Self(16);
    pub const CLASS: Self = Self(32);
    pub const INTERFACE: Self = Self(64);
    pub const CONST_ENUM: Self = Self(128);
    pub const REGULAR_ENUM: Self = Self(256);
    pub const VALUE_MODULE: Self = Self(512);
    pub const NAMESPACE_MODULE: Self = Self(1024);
    pub const TYPE_LITERAL: Self = Self(2048);
    pub const OBJECT_LITERAL: Self = Self(4096);
    pub const METHOD: Self = Self(8192);
    pub const CONSTRUCTOR: Self = Self(16384);
    pub const GET_ACCESSOR: Self = Self(32768);
    pub const SET_ACCESSOR: Self = Self(65536);
    pub const SIGNATURE: Self = Self(131072);
    pub const TYPE_PARAMETER: Self = Self(262144);
    pub const TYPE_ALIAS: Self = Self(524288);
    pub const EXPORT_VALUE: Self = Self(1048576);
    pub const ALIAS: Self = Self(2097152);
    pub const PROTOTYPE: Self = Self(4194304);
    pub const EXPORT_STAR: Self = Self(8388608);
    pub const OPTIONAL: Self = Self(16777216);
    pub const TRANSIENT: Self = Self(33554432);
    pub const ASSIGNMENT: Self = Self(67108864);
    pub const MODULE_EXPORTS: Self = Self(134217728);

    pub fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeFormatFlags(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SymbolFormatFlags(u32);
