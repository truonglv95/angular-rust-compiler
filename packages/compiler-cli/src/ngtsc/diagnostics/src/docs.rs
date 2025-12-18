use super::error_code::ErrorCode;
use std::collections::HashSet;
use std::sync::LazyLock;

/// Contains a set of error messages that have detailed guides at angular.io.
/// Full list of available error guides can be found at https://angular.dev/errors
pub static COMPILER_ERRORS_WITH_GUIDES: LazyLock<HashSet<ErrorCode>> = LazyLock::new(|| {
    let mut s = HashSet::new();
    s.insert(ErrorCode::DecoratorArgNotLiteral);
    s.insert(ErrorCode::ImportCycleDetected);
    s.insert(ErrorCode::ParamMissingToken);
    s.insert(ErrorCode::SchemaInvalidElement);
    s.insert(ErrorCode::SchemaInvalidAttribute);
    s.insert(ErrorCode::MissingReferenceTarget);
    s.insert(ErrorCode::ComponentInvalidShadowDomSelector);
    s.insert(ErrorCode::WarnNgmoduleIdUnnecessary);
    s
});
