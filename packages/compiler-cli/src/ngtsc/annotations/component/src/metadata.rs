// Component Metadata
//
// Metadata structures for component analysis.

/// R3 Component metadata for compilation.
#[derive(Debug, Clone)]
pub struct R3ComponentMetadata {
    /// Component name.
    pub name: String,
    /// Selector.
    pub selector: Option<String>,
    /// Export as names.
    pub export_as: Option<Vec<String>>,
    /// Inputs.
    pub inputs: Vec<ComponentInput>,
    /// Outputs.
    pub outputs: Vec<ComponentOutput>,
    /// Host bindings.
    pub host: ComponentHostBindings,
    /// Type reference.
    pub type_ref: String,
    /// Internal type reference.
    pub internal_type: String,
    /// Whether standalone.
    pub is_standalone: bool,
    /// View encapsulation.
    pub encapsulation: ViewEncapsulation,
    /// Template info.
    pub template: ComponentTemplateInfo,
    /// Styles.
    pub styles: Vec<String>,
    /// Style URLs.
    pub style_urls: Vec<String>,
    /// Animations.
    pub animations: Option<String>,
    /// Change detection strategy.
    pub change_detection: ChangeDetectionStrategy,
    /// Whether a signal component.
    pub is_signal: bool,
    /// Used directives.
    pub used_directives: Vec<String>,
    /// Used pipes.
    pub used_pipes: Vec<String>,
    /// Deferred blocks.
    pub defer_blocks: Vec<DeferredBlock>,
}

impl R3ComponentMetadata {
    pub fn new(name: impl Into<String>) -> Self {
        let n = name.into();
        Self {
            name: n.clone(),
            selector: None,
            export_as: None,
            inputs: Vec::new(),
            outputs: Vec::new(),
            host: ComponentHostBindings::default(),
            type_ref: n.clone(),
            internal_type: n,
            is_standalone: true,
            encapsulation: ViewEncapsulation::Emulated,
            template: ComponentTemplateInfo::default(),
            styles: Vec::new(),
            style_urls: Vec::new(),
            animations: None,
            change_detection: ChangeDetectionStrategy::Default,
            is_signal: false,
            used_directives: Vec::new(),
            used_pipes: Vec::new(),
            defer_blocks: Vec::new(),
        }
    }
    
    pub fn with_selector(mut self, selector: impl Into<String>) -> Self {
        self.selector = Some(selector.into());
        self
    }
    
    pub fn with_template(mut self, template: impl Into<String>) -> Self {
        self.template.content = Some(template.into());
        self
    }
}

/// Component input.
#[derive(Debug, Clone)]
pub struct ComponentInput {
    pub class_property_name: String,
    pub binding_property_name: String,
    pub required: bool,
    pub is_signal: bool,
    pub transform: Option<String>,
}

/// Component output.
#[derive(Debug, Clone)]
pub struct ComponentOutput {
    pub class_property_name: String,
    pub binding_property_name: String,
}

/// Host bindings for a component.
#[derive(Debug, Clone, Default)]
pub struct ComponentHostBindings {
    pub properties: Vec<(String, String)>,
    pub attributes: Vec<(String, String)>,
    pub listeners: Vec<(String, String)>,
    pub class_attr: Option<String>,
    pub style_attr: Option<String>,
}

/// Template info for component.
#[derive(Debug, Clone, Default)]
pub struct ComponentTemplateInfo {
    /// Template content (for inline).
    pub content: Option<String>,
    /// Template URL (for external).
    pub url: Option<String>,
    /// Whether preserve whitespace.
    pub preserve_whitespaces: bool,
    /// Interpolation config.
    pub interpolation: Option<(String, String)>,
}

/// View encapsulation modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ViewEncapsulation {
    #[default]
    Emulated = 0,
    None = 2,
    ShadowDom = 3,
}

/// Change detection strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChangeDetectionStrategy {
    #[default]
    Default = 0,
    OnPush = 1,
}

/// Deferred block metadata.
#[derive(Debug, Clone)]
pub struct DeferredBlock {
    /// Block name.
    pub name: String,
    /// Trigger type.
    pub trigger: DeferTrigger,
    /// Dependencies.
    pub dependencies: Vec<String>,
}

/// Defer trigger type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeferTrigger {
    Idle,
    Immediate,
    Timer,
    Viewport,
    Interaction,
    Hover,
    Prefetch,
}

impl Default for DeferTrigger {
    fn default() -> Self {
        DeferTrigger::Idle
    }
}
