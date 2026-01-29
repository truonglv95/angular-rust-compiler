use angular_compiler_cli::bundler::bundle_project;
use angular_compiler_cli::config::angular::AngularConfig;
use clap::{Arg, Command};
use std::path::{Path, PathBuf};
use std::process;

fn main() {
    let matches = Command::new("ng_bundle")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Angular Bundle Compiler")
        .arg(
            Arg::new("project")
                .short('p')
                .long("project")
                .value_name("PATH")
                .help("Path to angular.json or tsconfig.json")
                .global(true),
        )
        .subcommand(Command::new("serve").about("Serve the application using Vite"))
        .get_matches();

    let project_arg = matches.get_one::<String>("project").cloned();

    match matches.subcommand() {
        Some(("serve", _)) => {
            run_serve(project_arg);
        }
        _ => {
            run_build(project_arg);
        }
    }
}

fn resolve_project_path(project_arg: Option<String>) -> PathBuf {
    project_arg
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("angular.json"))
}

fn run_serve(project_arg: Option<String>) {
    let project_path = resolve_project_path(project_arg);
    // Canonicalize path to make it absolute
    let project_path = std::fs::canonicalize(&project_path).unwrap_or(project_path);

    let root_dir = project_path.parent().unwrap_or(Path::new("."));

    if !project_path.exists() {
        eprintln!("Warning: Project configuration not found at {:?}, assuming default vite config availability.", project_path);
    }

    println!("Starting Vite server via Node.js...");
    println!("Project: {:?}", project_path);

    // Call vite via nodejs (npx vite)
    let status = process::Command::new("npx")
        .arg("vite")
        .current_dir(root_dir)
        .env(
            "ANGULAR_PROJECT_PATH",
            project_path.to_string_lossy().as_ref(),
        )
        .status();

    match status {
        Ok(s) => {
            if !s.success() {
                eprintln!("Vite exited with error code: {:?}", s.code());
                process::exit(s.code().unwrap_or(1));
            }
        }
        Err(e) => {
            eprintln!("Failed to start Vite: {}", e);
            eprintln!("Ensure node and npm are installed and available in PATH.");
            process::exit(1);
        }
    }
}

fn run_build(project_arg: Option<String>) {
    let project_path = resolve_project_path(project_arg);

    if !project_path.exists() {
        eprintln!("Error: Project configuration not found: {:?}", project_path);
        process::exit(1);
    }

    println!("Loading configuration from {:?}", project_path);
    // Load config to determine dist dir and assets (we need this for assets not handled by bundler)
    let config = AngularConfig::load(&project_path).unwrap_or_else(|e| {
        eprintln!("Failed to parse config: {}", e);
        process::exit(1);
    });

    let (name, project) = config.get_project(None).unwrap_or_else(|| {
        eprintln!("No project found in configuration");
        process::exit(1);
    });

    println!("Building project: {}", name);

    let build_options = project
        .architect
        .as_ref()
        .and_then(|a| a.get("build"))
        .and_then(|t| t.options.as_ref());

    // Bundle using library
    let result = match bundle_project(&project_path, false) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Bundling failed: {}", e);
            process::exit(1);
        }
    };

    // Determine dist dir
    let mut dist_dir = project_path.parent().unwrap_or(Path::new(".")).join("dist");
    if let Some(options) = build_options {
        if let Some(out_path) = &options.output_path {
            dist_dir = project_path
                .parent()
                .unwrap_or(Path::new("."))
                .join(out_path);
        }
    }

    if !dist_dir.exists() {
        std::fs::create_dir_all(&dist_dir).unwrap();
    }

    // Write Bundle
    let bundle_path = dist_dir.join("bundle.js");
    std::fs::write(&bundle_path, &result.bundle_js).unwrap();
    println!("Bundle written to {:?}", bundle_path);

    // Write all individual files (including collected resources)
    for (rel_path, content) in &result.files {
        let file_path = dist_dir.join(rel_path);
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&file_path, content).unwrap();
        // println!("File written to {:?}", file_path);
    }
    println!("Wrote {} individual files to dist", result.files.len());

    // Write Chunks
    for (chunk_name, chunk_content) in &result.chunks {
        let chunk_path = dist_dir.join(chunk_name);
        std::fs::write(&chunk_path, chunk_content).unwrap();
        println!("Chunk written to {:?}", chunk_path);
    }

    // Write Styles
    if let Some(css) = result.styles_css {
        let styles_path = dist_dir.join("styles.css");
        std::fs::write(&styles_path, css).unwrap();
        println!("Styles written to {:?}", styles_path);
    }

    // Write Scripts
    if let Some(js) = result.scripts_js {
        let scripts_path = dist_dir.join("scripts.js");
        std::fs::write(&scripts_path, js).unwrap();
        println!("Scripts written to {:?}", scripts_path);
    }

    // Write Index HTML
    if let Some(html) = result.index_html {
        let index_path = dist_dir.join("index.html");
        std::fs::write(&index_path, html).unwrap();
        println!("Index HTML written to {:?}", index_path);
    }

    // Process Assets (Not handled by bundler which is memory-focused)
    if let Some(options) = build_options {
        if let Some(assets) = &options.assets {
            println!("Processing {} assets...", assets.len());
            process_assets(assets, &project_path.parent().unwrap(), &dist_dir).unwrap_or_else(
                |e| {
                    eprintln!("Failed to process assets: {}", e);
                },
            );
        }
    }
}

fn process_assets(
    assets: &[angular_compiler_cli::config::angular::Asset],
    project_root: &Path,
    dist_dir: &Path,
) -> anyhow::Result<()> {
    use angular_compiler_cli::config::angular::Asset;

    for asset in assets {
        match asset {
            Asset::String(pattern) => {
                let full_pattern = project_root.join(pattern);
                let pattern_str = full_pattern.to_string_lossy();

                for entry in glob::glob(&pattern_str)? {
                    let path = entry?;
                    if path.is_file() {
                        let relative = path.strip_prefix(project_root).unwrap_or(&path);
                        let dest = dist_dir.join(relative);
                        if let Some(parent) = dest.parent() {
                            std::fs::create_dir_all(parent)?;
                        }
                        std::fs::copy(&path, &dest)?;
                    }
                }
            }
            Asset::Object {
                glob,
                input,
                output,
            } => {
                let input_dir = project_root.join(input);
                let output_dir = if let Some(out) = output {
                    dist_dir.join(out)
                } else {
                    dist_dir.to_path_buf()
                };

                let pattern = input_dir.join(glob);
                let pattern_str = pattern.to_string_lossy();

                for entry in glob::glob(&pattern_str)? {
                    let path = entry?;
                    if path.is_file() {
                        let relative = path.strip_prefix(&input_dir).unwrap_or(&path);
                        let dest = output_dir.join(relative);
                        if let Some(parent) = dest.parent() {
                            std::fs::create_dir_all(parent)?;
                        }
                        std::fs::copy(&path, &dest)?;
                    }
                }
            }
        }
    }
    Ok(())
}
