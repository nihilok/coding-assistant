use crate::structs::{History, Message, MyError, Role};
use crate::{MAX_HISTORY_LENGTH, SYSTEM_MESSAGE};
use chrono::Local;
use serde_json::{from_reader, to_writer};
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;
use std::{fs, io, path};

static HISTORY_PATH: &'static str = "history.json";

fn get_history_path() -> PathBuf {
    let data_dir = dirs::home_dir()
        .expect("Home directory not found")
        .join(".coding-assistant-history");

    fs::create_dir_all(&data_dir).expect("Failed to create directory");

    let path = data_dir.join(HISTORY_PATH);
    path
}

pub fn write_history(history: &History) -> io::Result<()> {
    let file = fs::File::create(get_history_path())?;
    let writer = BufWriter::new(file);
    to_writer(writer, history)?;
    Ok(())
}

pub fn read_history() -> io::Result<History> {
    let file = match fs::File::open(get_history_path()) {
        Ok(file) => file,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            // File doesn't exist, return default empty history with default system message.
            return Ok(History::new());
        }
        Err(e) => return Err(e),
    };

    let reader = BufReader::new(file);
    let mut history: History =
        from_reader(reader).map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

    if history.history.len() > MAX_HISTORY_LENGTH {
        history.history = history
            .history
            .split_off(history.history.len() - MAX_HISTORY_LENGTH);
    }

    // Check the first message when history is not empty; prepend a system message if needed
    if history.history.is_empty() || history.history[0].role != Role::SYSTEM {
        history
            .history
            .insert(0, Message::new(Role::SYSTEM, SYSTEM_MESSAGE));
    }

    Ok(history)
}

#[tauri::command]
pub fn clear_history() -> Result<(), MyError> {
    let file_path = get_history_path();
    if file_path.exists() {
        let now = Local::now();
        let datetime_format = now.format("%Y%m%d%H%M%S").to_string();
        let backup_filename = format!("{}_{}.json", HISTORY_PATH, datetime_format);

        // Get parent directory of the current history path
        let parent_dir = file_path.parent().unwrap_or_else(|| path::Path::new("/"));
        let backup_path = parent_dir.join(&backup_filename);

        fs::copy(&file_path, &backup_path)?;
        fs::remove_file(&file_path)?;
    }

    let history = History::new();

    let content = serde_json::to_string(&history)?;

    fs::write(&file_path, content.as_bytes())?;

    Ok(())
}
