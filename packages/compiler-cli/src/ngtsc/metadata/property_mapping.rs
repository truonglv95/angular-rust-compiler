
use indexmap::IndexMap;

/// The name of a class property that backs an input or output declared by a directive or component.
pub type ClassPropertyName = String;

/// The name by which an input or output of a directive or component is bound in an Angular template.
pub type BindingPropertyName = String;

/// An input or output of a directive that has both a named JavaScript class property on a component
/// or directive class, as well as an Angular template property name used for binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputOrOutput {
    /// The name of the JavaScript property on the component or directive instance for this input or
    /// output.
    pub class_property_name: ClassPropertyName,

    /// The property name used to bind this input or output in an Angular template.
    pub binding_property_name: BindingPropertyName,

    /// Whether the input or output is signal based.
    pub is_signal: bool,
}

/// A mapping of component property and template binding property names, for example containing the
/// inputs of a particular directive or component.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClassPropertyMapping {
    /// Mapping from class property names to the single `InputOrOutput` for that class property.
    forward_map: IndexMap<ClassPropertyName, InputOrOutput>,
}

impl ClassPropertyMapping {
    pub fn new() -> Self {
        Self {
            forward_map: IndexMap::new(),
        }
    }

    pub fn from_map(map: IndexMap<String, String>) -> Self {
        let mut forward_map = IndexMap::new();
        // IndexMap preserves insertion order - no sorting needed
        for (key, binding_prop) in map {
            forward_map.insert(
                key.clone(),
                InputOrOutput {
                    class_property_name: key,
                    binding_property_name: binding_prop,
                    is_signal: false,
                },
            );
        }
        Self { forward_map }
    }

    pub fn insert(&mut self, item: InputOrOutput) {
        self.forward_map.insert(item.class_property_name.clone(), item);
    }
    
    pub fn get_by_class_property_name(&self, name: &str) -> Option<&InputOrOutput> {
         self.forward_map.get(name)
    }

    pub fn iter(&self) -> indexmap::map::Iter<ClassPropertyName, InputOrOutput> {
        self.forward_map.iter()
    }

    pub fn len(&self) -> usize {
        self.forward_map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.forward_map.is_empty()
    }
}
