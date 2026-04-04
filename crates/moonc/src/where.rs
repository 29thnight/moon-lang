//! `moonc where` — prints the absolute path of the moonc binary.
//! Used by Unity Editor, Visual Studio extensions, and other tools to locate moonc.

use std::env;
use std::path::PathBuf;

/// Returns the absolute path of the currently running moonc binary.
pub fn get_moonc_path() -> Option<PathBuf> {
    env::current_exe().ok()
}

/// Print the moonc path to stdout.
pub fn print_where() {
    match get_moonc_path() {
        Some(path) => println!("{}", path.display()),
        None => {
            eprintln!("Error: could not determine moonc location");
            std::process::exit(1);
        }
    }
}
