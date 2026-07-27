use std::fs;
use std::sync::Mutex;

use tauri::Manager;

use rusqlite::{OptionalExtension, Connection, Result};

use serde::Serialize;

struct DbState {
    conn: Mutex<Connection>
}

#[derive(Serialize)]
struct BookCard {
    id: i32,
    isbn: String,
    title: String,
    author: String,
    status: String,
    cover_url: String
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn search_isbn_book(state: tauri::State<'_, DbState>, isbn: String) -> Result<Option<BookCard>, String> {
   let conn = state.conn.lock().map_err(|e| e.to_string())?;
   
   let mut stmt = conn
        .prepare("SELECT id, isbn, title, author, status, cover_url FROM books WHERE isbn = ?1")
        .map_err(|e| e.to_string())?;

    let book_result = stmt.query_row([isbn], |row| {
        Ok(BookCard {
            id: row.get(0)?,
            isbn: row.get(1)?,
            title: row.get(2)?,
            author: row.get(3)?,
            status: row.get(4)?,
            cover_url: row.get(5)?,
        })
    });

    book_result.optional().map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_data_path = app.path().app_data_dir()
                .expect("Не определился путь к App Data");
            
            if !app_data_path.exists() {
                fs::create_dir_all(&app_data_path)?;
            }
            
            let db_path = app_data_path.join("database.db");
            
            let conn = Connection::open(db_path)?;

            conn.execute("CREATE TABLE IF NOT EXISTS books (id INTEGER PRIMARY KEY AUTOINCREMENT, isbn TEXT UNIQUE NOT NULL, title TEXT NOT NULL, author TEXT NOT NULL, status TEXT DEFAULT \"Не прочитано\", publisher TEXT, series TEXT, binding TEXT, page_count INTEGER, section TEXT, cover_url TEXT NOT NULL);", [])?;
            
            app.manage(DbState {
                conn: Mutex::new(conn),
            });

            println!("Путь к данным приложения: {:?}", app_data_path);
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet])
        .invoke_handler(tauri::generate_handler![search_isbn_book])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
