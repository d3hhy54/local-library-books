use base64::prelude::*;

use crate::structs::{AppState, Filters};


pub fn save_cover_from_data_url(
    state: &AppState,
    data_url: &str,
    isbn: &str,
) -> Result<String, String> {
    let parts: Vec<&str> = data_url.split(',').collect();
    if parts.len() != 2 {
        return Err("Неверный формат Data URL".to_string());
    }
    
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
        "jpg"
    };
    
    let base64_data = parts[1];
    let decoded = BASE64_STANDARD.decode(base64_data)
        .map_err(|e| format!("Ошибка декодирования base64: {}", e))?;
    
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


pub fn save_file_to_covers(state: tauri::State<'_, AppState>, file_path: &str, isbn: &str) -> Result<String, String> {
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

fn make_placeholders(count: usize) -> String {
    if count == 0 {
        return String::new();
    }
    
    let capacity = count + (count.saturating_sub(1) * 2) + 2;
    let mut result = String::with_capacity(capacity);
    
    result.push('(');
    for i in 0..count {
        if i > 0 {
            result.push_str(", ");
        }
        result.push('?');
    }
    result.push(')');
    
    result
}


pub fn from_filters_to_query_with_args<'a>(
    filters: &'a Filters
) -> (String, Vec<&'a String>) { 
    let mut parts = Vec::new();
    let mut args = Vec::new();

    macro_rules! add_filter {
        ($field:ident) => {
            add_filter!($field, stringify!($field));
        };
        ($field:ident, $sql_column:expr) => {
            if let Some(q) = filters.$field.as_ref().filter(|v| !v.is_empty()) {
                parts.push(format!("{} IN {}", $sql_column, make_placeholders(q.len())));
                args.extend(q);
            }
        };
    }

    add_filter!(status);
    add_filter!(publisher);
    add_filter!(series);
    add_filter!(binding);
    add_filter!(section);

    (parts.join(" AND "), args)
}
