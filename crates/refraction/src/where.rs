//! `prism where` — prints the absolute path of the prism binary.
//! Used by Unity Editor, Visual Studio extensions, and other tools to locate prism.

use std::env;
use std::path::PathBuf;

/// Returns the absolute path of the currently running prism binary.
pub fn get_prism_path() -> Option<PathBuf> {
    env::current_exe().ok()
}

/// Print the prism path to stdout.
pub fn print_where() {
    match get_prism_path() {
        Some(path) => println!("{}", path.display()),
        None => {
            eprintln!("Error: could not determine prism location");
            std::process::exit(1);
        }
    }
}
