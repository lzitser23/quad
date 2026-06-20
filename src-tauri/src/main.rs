// Hide the console window in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod actions;
mod app;
mod native;
mod settings;
mod winmgr;

fn main() {
    app::run();
}
