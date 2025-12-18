pub mod src {
    pub mod error;
    pub mod error_code;
    pub mod util;
    pub mod docs;
    pub mod extended_template_diagnostic_name;
    pub mod error_details_base_url;
}

pub use src::error::*;
pub use src::error_code::*;
pub use src::util::*;
pub use src::docs::*;
pub use src::extended_template_diagnostic_name::*;
pub use src::error_details_base_url::*;

#[cfg(test)]
mod test;
