// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod history;
mod open_ai_funcs;
mod structs;

static COMPLETION_TOKENS: u16 = 1024;
static MAX_HISTORY_LENGTH: usize = 10;
const SYSTEM_MESSAGE: &'static str = include_str!("system-message.txt");

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            open_ai_funcs::prompt,
            history::get,
            history::clear_history
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
