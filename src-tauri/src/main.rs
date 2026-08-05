// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (quick_clean, paths) = fileclear_lib::parse_args(&args);
    fileclear_lib::run(quick_clean, paths);
}
