// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Check if invoked as unattended background runner:
    // e.g. "resticdashboard runner --profile-id <id> --base-dir <dir>"
    if args.iter().any(|arg| arg == "runner" || arg == "--runner") {
        let mut profile_id = String::new();
        let mut base_dir = resticdashboard_lib::system::get_persistent_app_dir();

        let mut i = 0;
        while i < args.len() {
            if (args[i] == "--profile-id" || args[i] == "-p") && i + 1 < args.len() {
                profile_id = args[i + 1].clone();
                i += 1;
            } else if (args[i] == "--base-dir" || args[i] == "-b") && i + 1 < args.len() {
                base_dir = PathBuf::from(&args[i + 1]);
                i += 1;
            }
            i += 1;
        }

        if profile_id.is_empty() {
            eprintln!("Error: --profile-id is required for runner mode");
            std::process::exit(1);
        }

        match resticdashboard_lib::runner::execute_unattended_runner(&profile_id, &base_dir) {
            Ok(code) => std::process::exit(code),
            Err(e) => {
                eprintln!("Runner failed: {}", e);
                std::process::exit(1);
            }
        }
    }

    // Default: Run desktop GUI controller
    resticdashboard_lib::run();
}
