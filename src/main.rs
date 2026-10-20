#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod animation;
mod app;
mod config;
mod features;
mod platform;
mod render;
mod state;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting Rust Dynamic Island (Phase 2 Productivity MVP)...");
    app::run()?;
    Ok(())
}
