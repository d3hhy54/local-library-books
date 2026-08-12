use std::collections::HashMap;
use std::fs::File;
use std::io::copy;
use std::sync::OnceLock;

use reqwest;
use reqwest::blocking::Client;
use url::Url;
use scraper::{Html, Selector};

use crate::structs::*;

const DOMEN: &str = "https://bookvoed.ru";
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64; rv:150.0) Gecko/20100101 Firefox/150.0";

static HTTP_CLIENT: OnceLock<Client> = OnceLock::new();

fn get_client() -> &'static Client {
    HTTP_CLIENT.get_or_init(|| {
        Client::builder()
            .user_agent(USER_AGENT)
            .build()
            .expect("Failed to create HTTP client")
    })
}

fn replace_params_url(url: &str) -> Result<String, String> {
    let mut url = Url::parse(url).map_err(|e| e.to_string())?;

    let new_width = "300";
    let new_height = "400";

    // Собираем новые параметры запроса
    let mut query_pairs = Vec::new();

    // Проходим по старым параметрам и меняем нужные
    for (key, value) in url.query_pairs() {
        if key == "width" {
            query_pairs.push((key.into_owned(), new_width.to_string()));
        } else if key == "height" {
            query_pairs.push((key.into_owned(), new_height.to_string()));
        } else {
            query_pairs.push((key.into_owned(), value.into_owned()));
        }
    }

    // Записываем обновленные параметры обратно в URL
    url.set_query(None); // Очищаем старую строку запроса
    let mut encoder = url.query_pairs_mut();
    for (key, value) in query_pairs {
        encoder.append_pair(&key, &value);
    }
    drop(encoder);

    println!("Итоговая ссылка:\n{}", url);
    Ok(url.to_string())
}

pub fn download_image(state: &tauri::State<'_, AppState>, url: &str, isbn: &str) -> Result<String, String> {
    let output_path = state.cover_path.join(format!("{}.jpg", isbn));
    let url = replace_params_url(url)?;
    // Отправляем GET-запрос по ссылке
    let client = get_client();

    let mut response = client
        .get(&url)
        .send()
        .map_err(|e| e.to_string())?;

    // Создаем пустой файл для записи
    let mut dest = File::create(&output_path)
        .map_err(|e| e.to_string())?;

    // Копируем данные ответа в файл
    copy(&mut response, &mut dest)
        .map_err(|e| e.to_string())?;

    println!("Картинка успешно скачана!");

    let path = match output_path.to_str().map(|s| s.to_string()) {
        Some(path) => path,
        None => return Err("Нет пути созданного файла".to_string())
    };

    Ok(path)
}


fn get_catalog_page(isbn: &str) -> Result<String, String>{
    let client = get_client();

    let url = format!("{}/search?q={}", DOMEN, isbn);
    let response = client
        .get(&url)
        .send()
        .map_err(|e| e.to_string())?;

    let html = response.text().map_err(|e| e.to_string())?;

    Ok(html)
}

fn parse_catalog_page(html: &str) -> Result<(String, String), String> {
    let document = Html::parse_document(html);
    
    let link_sel = Selector::parse("a.product-description__link")
        .map_err(|e| e.to_string())?;
    
    let element = document
        .select(&link_sel)
        .next()
        .ok_or("Книга не найдена.")?;
    
    let name = element
        .text()
        .next()
        .ok_or("Название не было получено")?
        .to_string();
    
    let url = element
        .value()
        .attr("href")
        .ok_or("Атрибут href не найден.")?
        .to_string();
    
    Ok((name, format!("{}{}", DOMEN, url)))
}

fn get_book_page(url: &str) -> Result<String, String> {
    let client = get_client();

    let response = client
        .get(url)
        .send()
        .map_err(|e| e.to_string())?;

    let html = response.text().map_err(|e| e.to_string())?;

    Ok(html)
}

fn parse_url_image(html: &str) -> Result<String, String> {
    let document = Html::parse_document(&html);

    let url_sel = Selector::parse("img.product-preview__big-img")
        .map_err(|e| e.to_string())?;

    let element = document
        .select(&url_sel)
        .next()
        .ok_or("URL Книги не найдена.")?;

    let url = element
        .value()
        .attr("src")
        .ok_or("Атрибут src не найден.")?;

    Ok(format!("https:{}", url))
}


fn parse_metadata_table(html: &str) -> Result<HashMap<String, String>, String> {
    let document = Html::parse_document(html);
    
    let table_selector = Selector::parse("table.product-characteristics-full__table")
        .map_err(|e| format!("Ошибка селектора таблицы: {}", e))?;
    
    let table = document
        .select(&table_selector)
        .next()
        .ok_or_else(|| "Таблица с характеристиками не найдена".to_string())?;
    
    let row_selector = Selector::parse("tr.product-characteristics-full__row")
        .map_err(|e| format!("Ошибка селектора строк: {}", e))?;
    
    let th_selector = Selector::parse("th.product-characteristics-full__cell-th")
        .map_err(|e| format!("Ошибка селектора th: {}", e))?;
    
    let td_selector = Selector::parse("td.product-characteristics-full__cell-td")
        .map_err(|e| format!("Ошибка селектора td: {}", e))?;
    
    let mut metadata = HashMap::new();
    
    for row in table.select(&row_selector) {
        let key = match row.select(&th_selector).next() {
            Some(th) => th.text().collect::<String>().trim().to_string(),
            None => continue,
        };
        
        // Получаем значение (td)
        let value = match row.select(&td_selector).next() {
            Some(td) => {
                td.text()
                    .collect::<String>()
                    .split_whitespace()  
                    .collect::<Vec<_>>()
                    .join(" ")           
                    .trim()
                    .to_string()
            }
            None => continue, 
        };
        
        // Добавляем только непустые значения
        if !key.is_empty() && !value.is_empty() {
            metadata.insert(key, value);
        }
    }
    
    if metadata.is_empty() {
        return Err("Таблица пуста или не содержит данных".to_string());
    }
    
    Ok(metadata)
}

pub fn parse_bookvoed_book(isbn: &str) -> Result<BookParse, String> {
    let catalog_page = get_catalog_page(isbn)?;
    let (name, url_book) = parse_catalog_page(&catalog_page)?;
    let book_page = get_book_page(&url_book)?;
    let url_img = parse_url_image(&book_page)?;
    let metadata = parse_metadata_table(&book_page)?;

    let author = metadata.get("Автор")
        .map(|s| s.to_string())
        .ok_or("Автор не найден.")?;



    Ok(
        BookParse {
                url_page: url_book,
                isbn: isbn.to_string(),
                title: name,
                author: author,
                publisher: metadata.get("Издательство").map(|s| s.to_string()),
                series: metadata.get("Серия").map(|s| s.to_string()),
                binding: metadata.get("Переплет").map(|s| s.to_string()),
                page_count: metadata.get("Кол-во страниц").map(|s| s.to_string()),
                section: metadata.get("Раздел").map(|s| s.to_string()),
                cover_url: url_img
        }
    )
}