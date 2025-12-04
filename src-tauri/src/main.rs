//! `CreatorOps` - Content creator workflow management application
//!
//! Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> creatorops_lib::AppResult {
    creatorops_lib::run()
}
