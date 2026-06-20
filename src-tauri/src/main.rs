// Hide the console window in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod actions;
mod app;
mod hotkeys;
mod ipc;
mod layout;
mod native;
mod settings;
mod state;
mod tray;
mod winmgr;

fn main() {
    app::run();
}
