/**
 * Angular Compiler CLI - ngc (ng compiler)
 *
 * Main entry point for Angular compilation
 */

use clap::{Arg, Command};
use std::process;

fn main() {
    let matches = Command::new("ngc")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Angular Compiler (Rust implementation)")
        .arg(
            Arg::new("project")
                .short('p')
                .long("project")
                .value_name("PATH")
                .help("Path to tsconfig.json"),
        )
        .get_matches();

    // TODO: Handle watch mode via perform_watch
    
    let temp_project;
    let project = if let Some(p) = matches.get_one::<String>("project") {
        Some(p.as_str())
    } else {
        // Check if tsconfig.json exists in cwd
        if std::path::Path::new("tsconfig.json").exists() {
            temp_project = Some("tsconfig.json".to_string());
            temp_project.as_deref()
        } else {
            None
        }
    };

    use angular_compiler_cli::perform_compile::perform_compilation_simple;
    
    let result = perform_compilation_simple(project, None, None);
    
    if !result.diagnostics.is_empty() {
        for diag in result.diagnostics {
            eprintln!("Error: {}", diag);
        }
        process::exit(1);
    }
}
