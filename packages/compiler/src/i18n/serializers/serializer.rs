//! Serializer Module
//!
//! Corresponds to packages/compiler/src/i18n/serializers/serializer.ts
//! Base traits and implementations for i18n serializers

use crate::i18n::i18n_ast::{self as i18n, Message, Node, Visitor, RecurseVisitor};
use crate::i18n::translation_bundle::LoadResult;
use std::collections::HashMap;

/// Base trait for i18n serializers
pub trait Serializer {
    /// Serialize messages to string format
    /// - The `placeholders` and `placeholderToMessage` properties are irrelevant in the input messages
    /// - The `id` contains the message id that the serializer is expected to use
    /// - Placeholder names are already map to public names using the provided mapper
    fn write(&self, messages: &[Message], locale: Option<&str>) -> String;

    /// Load messages from serialized content
    fn load(&self, content: &str, url: &str) -> LoadResult;

    /// Compute digest for a message
    fn digest(&self, message: &Message) -> String;

    /// Creates a name mapper, see `PlaceholderMapper`
    /// Returning `None` means that no name mapping is used.
    fn create_name_mapper(&self, message: &Message) -> Option<Box<dyn PlaceholderMapper>> {
        None
    }
}

/// A `PlaceholderMapper` converts placeholder names from internal to serialized representation
/// and back.
///
/// It should be used for serialization format that put constraints on the placeholder names.
pub trait PlaceholderMapper {
    fn to_public_name(&self, internal_name: &str) -> Option<String>;
    fn to_internal_name(&self, public_name: &str) -> Option<String>;
}

/// A simple mapper that takes a function to transform an internal name to a public name
pub struct SimplePlaceholderMapper {
    internal_to_public: HashMap<String, String>,
    public_to_next_id: HashMap<String, usize>,
    public_to_internal: HashMap<String, String>,
}

impl SimplePlaceholderMapper {
    pub fn new<F>(message: &Message, map_name: F) -> Self
    where
        F: Fn(&str) -> String,
    {
        let mut mapper = SimplePlaceholderMapper {
            internal_to_public: HashMap::new(),
            public_to_next_id: HashMap::new(),
            public_to_internal: HashMap::new(),
        };

        // Create mapping from the message
        let mut visitor = MapperVisitor {
            mapper: &mut mapper,
            map_name,
        };

        for node in &message.nodes {
            node.visit(&mut visitor, None);
        }

        mapper
    }

    fn visit_placeholder_name<F>(&mut self, internal_name: &str, map_name: &F)
    where
        F: Fn(&str) -> String,
    {
        if internal_name.is_empty() || self.internal_to_public.contains_key(internal_name) {
            return;
        }

        let mut public_name = map_name(internal_name);

        if self.public_to_internal.contains_key(&public_name) {
            // Create a new name when it has already been used
            let next_id = *self.public_to_next_id.get(&public_name).unwrap();
            self.public_to_next_id.insert(public_name.clone(), next_id + 1);
            public_name = format!("{}_{}", public_name, next_id);
        } else {
            self.public_to_next_id.insert(public_name.clone(), 1);
        }

        self.internal_to_public.insert(internal_name.to_string(), public_name.clone());
        self.public_to_internal.insert(public_name, internal_name.to_string());
    }
}

impl PlaceholderMapper for SimplePlaceholderMapper {
    fn to_public_name(&self, internal_name: &str) -> Option<String> {
        self.internal_to_public.get(internal_name).cloned()
    }

    fn to_internal_name(&self, public_name: &str) -> Option<String> {
        self.public_to_internal.get(public_name).cloned()
    }
}

struct MapperVisitor<'a, F>
where
    F: Fn(&str) -> String,
{
    mapper: &'a mut SimplePlaceholderMapper,
    map_name: F,
}

impl<'a, F> Visitor for MapperVisitor<'a, F>
where
    F: Fn(&str) -> String,
{
    fn visit_text(&mut self, _text: &i18n::Text, _context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any> {
        Box::new(())
    }

    fn visit_container(&mut self, container: &i18n::Container, _context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any> {
        for child in &container.children {
            child.visit(self, None);
        }
        Box::new(())
    }

    fn visit_icu(&mut self, icu: &i18n::Icu, _context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any> {
        for node in icu.cases.values() {
            node.visit(self, None);
        }
        Box::new(())
    }

    fn visit_tag_placeholder(&mut self, ph: &i18n::TagPlaceholder, _context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any> {
        self.mapper.visit_placeholder_name(&ph.start_name, &self.map_name);
        for child in &ph.children {
            child.visit(self, None);
        }
        self.mapper.visit_placeholder_name(&ph.close_name, &self.map_name);
        Box::new(())
    }

    fn visit_placeholder(&mut self, ph: &i18n::Placeholder, _context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any> {
        self.mapper.visit_placeholder_name(&ph.name, &self.map_name);
        Box::new(())
    }

    fn visit_block_placeholder(&mut self, ph: &i18n::BlockPlaceholder, _context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any> {
        self.mapper.visit_placeholder_name(&ph.start_name, &self.map_name);
        for child in &ph.children {
            child.visit(self, None);
        }
        self.mapper.visit_placeholder_name(&ph.close_name, &self.map_name);
        Box::new(())
    }

    fn visit_icu_placeholder(&mut self, ph: &i18n::IcuPlaceholder, _context: Option<&mut dyn std::any::Any>) -> Box<dyn std::any::Any> {
        self.mapper.visit_placeholder_name(&ph.name, &self.map_name);
        Box::new(())
    }
}

