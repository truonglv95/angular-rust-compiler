use crate::directive_matching::{CssSelector, SelectorMatcher};
use crate::template::pipeline::ir::enums::OpKind;
use crate::template::pipeline::ir::ops::create::PipeOp;
use crate::template::pipeline::src::compilation::{
    CompilationJob, CompilationUnit, ComponentCompilationJob,
};

pub fn phase(job: &mut ComponentCompilationJob) {
    // Note: Directive matching is primarily handled during ingestion (ingest.rs)
    // where `maybe_record_directive_usage` is called.
    // This phase can serve as a secondary check or for pipes if needed.

    let mut pipes_to_mark = Vec::new();
    for unit in job.units() {
        for op in unit.ops() {
            match op.kind() {
                OpKind::Pipe => {
                    if let Some(pipe_op) = op.as_any().downcast_ref::<PipeOp>() {
                        pipes_to_mark.push(pipe_op.name.clone());
                    }
                }
                _ => {}
            }
        }
    }

    for pipe_name in pipes_to_mark {
        job.mark_pipe_used(&pipe_name);
    }
}
