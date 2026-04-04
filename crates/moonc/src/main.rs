use clap::{Parser as ClapParser, Subcommand};
use moonc::{driver, project, r#where};
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

#[derive(ClapParser)]
#[command(name = "moonc")]
#[command(version = "0.1.0")]
#[command(about = "Moon language compiler - Unity-first scripting language")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile .mn file(s) to C#
    Compile {
        /// File or directory to compile
        path: String,

        /// Output directory (default: same as source)
        #[arg(short, long)]
        output: Option<String>,

        /// Output diagnostics as JSON
        #[arg(long)]
        json: bool,

        /// Suppress warnings
        #[arg(short = 'w', long)]
        no_warnings: bool,
    },

    /// Check .mn file(s) without generating output
    Check {
        /// File or directory to check
        path: String,

        /// Output diagnostics as JSON
        #[arg(long)]
        json: bool,
    },

    /// Build project using .mnproject configuration
    Build {
        /// Output diagnostics as JSON
        #[arg(long)]
        json: bool,

        /// Watch for file changes and rebuild automatically
        #[arg(long)]
        watch: bool,
    },

    /// Initialize a new .mnproject in the current directory
    Init {
        /// Project name
        #[arg(short, long)]
        name: Option<String>,
    },

    /// Print the absolute path of the moonc binary
    Where,

    /// Print version information
    Version,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Where => {
            r#where::print_where();
        }
        Commands::Version => {
            println!("moonc 0.1.0");
            println!("Moon language compiler - Unity-first scripting language");
            println!("Backend: C# source generation");
        }
        Commands::Compile {
            path,
            output,
            json,
            no_warnings,
        } => {
            let files = match driver::collect_moon_files(&path) {
                Ok(files) if !files.is_empty() => files,
                Ok(_) => {
                    eprintln!("No .mn files found in '{}'", path);
                    process::exit(1);
                }
                Err(error) => {
                    eprintln!("Error: {}", error);
                    process::exit(1);
                }
            };

            let output_dir = output.as_ref().map(|value| Path::new(value.as_str()));
            let report = driver::compile_paths(&files, output_dir);

            if json {
                print_json(serde_json::json!({
                    "files": report.files,
                    "compiled": report.compiled,
                    "errors": report.errors,
                    "warnings": report.warnings,
                    "diagnostics": report.diagnostics,
                }));
            } else {
                print_text_diagnostics(&report, no_warnings);
                print_compiled_outputs(&report);
                println!();
                if report.errors > 0 {
                    eprintln!(
                        "Build failed: {} error(s), {} warning(s) in {} file(s)",
                        report.errors, report.warnings, report.files
                    );
                } else {
                    println!(
                        "Build succeeded: {} file(s) compiled, {} warning(s)",
                        report.compiled, report.warnings
                    );
                }
            }

            if report.errors > 0 {
                process::exit(1);
            }
        }
        Commands::Check { path, json } => {
            let files = match driver::collect_moon_files(&path) {
                Ok(files) if !files.is_empty() => files,
                Ok(_) => {
                    eprintln!("No .mn files found in '{}'", path);
                    process::exit(1);
                }
                Err(error) => {
                    eprintln!("Error: {}", error);
                    process::exit(1);
                }
            };

            let report = driver::check_paths(&files);

            if json {
                print_json(serde_json::json!({
                    "files": report.files,
                    "errors": report.errors,
                    "warnings": report.warnings,
                    "diagnostics": report.diagnostics,
                }));
            } else {
                print_text_diagnostics(&report, false);
                if report.errors > 0 {
                    eprintln!(
                        "Check failed: {} error(s), {} warning(s) in {} file(s)",
                        report.errors, report.warnings, report.files
                    );
                } else {
                    println!(
                        "Check passed: {} file(s), {} warning(s)",
                        report.files, report.warnings
                    );
                }
            }

            if report.errors > 0 {
                process::exit(1);
            }
        }
        Commands::Build { json, watch } => {
            let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let build = match driver::build_project(&cwd) {
                Ok(build) => build,
                Err(error) => {
                    eprintln!("{}", error);
                    process::exit(1);
                }
            };

            println!(
                "Project: {} ({})",
                build.project_name,
                build.project_root.display()
            );

            if json {
                print_json(serde_json::json!({
                    "project": build.project_name,
                    "files": build.report.files,
                    "compiled": build.report.compiled,
                    "errors": build.report.errors,
                    "warnings": build.report.warnings,
                    "output_dir": build.output_dir_display,
                    "diagnostics": build.report.diagnostics,
                }));
            } else {
                print_text_diagnostics(&build.report, false);
                print_compiled_outputs(&build.report);
                println!();
                if build.report.errors > 0 {
                    eprintln!(
                        "Build failed: {} error(s), {} warning(s) in {} file(s)",
                        build.report.errors, build.report.warnings, build.report.files
                    );
                } else {
                    println!(
                        "Build succeeded: {} file(s) -> {}",
                        build.report.compiled, build.output_dir_display
                    );
                }
            }

            if build.report.errors > 0 && !watch {
                process::exit(1);
            }

            if watch {
                watch_project(build.sources, build.output_dir);
            }
        }
        Commands::Init { name } => {
            let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let project_file = cwd.join(".mnproject");
            if project_file.exists() {
                eprintln!(".mnproject already exists");
                process::exit(1);
            }

            let project_name = name.unwrap_or_else(|| {
                cwd.file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_else(|| "MyProject".into())
            });

            let content = format!(
                r#"[project]
name = "{}"
moon_version = "0.1.0"

[compiler]
moonc_path = "moonc"
output_dir = "Assets/Generated/Moon"

[source]
include = ["Assets/Scripts/**/*.mn"]
exclude = []

[features]
auto_compile_on_save = true
generate_meta_files = true
pascal_case_methods = true
"#,
                project_name
            );

            fs::write(&project_file, content).expect("Failed to write .mnproject");
            println!("Created .mnproject for '{}'", project_name);
        }
    }
}

fn print_json(value: serde_json::Value) {
    println!("{}", serde_json::to_string_pretty(&value).unwrap());
}

fn print_text_diagnostics(report: &driver::DriverReport, no_warnings: bool) {
    for file_result in &report.file_results {
        let file_path = file_result.source_path.to_string_lossy();
        for diagnostic in &file_result.diagnostics {
            if no_warnings && diagnostic.severity == moonc::diagnostics::Severity::Warning {
                continue;
            }
            eprintln!("{}", driver::format_diagnostic(diagnostic, &file_path));
        }
    }
}

fn print_compiled_outputs(report: &driver::DriverReport) {
    for file_result in &report.file_results {
        if let Some(output_path) = &file_result.output_path {
            println!(
                "  {} -> {}",
                file_result.source_path.display(),
                output_path.display()
            );
        }
    }
}

fn watch_project(sources: Vec<PathBuf>, output_dir: PathBuf) {
    use notify::Watcher;
    use std::sync::mpsc;
    use std::time::Duration;

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let (project_config, project_root) = match project::MoonProject::find_and_load(&cwd) {
        Ok(result) => result,
        Err(error) => {
            eprintln!("{}", error);
            process::exit(1);
        }
    };

    println!("\nWatching for changes... (Ctrl+C to stop)");

    let (tx, rx) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(move |result: Result<notify::Event, notify::Error>| {
        if let Ok(event) = result {
            if event.kind.is_modify() || event.kind.is_create() {
                let has_mn = event
                    .paths
                    .iter()
                    .any(|path| path.extension().map_or(false, |ext| ext == "mn"));
                if has_mn {
                    let _ = tx.send(());
                }
            }
        }
    })
    .expect("Failed to create file watcher");

    for pattern in &project_config.source.include {
        let directory = pattern.split("**").next().unwrap_or(".");
        let watch_dir = project_root.join(directory);
        if watch_dir.exists() {
            watcher
                .watch(&watch_dir, notify::RecursiveMode::Recursive)
                .unwrap_or_else(|error| {
                    eprintln!("Watch error on {}: {}", watch_dir.display(), error)
                });
        }
    }

    let _ = sources;

    loop {
        match rx.recv() {
            Ok(()) => {
                std::thread::sleep(Duration::from_millis(300));
                while rx.try_recv().is_ok() {}

                println!("\n--- Rebuilding... ---");
                let sources = project_config.collect_sources(&project_root);
                let report = driver::compile_paths(&sources, Some(output_dir.as_path()));
                print_text_diagnostics(&report, false);
                if report.errors > 0 {
                    eprintln!("Rebuild: {} error(s)", report.errors);
                } else {
                    println!("Rebuild: {} file(s) compiled", report.compiled);
                }
            }
            Err(_) => break,
        }
    }
}
