// Hide the console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Elevated HidHide helper: do its job and exit before any UI starts.
    if let Some(code) = dualbridge_app::helper_main() {
        std::process::exit(code);
    }
    dualbridge_app::run()
}
