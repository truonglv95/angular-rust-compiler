//! TSC Plugin Module
//!
//! Corresponds to packages/compiler-cli/src/ngtsc/tsc_plugin.ts
//!
//! A plugin for `tsc_wrapped` which allows Angular compilation from a plain `ts_library`.

pub mod src;
#[cfg(test)]
mod test;

pub use src::*;
