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
            let (name, source_span) = match dep {
                R3TemplateDependencyMetadata::Directive(dir) => {
                    let name = if let Expression::ReadVar(rv) = &dir.type_ {
                        rv.name.as_str()
                    } else {
                        "Unknown"
                    };
                    (name, &dir.source_span)
                }
                R3TemplateDependencyMetadata::Pipe(pipe) => (pipe.name.as_str(), &pipe.source_span),
                R3TemplateDependencyMetadata::NgModule(_) => {
                    continue;
                }
            };

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
