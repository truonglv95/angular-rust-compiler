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
        .arg(
            Arg::new("watch")
                .short('w')
                .long("watch")
                .help("Watch for file changes"),
        )
        .get_matches();

    // TODO: Implement actual compilation logic
    println!("Angular Compiler (Rust) - ngc");
    println!("Version: {}", env!("CARGO_PKG_VERSION"));
    
    if let Some(project) = matches.get_one::<String>("project") {
        println!("Project: {}", project);
    }
    
    if matches.get_flag("watch") {
        println!("Watch mode enabled");
    }

    // Placeholder - will implement actual compilation
    eprintln!("Compilation not yet implemented");
    process::exit(1);
}

