use std::collections::HashMap;
use std::fs::File;
use std::io::copy;

use reqwest;
use url::Url;
use scraper::{Html, Selector};

use crate::structs::*;

const DOMEN: &str = "https://bookvoed.ru";
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64; rv:150.0) Gecko/20100101 Firefox/150.0";

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
    drop(encoder); // Освобождаем изменяемое заимствование

    println!("Итоговая ссылка:\n{}", url);
    Ok(url.to_string())
}

pub fn download_image(state: &tauri::State<'_, AppState>, url: &str, isbn: &str) -> Result<String, String> {
    let output_path = state.cover_path.join(format!("{}.jpg", isbn));
    let url = replace_params_url(url)?;
    // Отправляем GET-запрос по ссылке
    let client = reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| e.to_string())?;

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
    let client = reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!("{}/search?q={}", DOMEN, isbn);
    let response = client
        .get(&url)
        .send()
        .map_err(|e| e.to_string())?;

    let html = response.text().map_err(|e| e.to_string())?;

    Ok(html)
}

fn parse_url_book(html: &str) -> Result<String, String> {
    let document = Html::parse_document(&html);

    let url_sel = Selector::parse("a.product-description__link")
        .map_err(|e| e.to_string())?;

    let element = document
        .select(&url_sel)
        .next()
        .ok_or("Книга не найдена.")?;

    let url = element
        .value()
        .attr("href")
        .ok_or("Атрибут href не найден.")?;

    Ok(format!("{}{}", DOMEN, url))
}

fn parse_name_book(html: &str) -> Result<String, String> {
    let document = Html::parse_document(&html);

    let name_sel = Selector::parse("a.product-description__link")
        .map_err(|e| e.to_string())?;

    let element = document
        .select(&name_sel)
        .next()
        .ok_or("Книга не найдена.")?;

    let name = element
        .text()
        .next()
        .ok_or("Название не было получено")?;

    Ok(format!("{}", name))
}

fn get_book_page(url: &str) -> Result<String, String> {
    let client = reqwest::blocking::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .get(url)
        .send()
        .map_err(|e| e.to_string())?;

    let html = response.text().map_err(|e| e.to_string())?;

    Ok(html)
}

// <table class="product-characteristics-full__table" data-v-c31755e2="">
// <tbody class="product-characteristics-full__tbody" data-v-c31755e2="">
// <!--[-->
// <tr class="product-characteristics-full__row" style="" data-v-c31755e2="">
// <th class="product-characteristics-full__cell-th" data-v-c31755e2="">Код</th>
// <td class="product-characteristics-full__cell-td" data-v-c31755e2="">
// <!--[-->
// 3136516 
// <div class="product-characteristics-full__copy" data-v-c31755e2="" aria-expanded="false">
// <button class="ui-utility-button ui-utility-button--size-s ui-utility-button--kind-square ui-utility-button--theme-dark" aria-label="кнопка крестик" tabindex="0"><!--[--><svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path d="M6.854 4.051C7.471 4.001 8.264 4 9.4 4h7.1a1 1 0 1 0 0-2H9.357C8.273 2 7.4 2 6.691 2.058c-.728.06-1.369.185-1.96.487A5 5 0 0 0 2.544 4.73c-.302.592-.428 1.233-.487 1.961C2 7.4 2 8.273 2 9.357V16.5a1 1 0 1 0 2 0V9.4c0-1.137 0-1.929.051-2.546.05-.605.142-.953.276-1.216a3 3 0 0 1 1.311-1.311c.263-.134.611-.226 1.216-.276M17.838 5.5H9.662c-.527 0-.981 0-1.356.03-.395.033-.789.104-1.167.297a3 3 0 0 0-1.311 1.311c-.193.378-.264.772-.296 1.167-.031.375-.031.83-.031 1.356v8.178c0 .527 0 .982.03 1.356.033.395.104.789.297 1.167a3 3 0 0 0 1.311 1.311c.378.193.772.264 1.167.296.375.031.83.031 1.356.031h8.178c.527 0 .982 0 1.356-.03.395-.033.789-.104 1.167-.297a3 3 0 0 0 1.311-1.311c.193-.378.264-.772.296-1.167.031-.375.031-.83.031-1.357V9.662c0-.527 0-.981-.03-1.356-.033-.395-.104-.789-.297-1.167a3 3 0 0 0-1.311-1.311c-.378-.193-.772-.264-1.167-.296a18 18 0 0 0-1.357-.031m1.194 2.024c.272.022.372.06.422.085a1 1 0 0 1 .437.437c.025.05.063.15.085.422C20 8.75 20 9.123 20 9.7v8.1c0 .577 0 .949-.024 1.232-.022.272-.06.372-.085.422a1 1 0 0 1-.437.437c-.05.025-.15.063-.422.085C18.75 20 18.377 20 17.8 20H9.7c-.577 0-.949 0-1.232-.024-.272-.022-.373-.06-.422-.085a1 1 0 0 1-.437-.437c-.025-.05-.063-.15-.085-.422C7.5 18.75 7.5 18.377 7.5 17.8V9.7c0-.577 0-.949.024-1.232.022-.272.06-.373.085-.422a1 1 0 0 1 .437-.437c.05-.025.15-.063.422-.085C8.75 7.5 9.123 7.5 9.7 7.5h8.1c.577 0 .949 0 1.232.024"></path></svg>
// <!--]-->
// </button>
// </div><!--]--></td></tr><tr class="product-characteristics-full__row" style="" data-v-c31755e2=""><th class="product-characteristics-full__cell-th" data-v-c31755e2="">Издательство</th><td class="product-characteristics-full__cell-td" data-v-c31755e2=""><ul class="ui-comma-separated-links--one ui-comma-separated-links product-characteristics-full__link-list" data-v-c31755e2=""><!--[--><li class="ui-comma-separated-links__list-item"><a href="/brand/azbuka-116515" class="ui-link ui-link__color-scheme--one ui-link__border ui-link__border--solid ui-comma-separated-links__author base-link" inline="false" isselfhrefscrolltop="false"><!--[--><!--[--><span class="ui-comma-separated-links__tag">Азбука</span><!--]--><!--]--></a></li><!--]--></ul></td></tr><tr class="product-characteristics-full__row" style="" data-v-c31755e2=""><th class="product-characteristics-full__cell-th" data-v-c31755e2="">Серия</th><td class="product-characteristics-full__cell-td" data-v-c31755e2=""><a href="/serie/azbuka-bestseller-21203" class="ui-link ui-link__color-scheme--one ui-link__border ui-link__border--solid product-characteristics-full__link base-link" inline="false" data-v-c31755e2="" isselfhrefscrolltop="false"><!--[--><!--[-->Азбука-бестселлер<!--]--><!--]--></a></td></tr><tr class="product-characteristics-full__row" style="" data-v-c31755e2=""><th class="product-characteristics-full__cell-th" data-v-c31755e2="">Автор</th><td class="product-characteristics-full__cell-td" data-v-c31755e2=""><ul class="ui-comma-separated-links--one ui-comma-separated-links product-characteristics-full__link-list" data-v-c31755e2=""><!--[--><li class="ui-comma-separated-links__list-item"><a href="/author/shoyom-anna-24337739" class="ui-link ui-link__color-scheme--one ui-link__border ui-link__border--solid ui-comma-separated-links__author base-link" inline="false" isselfhrefscrolltop="false"><!--[--><!--[--><span class="ui-comma-separated-links__tag">Анна Шойом</span><!--]--><!--]--></a></li><!--]--></ul></td></tr><tr class="product-characteristics-full__row" style="" data-v-c31755e2=""><th class="product-characteristics-full__cell-th" data-v-c31755e2="">Переводчик</th><td class="product-characteristics-full__cell-td" data-v-c31755e2=""><!--[-->Капустина Вероника Л. <!----><!--]--></td></tr><tr class="product-characteristics-full__row" style="" data-v-c31755e2=""><th class="product-characteristics-full__cell-th" data-v-c31755e2="">Переплет</th><td class="product-characteristics-full__cell-td" data-v-c31755e2=""><!--[-->Твёрдый переплёт <!----><!--]--></td></tr><tr class="product-characteristics-full__row" style="" data-v-c31755e2=""><th class="product-characteristics-full__cell-th" data-v-c31755e2="">Кол-во страниц</th><td class="product-characteristics-full__cell-td" data-v-c31755e2=""><!--[-->320 <!----><!--]--></td></tr><tr class="product-characteristics-full__row" style="" data-v-c31755e2=""><th class="product-characteristics-full__cell-th" data-v-c31755e2="">Год издания</th><td class="product-characteristics-full__cell-td" data-v-c31755e2=""><!--[-->2025 <!----><!--]--></td></tr><tr class="product-characteristics-full__row" style="" data-v-c31755e2=""><th class="product-characteristics-full__cell-th" data-v-c31755e2="">Тираж</th><td class="product-characteristics-full__cell-td" data-v-c31755e2=""><!--[-->4&nbsp;000 экз. <!----><!--]--></td></tr><tr class="product-characteristics-full__row" style="" data-v-c31755e2=""><th class="product-characteristics-full__cell-th" data-v-c31755e2="">ISBN</th><td class="product-characteristics-full__cell-td" data-v-c31755e2=""><!--[-->978-5-389-28774-7 <div class="product-characteristics-full__copy" data-v-c31755e2="" aria-expanded="false"><button class="ui-utility-button ui-utility-button--size-s ui-utility-button--kind-square ui-utility-button--theme-dark" aria-label="кнопка крестик" tabindex="0"><!--[--><svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path d="M6.854 4.051C7.471 4.001 8.264 4 9.4 4h7.1a1 1 0 1 0 0-2H9.357C8.273 2 7.4 2 6.691 2.058c-.728.06-1.369.185-1.96.487A5 5 0 0 0 2.544 4.73c-.302.592-.428 1.233-.487 1.961C2 7.4 2 8.273 2 9.357V16.5a1 1 0 1 0 2 0V9.4c0-1.137 0-1.929.051-2.546.05-.605.142-.953.276-1.216a3 3 0 0 1 1.311-1.311c.263-.134.611-.226 1.216-.276M17.838 5.5H9.662c-.527 0-.981 0-1.356.03-.395.033-.789.104-1.167.297a3 3 0 0 0-1.311 1.311c-.193.378-.264.772-.296 1.167-.031.375-.031.83-.031 1.356v8.178c0 .527 0 .982.03 1.356.033.395.104.789.297 1.167a3 3 0 0 0 1.311 1.311c.378.193.772.264 1.167.296.375.031.83.031 1.356.031h8.178c.527 0 .982 0 1.356-.03.395-.033.789-.104 1.167-.297a3 3 0 0 0 1.311-1.311c.193-.378.264-.772.296-1.167.031-.375.031-.83.031-1.357V9.662c0-.527 0-.981-.03-1.356-.033-.395-.104-.789-.297-1.167a3 3 0 0 0-1.311-1.311c-.378-.193-.772-.264-1.167-.296a18 18 0 0 0-1.357-.031m1.194 2.024c.272.022.372.06.422.085a1 1 0 0 1 .437.437c.025.05.063.15.085.422C20 8.75 20 9.123 20 9.7v8.1c0 .577 0 .949-.024 1.232-.022.272-.06.372-.085.422a1 1 0 0 1-.437.437c-.05.025-.15.063-.422.085C18.75 20 18.377 20 17.8 20H9.7c-.577 0-.949 0-1.232-.024-.272-.022-.373-.06-.422-.085a1 1 0 0 1-.437-.437c-.025-.05-.063-.15-.085-.422C7.5 18.75 7.5 18.377 7.5 17.8V9.7c0-.577 0-.949.024-1.232.022-.272.06-.373.085-.422a1 1 0 0 1 .437-.437c.05-.025.15-.063.422-.085C8.75 7.5 9.123 7.5 9.7 7.5h8.1c.577 0 .949 0 1.232.024"></path></svg><!--]--></button></div><!--]--></td></tr><tr class="product-characteristics-full__row" style="" data-v-c31755e2=""><th class="product-characteristics-full__cell-th" data-v-c31755e2="">Раздел</th><td class="product-characteristics-full__cell-td" data-v-c31755e2=""><a href="/catalog/sovremennaya-zarubezhnaya-proza-110038" class="ui-link ui-link__color-scheme--one ui-link__border ui-link__border--solid product-characteristics-full__link base-link" inline="false" data-v-c31755e2="" isselfhrefscrolltop="false"><!--[--><!--[-->Современная зарубежная проза<!--]--><!--]--></a></td></tr><tr class="product-characteristics-full__row" style="" data-v-c31755e2=""><th class="product-characteristics-full__cell-th" data-v-c31755e2="">Размеры</th><td class="product-characteristics-full__cell-td" data-v-c31755e2=""><!--[-->2.1 см × 12 см × 20 см <!----><!--]--></td></tr><tr class="product-characteristics-full__row" style="" data-v-c31755e2=""><th class="product-characteristics-full__cell-th" data-v-c31755e2="">Вес</th><td class="product-characteristics-full__cell-td" data-v-c31755e2=""><!--[-->0.34 кг <!----><!--]--></td></tr><!--]--></tbody></table>

// pub fn parse_metadata(html: String) -> Result<HashMap<String, String>, String> {
//     let document = Html::parse_document(&html);

//     let table_sel = Selector::parse("table.product-characteristics-full__table")
//         .map_err(|e| e.to_string())?;

//     let element = document
// }

// TODO: name="img", class_="product-preview__big-img"
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
    
    // 1. Находим таблицу
    let table_selector = Selector::parse("table.product-characteristics-full__table")
        .map_err(|e| format!("Ошибка селектора таблицы: {}", e))?;
    
    let table = document
        .select(&table_selector)
        .next()
        .ok_or_else(|| "Таблица с характеристиками не найдена".to_string())?;
    
    // 2. Находим все строки в таблице
    let row_selector = Selector::parse("tr.product-characteristics-full__row")
        .map_err(|e| format!("Ошибка селектора строк: {}", e))?;
    
    // 3. Селекторы для ячеек (th - заголовок, td - значение)
    let th_selector = Selector::parse("th.product-characteristics-full__cell-th")
        .map_err(|e| format!("Ошибка селектора th: {}", e))?;
    
    let td_selector = Selector::parse("td.product-characteristics-full__cell-td")
        .map_err(|e| format!("Ошибка селектора td: {}", e))?;
    
    let mut metadata = HashMap::new();
    
    // 4. Проходим по каждой строке
    for row in table.select(&row_selector) {
        // Получаем заголовок (th)
        let key = match row.select(&th_selector).next() {
            Some(th) => th.text().collect::<String>().trim().to_string(),
            None => continue, // пропускаем строки без заголовка
        };
        
        // Получаем значение (td)
        let value = match row.select(&td_selector).next() {
            Some(td) => {
                // Извлекаем текст, очищая от HTML-тегов и лишних пробелов
                td.text()
                    .collect::<String>()
                    .split_whitespace()  // разбиваем по пробелам
                    .collect::<Vec<_>>()
                    .join(" ")           // собираем обратно с одним пробелом
                    .trim()
                    .to_string()
            }
            None => continue, // пропускаем строки без значения
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
    let name = parse_name_book(&catalog_page)?;
    let url_book = parse_url_book(&catalog_page)?;
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