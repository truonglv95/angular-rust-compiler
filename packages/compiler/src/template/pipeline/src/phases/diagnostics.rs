//! Template Diagnostics Phase
//!
//! Reports warnings for potential issues in the template, such as unused imports.

use crate::render3::view::api::R3TemplateDependencyMetadata;
use crate::template::pipeline::ir;
use crate::template::pipeline::src::compilation::ComponentCompilationJob;

pub fn phase(job: &mut ComponentCompilationJob) {
    check_unused_imports(job);
}

fn check_unused_imports(job: &mut ComponentCompilationJob) {
    use crate::output::output_ast::Expression;
    use crate::parse_util::{ParseError, ParseErrorLevel};

    for (i, dep) in job.available_dependencies.iter().enumerate() {
        if !job.used_dependencies.contains(&i) {
            let (name, module_name, source_span) = match dep {
                R3TemplateDependencyMetadata::Directive(dir) => {
                    // If selector is empty, we couldn't resolve metadata (e.g. user component in "lite" mode).
                    // Assume it's used to avoid false positive error NG8113.
                    if dir.selector.is_empty() {
                        continue;
                    }

                    let (name, module_name) = match &dir.type_ {
                        Expression::ReadVar(rv) => (rv.name.clone(), None),
                        Expression::External(ext) => (
                            ext.value
                                .name
                                .clone()
                                .unwrap_or_else(|| "Unknown".to_string()),
                            ext.value.module_name.clone(),
                        ),
                        _ => ("Unknown".to_string(), None),
                    };
                    (name, module_name, &dir.source_span)
                }
                R3TemplateDependencyMetadata::Pipe(pipe) => {
                    let module_name = match &pipe.type_ {
                        Expression::External(ext) => ext.value.module_name.clone(),
                        _ => None,
                    };
                    (pipe.name.clone(), module_name, &pipe.source_span)
                }
                R3TemplateDependencyMetadata::NgModule(_) => {
                    continue;
                }
            };

            // Filter out auto-generated namespace dependencies (i0, i1 etc. which usually map to core/common/forms)
            if let Some(mod_name) = &module_name {
                if mod_name == "@angular/core"
                    || mod_name == "@angular/common"
                    || mod_name == "@angular/forms"
                {
                    continue;
                }
            }

            if let Some(span) = source_span {
                job.diagnostics.push(ParseError {
                    span: span.clone(),
                    msg: format!(
                        "{} is not used within the template of {}",
                        name, job.component_name
                    ),
                    level: ParseErrorLevel::Warning,
                });
            }
        }
    }
}
