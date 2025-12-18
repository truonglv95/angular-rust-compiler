use angular_compiler::VERSION;
use std::sync::LazyLock;

/// Base URL for the error details page.
pub static ERROR_DETAILS_BASE_URL: LazyLock<String> = LazyLock::new(|| {
    let version_sub_domain = if VERSION.major != "0" {
        format!("v{}.", VERSION.major)
    } else {
        "".to_string()
    };
    format!("https://{}angular.dev/errors", version_sub_domain)
});

/// Base URL for the error details page for the extended template diagnostics.
pub const EXTENDED_TEMPLATE_DIAGNOSTIC_BASE_URL: &str = "https://angular.dev/extended-diagnostics";
