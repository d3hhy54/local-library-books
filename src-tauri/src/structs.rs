use serde::{Deserialize, Serialize};

use std::{path::PathBuf, sync::Mutex};
use rusqlite::Connection;

pub struct AppState {
    pub conn: Mutex<Connection>,
    pub cover_path: PathBuf
}

#[derive(Serialize, Debug)]
pub struct BookCard {
    pub id: i32,
    pub isbn: String,
    pub title: String,
    pub author: String,
    pub status: String,
    pub cover_url: String
}

#[derive(Serialize, Deserialize)]
pub struct BookPage {
    pub id: i32,
    pub isbn: String,
    pub title: String,
    pub author: String,
    pub status: String, // потом добавлю enums
    pub publisher: Option<String>,
    pub series: Option<String>,
    pub binding: Option<String>,
    pub page_count: Option<i32>,
    pub section: Option<String>,
    pub cover_url: String
}

#[derive(Serialize, Debug)]
pub struct BookParse {
    pub url_page: String,
    pub isbn: String,
    pub title: String,
    pub author: String,
    pub publisher: Option<String>,
    pub series: Option<String>,
    pub binding: Option<String>,
    pub page_count: Option<String>,
    pub section: Option<String>,
    pub cover_url: String
}

#[derive(Serialize, Deserialize)]
pub struct Filters {
    pub status: Option<Vec<String>>,
    pub publisher: Option<Vec<String>>,
    pub series: Option<Vec<String>>,
    pub binding: Option<Vec<String>>,
    pub section: Option<Vec<String>>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FiltersParams {
    pub status: Vec<String>,
    pub publisher: Vec<String>,
    pub series: Vec<String>,
    pub binding: Vec<String>,
    pub section: Vec<String>
}