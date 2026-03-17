use dashmap::DashMap;
use ouroboros::self_referencing;
use oxc_allocator::Allocator;
use oxc_ast::ast::Program;
use std::path::PathBuf;
use std::sync::Arc;

#[self_referencing(pub_extras)]
pub struct CachedAst {
    pub source: String,
    pub allocator: Allocator,
    #[borrows(source, allocator)]
    #[covariant]
    pub program: Program<'this>,
}

// Manually implement Send and Sync since we know our Allocator and Program are safe to share
// across threads as long as we only read from them (which dashmap provides via Arc).
// The Program references the Allocator and the String source, all of which are stable in memory.
unsafe impl Send for CachedAst {}
unsafe impl Sync for CachedAst {}

pub type AstCache = Arc<DashMap<PathBuf, CachedAst>>;
