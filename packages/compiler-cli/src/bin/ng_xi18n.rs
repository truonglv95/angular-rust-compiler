/**
 * Angular Compiler CLI - ng-xi18n
 *
 * Extract i18n messages from Angular templates
 */

use clap::{Arg, Command};
use std::process;

fn main() {
    let matches = Command::new("ng-xi18n")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Extract i18n messages from Angular templates")
        .arg(
            Arg::new("project")
                .short('p')
                .long("project")
                .value_name("PATH")
                .help("Path to tsconfig.json"),
        )
        .arg(
            Arg::new("output-path")
                .short('o')
                .long("output-path")
                .value_name("PATH")
                .help("Output path for extracted messages"),
        )
        .get_matches();

    // TODO: Implement actual i18n extraction logic
    println!("Angular i18n Extractor (Rust) - ng-xi18n");
    println!("Version: {}", env!("CARGO_PKG_VERSION"));
    
    if let Some(project) = matches.get_one::<String>("project") {
        println!("Project: {}", project);
    }
    
    if let Some(output) = matches.get_one::<String>("output-path") {
        println!("Output path: {}", output);
    }

    // Placeholder - will implement actual extraction
    eprintln!("i18n extraction not yet implemented");
    process::exit(1);
}

