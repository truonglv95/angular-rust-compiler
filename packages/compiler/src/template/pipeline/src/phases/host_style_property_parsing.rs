//! Parse host style properties.
//!
//! Corresponds to packages/compiler/src/template/pipeline/src/phases/host_style_property_parsing.ts
//!
//! Host bindings are compiled using a different parser entrypoint, and are parsed quite differently
//! as a result. Therefore, we need to do some extra parsing for host style properties, as compared
//! to non-host style properties.

use crate::template::pipeline::ir;
use crate::template::pipeline::ir::enums::BindingKind;
use crate::template::pipeline::ir::ops::update::BindingOp;
use crate::template::pipeline::src::compilation::{CompilationJob, HostBindingCompilationJob};

const STYLE_DOT: &str = "style.";
const CLASS_DOT: &str = "class.";
const STYLE_BANG: &str = "style!";
const CLASS_BANG: &str = "class!";
const BANG_IMPORTANT: &str = "!important";

pub fn parse_host_style_properties(job: &mut dyn CompilationJob) {
    if job.kind() != crate::template::pipeline::src::compilation::CompilationJobKind::Host {
        return;
    }

    let host_job = unsafe {
        let job_ptr = job as *mut dyn CompilationJob;
        let host_job_ptr = job_ptr as *mut HostBindingCompilationJob;
        &mut *host_job_ptr
    };

    for op in host_job.root.update.iter_mut() {
        if op.kind() != ir::OpKind::Binding {
            continue;
        }

        unsafe {
            let op_ptr = op.as_mut() as *mut dyn ir::UpdateOp;
            let binding_ptr = op_ptr as *mut BindingOp;
            let binding = &mut *binding_ptr;

            if binding.binding_kind != BindingKind::Property {
                continue;
            }

            // Delete any `!important` suffixes from the binding name
            if binding.name.ends_with(BANG_IMPORTANT) {
                binding.name = binding.name[..binding.name.len() - BANG_IMPORTANT.len()]
                    .to_string()
                    .into();
            }

            if binding.name.starts_with(STYLE_DOT) {
                binding.binding_kind = BindingKind::StyleProperty;
                binding.name = binding.name[STYLE_DOT.len()..].to_string().into();

                if !is_css_custom_property(&binding.name) {
                    binding.name = hyphenate(&binding.name).into();
                }

                let parsed = parse_property(&binding.name);
                binding.name = parsed.property.into();
                binding.unit = parsed.suffix;
            } else if binding.name.starts_with(STYLE_BANG) {
                binding.binding_kind = BindingKind::StyleProperty;
                binding.name = "style".into();
            } else if binding.name.starts_with(CLASS_DOT) {
                binding.binding_kind = BindingKind::ClassName;
                let parsed = parse_property(&binding.name[CLASS_DOT.len()..]);
                binding.name = parsed.property.into();
            } else if binding.name.starts_with(CLASS_BANG) {
                binding.binding_kind = BindingKind::ClassName;
                let parsed = parse_property(&binding.name[CLASS_BANG.len()..]);
                binding.name = parsed.property.into();
            }
        }
    }
}

/// Checks whether property name is a custom CSS property.
/// See: https://www.w3.org/TR/css-variables-1
fn is_css_custom_property(name: &str) -> bool {
    name.starts_with("--")
}

fn hyphenate(value: &str) -> String {
    // Convert camelCase to kebab-case
    let mut result = String::with_capacity(value.len() + 10); // Pre-allocate some extra space
    let chars: Vec<char> = value.chars().collect();

    for (i, ch) in chars.iter().enumerate() {
        if i > 0 && ch.is_uppercase() && chars[i - 1].is_lowercase() {
            result.push('-');
        }
        result.push(ch.to_ascii_lowercase());
    }

    result
}

struct ParsedProperty {
    property: String,
    suffix: Option<String>,
}

fn parse_property(name: &str) -> ParsedProperty {
    let mut name = name.to_string();

    // Remove !important if present
    if let Some(override_index) = name.find("!important") {
        if override_index > 0 {
            name = name[..override_index].to_string();
        } else {
            name = String::new();
        }
    }

    let mut suffix: Option<String> = None;
    let property: String;

    // Parse unit from last dot (e.g., "width.px" -> property="width", suffix="px")
    if let Some(unit_index) = name.rfind('.') {
        if unit_index > 0 {
            suffix = Some(name[unit_index + 1..].to_string());
            property = name[..unit_index].to_string();
        } else {
            property = name;
        }
    } else {
        property = name;
    }

    ParsedProperty { property, suffix }
}
