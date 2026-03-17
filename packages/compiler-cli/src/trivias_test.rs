use oxc_ast::{Trivias, Comment, CommentKind};
use oxc_span::Span;

pub fn test() {
    let mut trivias = Trivias::default();
    trivias.add_comment(Span::new(0, 5), CommentKind::Block, false, false, false);
}
