//! Compilation Module
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/compilation.ts
//! Contains the core compilation structures for template pipeline

use crate::constant_pool::ConstantPool;
use crate::core::ChangeDetectionStrategy;
use crate::directive_matching::{CssSelector, SelectorMatcher};
use crate::output::output_ast::Expression;
use crate::parse_util::ParseError;
use crate::render3::view::api::{R3ComponentDeferMetadata, R3TemplateDependencyMetadata};
use crate::schema::dom_element_schema_registry::DomElementSchemaRegistry;
use crate::template::pipeline::ir;
use std::collections::HashSet;

/// The kind of compilation job
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilationJobKind {
    /// Template compilation
    Tmpl,
    /// Host binding compilation
    Host,
    /// A special value used to indicate that some logic applies to both compilation types
    Both,
}

/// Possible modes in which a component's template can be compiled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateCompilationMode {
    /// Supports the full instruction set, including directives.
    Full,
    /// Uses a narrower instruction set that doesn't support directives and allows optimizations.
    DomOnly,
}

/// An entire ongoing compilation, which will result in one or more template functions when complete.
/// Contains one or more corresponding compilation units.
pub trait CompilationJob {
    /// Get the component name
    fn component_name(&self) -> &str;
    /// Get the constant pool
    fn pool(&self) -> &ConstantPool;
    /// Get the constant pool (mutable)
    fn pool_mut(&mut self) -> &mut ConstantPool;
    /// Get the compatibility mode
    fn compatibility(&self) -> ir::CompatibilityMode;
    /// Get the compilation mode
    fn mode(&self) -> TemplateCompilationMode;
    /// Get the job kind
    fn kind(&self) -> CompilationJobKind;
    /// Get the function suffix
    fn fn_suffix(&self) -> &str;
    /// Get the root compilation unit
    fn root(&self) -> &dyn CompilationUnit;
    /// Get all compilation units
    fn units(&self) -> Box<dyn Iterator<Item = &dyn CompilationUnit> + '_>;
    /// Get all compilation units (mutable)
    fn units_mut(&mut self) -> Box<dyn Iterator<Item = &mut dyn CompilationUnit> + '_>;
    /// Get the root compilation unit (mutable)
    fn root_mut(&mut self) -> &mut dyn CompilationUnit;
    /// Allocate a new XrefId
    /// Allocate a new XrefId
    fn allocate_xref_id(&mut self) -> ir::XrefId;
    /// Get the root unit's xref ID
    fn root_xref(&self) -> ir::XrefId;
    /// Get as Any
    fn as_any(&self) -> &dyn std::any::Any;
    /// Get as Any mut
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;

    /// Add a pipe to the job and return its slot handle and xref
    fn add_pipe(
        &mut self,
        name: &str,
        view_xref: ir::XrefId,
    ) -> (ir::handle::SlotHandle, ir::XrefId);
    /// Mark a pipe as used
    fn mark_pipe_used(&mut self, name: &str);
    /// Mark a directive as used by its selector or tag name
    fn mark_directive_used(&mut self, tag_or_selector: &str);
}

/// Compilation-in-progress of a whole component's template, including the main template and any
/// embedded views or host bindings.
pub struct ComponentCompilationJob {
    pub component_name: String,
    pub pool: ConstantPool,
    pub compatibility: ir::CompatibilityMode,
    pub mode: TemplateCompilationMode,
    pub relative_context_file_path: String,
    pub i18n_use_external_ids: bool,
    pub defer_meta: R3ComponentDeferMetadata,
    pub all_deferrable_deps_fn: Option<Expression>,
    pub relative_template_path: Option<String>,
    pub enable_debug_locations: bool,
    pub change_detection: Option<ChangeDetectionStrategy>,

    pub root: ViewCompilationUnit,
    pub views: indexmap::IndexMap<ir::XrefId, ViewCompilationUnit>,
    pub content_selectors: Option<Expression>,
    pub consts: Vec<Expression>,
    pub consts_initializers: Vec<Expression>,

    pub available_dependencies: Vec<R3TemplateDependencyMetadata>,
    pub used_dependencies: HashSet<usize>,
    pub selector_matcher: SelectorMatcher<usize>,
    pub schema_registry: DomElementSchemaRegistry,
    pub diagnostics: Vec<ParseError>,

    pub next_xref_id: ir::XrefId,
    pub temp_selector: CssSelector,
}

impl ComponentCompilationJob {
    pub fn new(
        component_name: String,
        pool: ConstantPool,
        compatibility: ir::CompatibilityMode,
        mode: TemplateCompilationMode,
        relative_context_file_path: String,
        i18n_use_external_ids: bool,
        defer_meta: R3ComponentDeferMetadata,
        all_deferrable_deps_fn: Option<Expression>,
        relative_template_path: Option<String>,
        enable_debug_locations: bool,
        change_detection: Option<ChangeDetectionStrategy>,
        available_dependencies: Vec<R3TemplateDependencyMetadata>,
    ) -> Self {
        let root_xref = ir::XrefId::new(0);
        let root = ViewCompilationUnit::new(root_xref, None);

        let views = indexmap::IndexMap::new();
        // Note: In TypeScript, root is stored in views as well
        // In Rust, we store it separately in the root field
        // If needed, we could use Rc<RefCell<ViewCompilationUnit>> to share ownership

        let selector_matcher = Self::create_selector_matcher(&available_dependencies);
        let schema_registry = DomElementSchemaRegistry::new();

        ComponentCompilationJob {
            component_name,
            pool,
            compatibility,
            mode,
            relative_context_file_path,
            i18n_use_external_ids,
            defer_meta,
            all_deferrable_deps_fn,
            relative_template_path,
            enable_debug_locations,
            change_detection,
            root,
            views,
            content_selectors: None,
            consts: Vec::new(),
            consts_initializers: Vec::new(),
            available_dependencies,
            used_dependencies: HashSet::new(),
            selector_matcher,
            schema_registry,
            diagnostics: Vec::new(),
            next_xref_id: ir::XrefId::new(1),
            temp_selector: CssSelector::new(),
        }
    }

    fn create_selector_matcher(deps: &Vec<R3TemplateDependencyMetadata>) -> SelectorMatcher<usize> {
        let mut matcher = SelectorMatcher::new();
        for (i, dep) in deps.iter().enumerate() {
            if let R3TemplateDependencyMetadata::Directive(dir) = dep {
                match CssSelector::parse(&dir.selector) {
                    Ok(selectors) => {
                        for selector in selectors {
                            matcher.add_selectable(selector, i);
                        }
                    }
                    Err(_) => {}
                }
            }
        }
        matcher
    }

    /// Add a `ViewCompilationUnit` for a new embedded view to this compilation.
    pub fn allocate_view(&mut self, parent: Option<ir::XrefId>) -> ir::XrefId {
        let xref = self.allocate_xref_id();
        let view = ViewCompilationUnit::new(xref, parent);
        self.views.insert(xref, view);
        xref
    }

    /// Add a constant `Expression` to the compilation and return its index in the `consts` array.
    pub fn add_const(
        &mut self,
        new_const: Expression,
        initializers: Option<Vec<Expression>>,
    ) -> ir::ConstIndex {
        // Check for equivalent constants
        for (idx, existing) in self.consts.iter().enumerate() {
            if self.expressions_equivalent(existing, &new_const) {
                return ir::ConstIndex::new(idx);
            }
        }

        let idx = self.consts.len();
        self.consts.push(new_const);
        if let Some(init) = initializers {
            self.consts_initializers.extend(init);
        }
        ir::ConstIndex::new(idx)
    }

    /// Check if two expressions are equivalent
    fn expressions_equivalent(&self, a: &Expression, b: &Expression) -> bool {
        a.is_equivalent(b)
    }
}

impl CompilationJob for ComponentCompilationJob {
    fn pool_mut(&mut self) -> &mut ConstantPool {
        &mut self.pool
    }
    fn root_xref(&self) -> ir::XrefId {
        self.root.xref
    }
    fn component_name(&self) -> &str {
        &self.component_name
    }

    fn pool(&self) -> &ConstantPool {
        &self.pool
    }

    fn compatibility(&self) -> ir::CompatibilityMode {
        self.compatibility
    }

    fn mode(&self) -> TemplateCompilationMode {
        self.mode
    }

    fn kind(&self) -> CompilationJobKind {
        CompilationJobKind::Tmpl
    }

    fn fn_suffix(&self) -> &str {
        "Template"
    }

    fn root(&self) -> &dyn CompilationUnit {
        &self.root
    }

    fn units(&self) -> Box<dyn Iterator<Item = &dyn CompilationUnit> + '_> {
        let root = &self.root as &dyn CompilationUnit;
        let views = self.views.values().map(|v| v as &dyn CompilationUnit);
        Box::new(std::iter::once(root).chain(views))
    }

    fn units_mut(&mut self) -> Box<dyn Iterator<Item = &mut dyn CompilationUnit> + '_> {
        Box::new(
            std::iter::once(&mut self.root as &mut dyn CompilationUnit).chain(
                self.views
                    .values_mut()
                    .map(|v| v as &mut dyn CompilationUnit),
            ),
        )
    }

    fn root_mut(&mut self) -> &mut dyn CompilationUnit {
        &mut self.root
    }

    fn allocate_xref_id(&mut self) -> ir::XrefId {
        let id = self.next_xref_id;
        self.next_xref_id = ir::XrefId::new(id.as_usize() + 1);
        id
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn add_pipe(
        &mut self,
        name: &str,
        view_xref: ir::XrefId,
    ) -> (ir::handle::SlotHandle, ir::XrefId) {
        // First check if pipe exists (immutably)
        {
            let unit = if view_xref == self.root.xref {
                &self.root
            } else {
                self.views.get(&view_xref).expect("View not found")
            };
            if let Some((slot, xref)) = unit.pipes.get(name) {
                return (slot.clone(), *xref);
            }
        }

        // Create new pipe
        let slot = ir::handle::SlotHandle::new();
        let xref = self.allocate_xref_id();

        // Re-borrow mutably to insert
        let unit = if view_xref == self.root.xref {
            &mut self.root
        } else {
            self.views.get_mut(&view_xref).expect("View not found")
        };

        unit.pipes.insert(name.to_string(), (slot.clone(), xref));

        let op = ir::ops::create::PipeOp::new(xref, slot.clone(), name.into());
        unit.create.push(Box::new(op));

        // Also mark as used
        self.mark_pipe_used(name);

        (slot, xref)
    }

    fn mark_pipe_used(&mut self, name: &str) {
        for (i, dep) in self.available_dependencies.iter().enumerate() {
            if let R3TemplateDependencyMetadata::Pipe(pipe) = dep {
                if pipe.name == name {
                    self.used_dependencies.insert(i);
                }
            }
        }
    }

    fn mark_directive_used(&mut self, name: &str) {
        for (i, dep) in self.available_dependencies.iter().enumerate() {
            if let R3TemplateDependencyMetadata::Directive(dir) = dep {
                if dir.selector == name {
                    self.used_dependencies.insert(i);
                } else {
                    // Also check if name is the component class name
                    let type_name = match &dir.type_ {
                        Expression::ReadVar(rv) => Some(&rv.name),
                        _ => None,
                    };
                    if let Some(tn) = type_name {
                        if tn == name {
                            self.used_dependencies.insert(i);
                        }
                    }
                }
            }
        }
    }
}

/// A compilation unit is compiled into a template function. Some example units are views and host
/// bindings.
pub trait CompilationUnit {
    /// Get the xref ID
    fn xref(&self) -> ir::XrefId;
    /// Get the job this unit belongs to
    fn job(&self) -> &dyn CompilationJob;
    /// Get the function name
    fn fn_name(&self) -> Option<&str>;
    /// Set the function name
    fn set_fn_name(&mut self, name: String);
    /// Get the number of variable slots
    fn vars(&self) -> Option<usize>;
    /// Set the number of variable slots
    fn set_vars(&mut self, vars: usize);
    /// Get the create operations list
    fn create(&self) -> &ir::OpList<Box<dyn ir::CreateOp + Send + Sync>>;
    /// Get the create operations list (mutable)
    fn create_mut(&mut self) -> &mut ir::OpList<Box<dyn ir::CreateOp + Send + Sync>>;
    /// Get the update operations list
    fn update(&self) -> &ir::OpList<Box<dyn ir::UpdateOp + Send + Sync>>;
    /// Get the update operations list (mutable)
    fn update_mut(&mut self) -> &mut ir::OpList<Box<dyn ir::UpdateOp + Send + Sync>>;

    /// Iterate over all operations within this view.
    ///
    /// Some operations may have child operations (like ListenerOp with handlerOps,
    /// or RepeaterCreateOp with trackByOps), which this iterator will visit.
    ///
    /// Returns an iterator that yields references to operations.
    /// Note: This is a simplified implementation that currently only iterates
    /// over create and update ops directly, without visiting child ops.
    /// Full implementation would need to handle child ops as well.
    fn ops<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn ir::Op> + 'a>;
}

/// Compilation-in-progress of an individual view within a template.
pub struct ViewCompilationUnit {
    pub xref: ir::XrefId,
    pub parent: Option<ir::XrefId>,
    pub context_variables: indexmap::IndexMap<String, String>,
    pub aliases: Vec<ir::AliasVariable>,
    pub decls: Option<usize>,

    pub fn_name: Option<String>,
    pub vars: Option<usize>,
    pub create: ir::OpList<Box<dyn ir::CreateOp + Send + Sync>>,
    pub update: ir::OpList<Box<dyn ir::UpdateOp + Send + Sync>>,
    pub pipes: indexmap::IndexMap<String, (ir::handle::SlotHandle, ir::XrefId)>,
}

impl ViewCompilationUnit {
    pub fn new(xref: ir::XrefId, parent: Option<ir::XrefId>) -> Self {
        ViewCompilationUnit {
            xref,
            parent,
            context_variables: indexmap::IndexMap::new(),
            aliases: Vec::new(),
            decls: None,
            fn_name: None,
            vars: None,
            create: ir::OpList::new(),
            update: ir::OpList::new(),
            pipes: indexmap::IndexMap::new(),
        }
    }
}

impl CompilationUnit for ViewCompilationUnit {
    fn xref(&self) -> ir::XrefId {
        self.xref
    }

    fn job(&self) -> &dyn CompilationJob {
        // TODO: Need to return reference to parent job
        // This requires storing a reference or using Rc/Arc for shared ownership
        // For now, we use todo!() to indicate this needs to be implemented
        todo!("Need to implement job reference - requires refactoring to store job reference")
    }

    fn fn_name(&self) -> Option<&str> {
        self.fn_name.as_deref()
    }

    fn set_fn_name(&mut self, name: String) {
        self.fn_name = Some(name);
    }

    fn vars(&self) -> Option<usize> {
        self.vars
    }

    fn set_vars(&mut self, vars: usize) {
        self.vars = Some(vars);
    }

    fn create(&self) -> &ir::OpList<Box<dyn ir::CreateOp + Send + Sync>> {
        &self.create
    }

    fn create_mut(&mut self) -> &mut ir::OpList<Box<dyn ir::CreateOp + Send + Sync>> {
        &mut self.create
    }

    fn update(&self) -> &ir::OpList<Box<dyn ir::UpdateOp + Send + Sync>> {
        &self.update
    }

    fn update_mut(&mut self) -> &mut ir::OpList<Box<dyn ir::UpdateOp + Send + Sync>> {
        &mut self.update
    }

    fn ops<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn ir::Op> + 'a> {
        // Create an iterator that yields both CreateOp and UpdateOp as Op
        // This is a simplified version - full implementation would also visit child ops
        Box::new(
            self.create()
                .iter()
                .map(|op| op.as_ref() as &dyn ir::Op)
                .chain(self.update().iter().map(|op| op.as_ref() as &dyn ir::Op)),
        )
    }
}

/// Compilation-in-progress of a host binding, which contains a single unit for that host binding.
pub struct HostBindingCompilationJob {
    pub component_name: String,
    pub pool: ConstantPool,
    pub compatibility: ir::CompatibilityMode,
    pub mode: TemplateCompilationMode,
    pub root: HostBindingCompilationUnit,
}

impl HostBindingCompilationJob {
    pub fn new(
        component_name: String,
        pool: ConstantPool,
        compatibility: ir::CompatibilityMode,
        mode: TemplateCompilationMode,
    ) -> Self {
        let root = HostBindingCompilationUnit::new();
        HostBindingCompilationJob {
            component_name,
            pool,
            compatibility,
            mode,
            root,
        }
    }
}

impl CompilationJob for HostBindingCompilationJob {
    fn pool_mut(&mut self) -> &mut ConstantPool {
        &mut self.pool
    }
    fn root_xref(&self) -> ir::XrefId {
        self.root.xref
    }
    fn component_name(&self) -> &str {
        &self.component_name
    }

    fn pool(&self) -> &ConstantPool {
        &self.pool
    }

    fn compatibility(&self) -> ir::CompatibilityMode {
        self.compatibility
    }

    fn mode(&self) -> TemplateCompilationMode {
        self.mode
    }

    fn kind(&self) -> CompilationJobKind {
        CompilationJobKind::Host
    }

    fn fn_suffix(&self) -> &str {
        "HostBindings"
    }

    fn root(&self) -> &dyn CompilationUnit {
        &self.root
    }

    fn units(&self) -> Box<dyn Iterator<Item = &dyn CompilationUnit> + '_> {
        // HostBinding (job.root) is a single unit
        let root = &self.root as &dyn CompilationUnit;
        Box::new(std::iter::once(root))
    }

    fn units_mut(&mut self) -> Box<dyn Iterator<Item = &mut dyn CompilationUnit> + '_> {
        Box::new(std::iter::once(&mut self.root as &mut dyn CompilationUnit))
    }

    fn root_mut(&mut self) -> &mut dyn CompilationUnit {
        &mut self.root
    }

    fn allocate_xref_id(&mut self) -> ir::XrefId {
        // Host bindings don't need XrefIds in the same way
        ir::XrefId::new(0)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn add_pipe(
        &mut self,
        name: &str,
        _view_xref: ir::XrefId,
    ) -> (ir::handle::SlotHandle, ir::XrefId) {
        let unit = &mut self.root;
        if let Some((slot, xref)) = unit.pipes.get(name) {
            (slot.clone(), *xref)
        } else {
            let slot = ir::handle::SlotHandle::new();
            // Host binding job xref handling
            let xref = ir::XrefId::new(0); // Dummy xref as before
            unit.pipes.insert(name.to_string(), (slot.clone(), xref));

            let op = ir::ops::create::PipeOp::new(xref, slot.clone(), name.into());
            unit.create.push(Box::new(op));

            (slot, xref)
        }
    }

    fn mark_pipe_used(&mut self, _name: &str) {}
    fn mark_directive_used(&mut self, _name: &str) {}
}

pub struct HostBindingCompilationUnit {
    pub xref: ir::XrefId,
    pub attributes: Option<Expression>,
    pub fn_name: Option<String>,
    pub vars: Option<usize>,
    pub create: ir::OpList<Box<dyn ir::CreateOp + Send + Sync>>,
    pub update: ir::OpList<Box<dyn ir::UpdateOp + Send + Sync>>,
    pub pipes: indexmap::IndexMap<String, (ir::handle::SlotHandle, ir::XrefId)>,
}

impl HostBindingCompilationUnit {
    pub fn new() -> Self {
        HostBindingCompilationUnit {
            xref: ir::XrefId::new(0),
            attributes: None,
            fn_name: None,
            vars: None,
            create: ir::OpList::new(),
            update: ir::OpList::new(),
            pipes: indexmap::IndexMap::new(),
        }
    }
}

impl CompilationUnit for HostBindingCompilationUnit {
    fn xref(&self) -> ir::XrefId {
        self.xref
    }

    fn job(&self) -> &dyn CompilationJob {
        // TODO: Need to return reference to parent job
        // This requires storing a reference or using Rc/Arc for shared ownership
        todo!("Need to implement job reference - requires refactoring to store job reference")
    }

    fn fn_name(&self) -> Option<&str> {
        self.fn_name.as_deref()
    }

    fn set_fn_name(&mut self, name: String) {
        self.fn_name = Some(name);
    }

    fn vars(&self) -> Option<usize> {
        self.vars
    }

    fn set_vars(&mut self, vars: usize) {
        self.vars = Some(vars);
    }

    fn create(&self) -> &ir::OpList<Box<dyn ir::CreateOp + Send + Sync>> {
        &self.create
    }

    fn create_mut(&mut self) -> &mut ir::OpList<Box<dyn ir::CreateOp + Send + Sync>> {
        &mut self.create
    }

    fn update(&self) -> &ir::OpList<Box<dyn ir::UpdateOp + Send + Sync>> {
        &self.update
    }

    fn update_mut(&mut self) -> &mut ir::OpList<Box<dyn ir::UpdateOp + Send + Sync>> {
        &mut self.update
    }

    fn ops<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn ir::Op> + 'a> {
        // Create an iterator that yields both CreateOp and UpdateOp as Op
        // This is a simplified version - full implementation would also visit child ops
        Box::new(
            self.create()
                .iter()
                .map(|op| op.as_ref() as &dyn ir::Op)
                .chain(self.update().iter().map(|op| op.as_ref() as &dyn ir::Op)),
        )
    }
}
