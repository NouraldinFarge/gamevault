#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if std::env::args().any(|argument| argument == "--health-check") {
        if gamevault_lib::health_check().is_err() {
            std::process::exit(2);
        }
        return;
    }
    if gamevault_lib::run().is_err() {
        std::process::exit(1);
    }
}
