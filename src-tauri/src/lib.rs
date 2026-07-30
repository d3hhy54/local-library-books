mod parser;
mod structs;

use structs::*;
use parser::parse_bookvoed_book;

use std::fs;
use std::sync::Mutex;

use base64::prelude::*;
use tauri::Manager;
use rusqlite::{OptionalExtension, Connection, Result};

use crate::parser::download_image;

fn save_cover_from_data_url(
    state: &AppState,
    data_url: &str,
    isbn: &str,
) -> Result<String, String> {
    // Парсим Data URL: data:image/jpeg;base64,/9j/4AAQSkZJRg...
    let parts: Vec<&str> = data_url.split(',').collect();
    if parts.len() != 2 {
        return Err("Неверный формат Data URL".to_string());
    }
    
    // Получаем расширение из MIME типа
    let mime_part = parts[0];
    let ext = if mime_part.contains("jpeg") || mime_part.contains("jpg") {
        "jpg"
    } else if mime_part.contains("png") {
        "png"
    } else if mime_part.contains("gif") {
        "gif"
    } else if mime_part.contains("webp") {
        "webp"
    } else {
        "jpg" // по умолчанию
    };
    
    // Декодируем base64
    let base64_data = parts[1];
    let decoded = BASE64_STANDARD.decode(base64_data)
        .map_err(|e| format!("Ошибка декодирования base64: {}", e))?;
    
    // Сохраняем файл
    let mut cover_path = state.cover_path.clone();
    let filename = format!("{}.{}", isbn, ext);
    cover_path.push(&filename);
    
    std::fs::write(&cover_path, decoded)
        .map_err(|e| format!("Ошибка сохранения файла: {}", e))?;

    let file = match cover_path.to_str().map(|s| s.to_string()){
        Some(file) => file,
        None => return Err("Что то не так с файлом".to_string())
    };
    
    Ok(format!("{}", file))
}

#[tauri::command(rename_all="camelCase")]
fn save_file_to_covers(state: tauri::State<'_, AppState>, file_path: &str, isbn: &str) -> Result<String, String> {
    let covers_dir = &state.cover_path;
    
    let extension = std::path::Path::new(&file_path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("jpg");
    
    let file_name = format!("{}.{}", isbn, extension);
    let dest_path = covers_dir.join(&file_name);
    
    std::fs::copy(&file_path, &dest_path).map_err(|e| e.to_string())?;
    
    println!("Файл скопирован в: {:?}", dest_path);
    
    Ok(file_name)
}
#[tauri::command]
fn select_from_isbn_book(state: tauri::State<'_, AppState>, isbn: String) -> Result<Option<BookCard>, String> {
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

#[tauri::command]
fn search_by_query_book(state: tauri::State<'_, AppState>, query: String) -> Result<Vec<BookCard>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;

    let query = format!("%{}%", query.to_lowercase());
    let sql = String::from("SELECT id, isbn, title, author, status, cover_url FROM books WHERE title_lower LIKE ?1 OR author_lower LIKE ?1");

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| e.to_string())?;

    let book_iter = stmt
        .query_map([query], |row| {
            Ok(BookCard {
                id: row.get(0)?,
                isbn: row.get(1)?,
                title: row.get(2)?,
                author: row.get(3)?,
                status: row.get(4)?,
                cover_url: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?;
    
    let mut books = Vec::new();
    for book in book_iter {
        books.push(book.map_err(|e| e.to_string())?);
    }
    
    Ok(books)


}

#[tauri::command]
fn get_all_books(state: tauri::State<'_, AppState>) -> Result<Vec<BookCard>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;

    let sql = String::from("SELECT id, isbn, title, author, status, cover_url FROM books");

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| e.to_string())?;

    let book_iter = stmt
        .query_map([], |row| {
            Ok(BookCard {
                id: row.get(0)?,
                isbn: row.get(1)?,
                title: row.get(2)?,
                author: row.get(3)?,
                status: row.get(4)?,
                cover_url: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?;
    
    let mut books = Vec::new();
    for book in book_iter {
        books.push(book.map_err(|e| e.to_string())?);
    } 
    
    Ok(books)
}

#[tauri::command]
fn get_id_book(state: tauri::State<'_, AppState>, id: i32) -> Result<Option<BookPage>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;

    let sql = String::from("SELECT * FROM books WHERE id = ?1");

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| e.to_string())?;

    let book_result = stmt.query_row([id], |row| {
        Ok(BookPage {
            id: row.get(0)?,
            isbn: row.get(1)?,
            title: row.get(2)?,
            author: row.get(3)?,
            status: row.get(4)?, // потом добавлю enums
            publisher: row.get(5)?,
            series: row.get(6)?,
            binding: row.get(7)?,
            page_count: row.get(8)?,
            section: row.get(9)?,
            cover_url: row.get(10)?
        })
    });
    
    book_result.optional().map_err(|e| e.to_string())
}

#[tauri::command]
fn search_parse_book(state: tauri::State<'_, AppState>, isbn: String) -> Result<BookParse, String> {
    let isbn_copy = (&isbn).to_string();
    let book = select_from_isbn_book(state, isbn_copy)?;
    if !book.is_none(){
        return Err("Это книга есть в базе данных!".to_string())
    }
    let book = parse_bookvoed_book(&isbn)?;
    println!("BookParse: {:?}", book);
    Ok(book)
}

#[tauri::command(rename_all = "snake_case")]
fn insert_book(
    state: tauri::State<'_, AppState>,
    isbn: String,
    title: String,
    author: String,
    status: String,
    cover_url: String,
    publisher: Option<String>,
    series: Option<String>,
    binding: Option<String>,
    page_count: Option<String>,
    section: Option<String>,
) -> Result<String, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;

    let sql = String::from("INSERT INTO books (isbn, title, author, status, cover_url, publisher, series, binding, page_count, section, title_lower, author_lower) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)");

    let page_count: Option<i32> = page_count.and_then(|s| s.parse().ok());

    let path;
    if cover_url.starts_with("data:") {
        path = save_cover_from_data_url(&state, &cover_url, &isbn)?
    } else if cover_url.starts_with("https") {
        path = download_image(&state, &cover_url, &isbn)?
    } else if !cover_url.is_empty() {
        path = save_file_to_covers(state.clone(), &cover_url, &isbn)?
    } else {
        return Err("Не указана обложка".to_string());
    };
    println!("PATH: {}", path);
    conn.execute(
        &sql,
        (
            isbn,
            &title,
            &author,
            status, 
            path,
            publisher,
            series,
            binding,
            page_count,
            section,
            title.to_lowercase(),
            author.to_lowercase()
        )
    )
    .map_err(|e| e.to_string())?;

    Ok("Книга успешна добавлена.".to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init()) // <--- ДОБАВЬТЕ ЭТУ СТРОКУ
        .setup(|app| {
            let app_data_path = app.path().app_data_dir()
                .expect("Не определился путь к App Data");

            if !app_data_path.exists() {
                fs::create_dir_all(&app_data_path)?;
            }
            
            let db_path = app_data_path.join("database.db");
            let cover_path = app_data_path.join("covers");
            
            let conn = Connection::open(db_path)?;

            conn.execute("CREATE TABLE IF NOT EXISTS books (id INTEGER PRIMARY KEY AUTOINCREMENT, isbn TEXT UNIQUE NOT NULL, title TEXT NOT NULL, author TEXT NOT NULL, status TEXT DEFAULT \"Не прочитано\", publisher TEXT, series TEXT, binding TEXT, page_count INTEGER, section TEXT, cover_url TEXT NOT NULL, title_lower TEXT NOT NULL, author_lower TEXT NOT NULL);", [])?;

            conn.execute("CREATE INDEX IF NOT EXISTS idx_books_title_lower ON books(title_lower);", [])?;
            conn.execute("CREATE INDEX IF NOT EXISTS idx_books_author_lower ON books(author_lower);", [])?;

            app.manage(AppState {
                conn: Mutex::new(conn),
                cover_path: cover_path.clone()
            });

            fs::create_dir_all(cover_path)?;

            println!("Путь к данным приложения: {:?}", app_data_path);
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            select_from_isbn_book, 
            search_by_query_book,
            search_parse_book, 
            get_all_books,
            get_id_book,
            insert_book,
            save_file_to_covers
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}