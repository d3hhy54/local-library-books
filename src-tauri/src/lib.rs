pub mod parser;
pub mod structs;
pub mod utils;

use crate::structs::*;
use crate::parser::*;
use crate::utils::*;

use std::fs;
use std::sync::Mutex;

use tauri::AppHandle;
use tauri::Manager;
use rusqlite::{OptionalExtension, Connection, Result, params_from_iter};

fn book_card_from_row(row: &rusqlite::Row) -> rusqlite::Result<BookCard> {
    Ok(BookCard {
        id: row.get(0)?,
        isbn: row.get(1)?,
        title: row.get(2)?,
        author: row.get(3)?,
        status: row.get(4)?,
        cover_url: row.get(5)?,
    })
}

#[tauri::command]
fn select_from_isbn_book(
    state: tauri::State<'_, AppState>, 
    isbn: String
) -> Result<Option<BookCard>, String> {
   let conn = state.conn.lock().map_err(|e| e.to_string())?;
   
   let mut stmt = conn
        .prepare("SELECT id, isbn, title, author, status, cover_url FROM books WHERE isbn = ?1")
        .map_err(|e| e.to_string())?;

    let book_result = stmt.query_row([isbn], book_card_from_row);
    
    book_result.optional().map_err(|e| e.to_string())
}

#[tauri::command]
fn search_by_query_book(
    state: tauri::State<'_, AppState>, 
    query: String, 
    filters: Option<Filters>,
    limit: Option<i32>,
    offset: Option<i32>
) -> Result<Vec<BookCard>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;

    let query_param = format!("%{}%", query.to_lowercase());
    
    let base_sql = "SELECT id, isbn, title, author, status, cover_url FROM books WHERE (title_lower LIKE ?1 OR author_lower LIKE ?1)";

    let mut final_args = vec![&query_param];

    let sql = if let Some(ref f) = filters.as_ref() {
        let (filters_query, query_args) = from_filters_to_query_with_args(f);
        
        if filters_query.is_empty() {
            base_sql.to_string()
        } else {
            final_args.extend(query_args);
            format!("{} AND {}", base_sql, filters_query)
        }
    } else {
        base_sql.to_string()
    };

    let limit = match limit {
        Some(l) => l.to_string(),
        None => "30".to_string()
    };

    let offset = match offset {
        Some(o) => o.to_string(),
        None => "0".to_string()
    };

    let sql = format!("{} ORDER BY title ASC LIMIT ? OFFSET ?", sql);
    final_args.push(&limit);
    final_args.push(&offset);
    

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| e.to_string())?;

    let book_iter = stmt
        .query_map(params_from_iter(final_args.iter()), book_card_from_row)
        .map_err(|e| e.to_string())?;
    
    let mut books = Vec::new();
    for book in book_iter {
        books.push(book.map_err(|e| e.to_string())?);
    }
    
    Ok(books)


}

#[tauri::command]
fn get_all_books(
    state: tauri::State<'_, AppState>, 
    filters: Option<Filters>,
    limit: Option<i32>,
    offset: Option<i32>
) -> Result<Vec<BookCard>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;

    let (sql, mut args) = if let Some(ref f) = filters {
        let (filters_query, args) = from_filters_to_query_with_args(f);
        let query = format!("SELECT id, isbn, title, author, status, cover_url FROM books WHERE {}", filters_query);
        (query, args)
    } else {
        let query = "SELECT id, isbn, title, author, status, cover_url FROM books".to_string();
        (query, Vec::new())
    };

    let limit = match limit {
        Some(l) => l.to_string(),
        None => "30".to_string()
    };

    let offset = match offset {
        Some(o) => o.to_string(),
        None => "0".to_string()
    };

    let sql = format!("{} ORDER BY title ASC LIMIT ? OFFSET ?", sql);
    args.push(&limit);
    args.push(&offset);

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| e.to_string())?;

    let book_iter = stmt
        .query_map(params_from_iter(args.iter()), book_card_from_row)
        .map_err(|e| e.to_string())?;
    
    let mut books = Vec::new();
    for book in book_iter {
        books.push(book.map_err(|e| e.to_string())?);
    } 
    
    Ok(books)
}

#[tauri::command]
fn get_id_book(
    state: tauri::State<'_, AppState>, 
    id: i32
) -> Result<Option<BookPage>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;

    let sql = "SELECT * FROM books WHERE id = ?1";

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
async fn search_parse_book(
    app_handle: AppHandle, 
    isbn: String
) -> Result<BookParse, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app_handle.state::<AppState>();

        let isbn_copy = (&isbn).to_string();
        let book = select_from_isbn_book(state, isbn_copy)?;
        if book.is_some(){
            return Err("Это книга есть в базе данных!".to_string())
        }
        let book = parse_bookvoed_book(&isbn)?;
        Ok(book)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
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
    page_count: Option<i32>,
    section: Option<String>,
) -> Result<String, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;

    let sql = "INSERT INTO books (isbn, title, author, status, cover_url, publisher, series, binding, page_count, section, title_lower, author_lower) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)";

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
    conn.execute(
        sql,
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

#[tauri::command]
fn update_status_book(state: tauri::State<'_, AppState>, status: String, id: i32) -> Result<String, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;

    let sql = "UPDATE books SET status = ? WHERE id = ?";

    match conn.execute(
        sql,
        (
            status, 
            id
        )
    ) {
        Ok(_) => {
            Ok("Успешно обновлен статус!".to_string())
        },
        Err(e) => Err(e.to_string())
    }  
}

#[tauri::command]
fn get_filters_params(state: tauri::State<'_, AppState>) -> Result<FiltersParams, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;

    let params = ["status", "publisher", "series", "binding", "section"];
    let mut results: Vec<Vec<String>> = Vec::new();

    for param in params {
        let mut stmt = conn
        .prepare(&format!("SELECT DISTINCT {0} FROM books WHERE {0} IS NOT NULL AND {0} != '' ORDER BY {0} ASC", param))
        .map_err(|e| e.to_string())?;

        let result = stmt
            .query_map([], |row| {
                let value: String = row.get(0)?; 
                Ok(value)
            })
            .map_err(|e| e.to_string())?;

        let column_values = result
            .collect::<Result<Vec<String>, _>>()
            .map_err(|e| e.to_string())?;

        results.push(column_values);
    }

    Ok(
        FiltersParams {
            status: results.remove(0),
            publisher: results.remove(0),
            series: results.remove(0),
            binding: results.remove(0),
            section: results.remove(0)
        }
    )
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
            get_filters_params,
            update_status_book
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}