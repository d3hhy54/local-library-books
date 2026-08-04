// ==========================================
// Глобальное состояние
// ==========================================
let currentQuery = "";
let selectedFilters = {
  status: [],
  publisher: [],
  series: [],
  binding: [],
  section: []
};
let limit = 30;
let offset = 0;
let isLoading = false;
let hasMore = true;

// ==========================================
// DOM элементы
// ==========================================
const booksGrid = document.getElementById("books-grid");
const searchInput = document.getElementById("search-input");
const dynamicFiltersContainer = document.getElementById("dynamic-filters");
const loadingTrigger = document.getElementById("loading-trigger");

// ==========================================
// Tauri API (с проверкой доступности)
// ==========================================
const tauriInvoke = window.__TAURI__?.core?.invoke;
const tauriConvertFileSrc = window.__TAURI__?.core?.convertFileSrc;

if (!tauriInvoke) {
  console.warn("⚠️ Tauri API не обнаружен. Приложение работает в режиме демонстрации.");
}

// ==========================================
// Утилиты
// ==========================================

/**
 * Преобразование пути к обложке в безопасный URL
 */
function formatCoverUrl(url) {
  if (!url) {
    return 'data:image/svg+xml,' + encodeURIComponent(
      '<svg xmlns="http://www.w3.org/2000/svg" width="200" height="300" fill="%2311111b">' +
      '<rect width="200" height="300"/>' +
      '<text x="100" y="150" text-anchor="middle" fill="%237f849c" font-size="14">Нет обложки</text>' +
      '</svg>'
    );
  }

  // Если это уже HTTP(S) или data URL — возвращаем как есть
  if (url.startsWith("http://") || url.startsWith("https://") || url.startsWith("data:")) {
    return url;
  }

  // Если доступен Tauri convertFileSrc — используем его
  if (tauriConvertFileSrc) {
    try {
      return tauriConvertFileSrc(url);
    } catch (e) {
      console.error("Ошибка convertFileSrc:", e);
    }
  }

  // Фоллбек: показываем placeholder
  return 'data:image/svg+xml,' + encodeURIComponent(
    '<svg xmlns="http://www.w3.org/2000/svg" width="200" height="300" fill="%2311111b">' +
    '<rect width="200" height="300"/>' +
    '<text x="100" y="150" text-anchor="middle" fill="%23f38ba8" font-size="12">Ошибка загрузки</text>' +
    '</svg>'
  );
}

/**
 * Экранирование HTML
 */
function escapeHtml(str) {
  if (!str && str !== 0) return '';
  const div = document.createElement('div');
  div.textContent = String(str);
  return div.innerHTML;
}

/**
 * Валидация ISBN
 */
function isValidIsbn(isbn) {
  return /^[\d-]{10,17}$/.test(isbn);
}

// ==========================================
// Инициализация
// ==========================================
document.addEventListener("DOMContentLoaded", async () => {
  if (!tauriInvoke) {
    booksGrid.innerHTML = '<div class="empty-state">⚠️ Tauri API не доступен. Запустите приложение через Tauri.</div>';
    loadingTrigger.style.display = 'none';
    return;
  }

  await initFilters();
  await loadBooks(true);
  setupIntersectionObserver();
  setupEventListeners();
});

// ==========================================
// Загрузка параметров фильтров
// ==========================================
async function initFilters() {
  try {
    const params = await tauriInvoke("get_filters_params");
    renderFilterGroups(params);
  } catch (err) {
    console.error("Ошибка получения фильтров:", err);
  }
}

/**
 * Отрисовка групп фильтров
 */
function renderFilterGroups(params) {
  if (!params) return;
  
  dynamicFiltersContainer.innerHTML = "";

  const groups = [
    { key: "status", label: "Статус" },
    { key: "publisher", label: "Издательство" },
    { key: "series", label: "Серия" },
    { key: "binding", label: "Переплёт" },
    { key: "section", label: "Раздел" }
  ];

  groups.forEach(({ key, label }) => {
    const values = params[key];
    if (!values || values.length === 0) return;

    const groupDiv = document.createElement("div");
    groupDiv.className = "filter-group";

    groupDiv.innerHTML = `
      <div class="filter-group-title">${label}</div>
      <div class="filter-options">
        ${values.map(val => `
          <label class="filter-item">
            <input type="checkbox" 
                   data-group="${key}" 
                   value="${escapeHtml(val)}"
                   ${selectedFilters[key].includes(val) ? 'checked' : ''}>
            <span>${escapeHtml(val)}</span>
          </label>
        `).join('')}
      </div>
    `;

    dynamicFiltersContainer.appendChild(groupDiv);
  });
}

// ==========================================
// Формирование payload для фильтров
// ==========================================

/**
 * Возвращает объект Filters или undefined
 */
function getActiveFiltersPayload() {
  const payload = {};
  let hasFilters = false;

  for (const [key, values] of Object.entries(selectedFilters)) {
    if (values && values.length > 0) {
      payload[key] = values;
      hasFilters = true;
    }
  }

  return hasFilters ? payload : undefined;
}

// ==========================================
// Загрузка книг
// ==========================================

/**
 * Основная функция загрузки книг
 * @param {boolean} reset - Сбросить offset и очистить сетку
 */
async function loadBooks(reset = false) {
  if (isLoading) return;
  if (!hasMore && !reset) return;

  isLoading = true;
  loadingTrigger.style.display = "block";

  if (reset) {
    offset = 0;
    hasMore = true;
    booksGrid.innerHTML = "";
  }

  try {
    const filtersPayload = getActiveFiltersPayload();
    let books = [];

    if (currentQuery.trim() === "") {
      // Обычная загрузка с пагинацией
      books = await tauriInvoke("get_all_books", {
        filters: filtersPayload || null,
        limit: limit,
        offset: offset
      });
    } else {
      // Поиск с пагинацией
      books = await tauriInvoke("search_by_query_book", {
        query: currentQuery.trim(),
        filters: filtersPayload || null,
        limit: limit,
        offset: offset
      });
    }

    if (books && books.length > 0) {
      renderBookCards(books);
      offset += limit;
      
      if (books.length < limit) {
        hasMore = false;
        loadingTrigger.textContent = "📚 Все книги загружены";
        setTimeout(() => {
          loadingTrigger.style.display = "none";
        }, 2000);
      }
    } else {
      hasMore = false;
      if (reset) {
        booksGrid.innerHTML = '<div class="empty-state">📖 Книги не найдены</div>';
      }
      loadingTrigger.style.display = "none";
    }
  } catch (err) {
    console.error("Ошибка при загрузке книг:", err);
    if (reset) {
      booksGrid.innerHTML = '<div class="empty-state">❌ Ошибка загрузки данных</div>';
    }
    hasMore = false;
    loadingTrigger.style.display = "none";
  } finally {
    isLoading = false;
  }
}

// ==========================================
// Отображение карточек книг
// ==========================================

/**
 * Добавляет карточки книг в сетку
 */
function renderBookCards(books) {
  if (!books || books.length === 0) return;

  const fragment = document.createDocumentFragment();

  books.forEach(book => {
    const card = document.createElement("div");
    card.className = "book-card";
    card.dataset.id = book.id;

    card.innerHTML = `
      <div class="card-img-wrapper">
        <img src="${formatCoverUrl(book.cover_url)}" 
             alt="${escapeHtml(book.title)}" 
             loading="lazy"
             onerror="this.src='data:image/svg+xml,${encodeURIComponent('<svg xmlns=%22http://www.w3.org/2000/svg%22 width=%22200%22 height=%22300%22 fill=%22%2311111b%22><rect width=%22200%22 height=%22300%22/><text x=%22100%22 y=%22150%22 text-anchor=%22middle%22 fill=%22%237f849c%22 font-size=%2212%22>Ошибка</text></svg>')}'">
      </div>
      <div class="card-info">
        <div>
          <div class="card-title" title="${escapeHtml(book.title)}">${escapeHtml(book.title)}</div>
          <div class="card-author">${escapeHtml(book.author)}</div>
        </div>
        <div class="card-status">${escapeHtml(book.status)}</div>
      </div>
    `;

    card.addEventListener("click", () => openDetailsModal(book.id));
    fragment.appendChild(card);
  });

  booksGrid.appendChild(fragment);
}

// ==========================================
// Модальное окно: Детали книги
// ==========================================

/**
 * Открывает модальное окно с полной информацией о книге
 */
async function openDetailsModal(id) {
  try {
    const book = await tauriInvoke("get_id_book", { id: parseInt(id) });
    if (!book) {
      alert("Книга не найдена");
      return;
    }

    // Заполняем данные
    document.getElementById("detail-cover-img").src = formatCoverUrl(book.cover_url);
    document.getElementById("detail-cover-img").onerror = function() {
      this.src = 'data:image/svg+xml,' + encodeURIComponent(
        '<svg xmlns="http://www.w3.org/2000/svg" width="200" height="300" fill="%2311111b">' +
        '<rect width="200" height="300"/>' +
        '<text x="100" y="150" text-anchor="middle" fill="%237f849c" font-size="14">Обложка не найдена</text>' +
        '</svg>'
      );
    };

    document.getElementById("detail-title").textContent = book.title || "Без названия";
    document.getElementById("detail-author").textContent = book.author || "Неизвестный автор";
    document.getElementById("detail-isbn").textContent = book.isbn || "—";
    document.getElementById("detail-status").textContent = book.status || "—";

    // Опциональные поля
    const optionalFields = [
      { field: "publisher", rowId: "row-publisher", cellId: "detail-publisher" },
      { field: "series", rowId: "row-series", cellId: "detail-series" },
      { field: "binding", rowId: "row-binding", cellId: "detail-binding" },
      { field: "page_count", rowId: "row-pages", cellId: "detail-pages" },
      { field: "section", rowId: "row-section", cellId: "detail-section" }
    ];

    optionalFields.forEach(({ field, rowId, cellId }) => {
      const row = document.getElementById(rowId);
      const cell = document.getElementById(cellId);
      
      if (book[field] !== null && book[field] !== undefined && book[field] !== "") {
        cell.textContent = book[field];
        row.style.display = "table-row";
      } else {
        row.style.display = "none";
      }
    });

    // Показываем модальное окно
    document.getElementById("book-details-modal").classList.add("active");
    document.body.style.overflow = "hidden";
  } catch (err) {
    console.error("Ошибка загрузки деталей книги:", err);
    alert("Не удалось загрузить информацию о книге");
  }
}

// ==========================================
// Парсинг по ISBN
// ==========================================

/**
 * Обработчик кнопки "Найти в сети"
 */
async function handleParseIsbn() {
  const isbnInput = document.getElementById("parse-isbn-input");
  const errorDiv = document.getElementById("parse-error");
  const parseBtn = document.getElementById("parse-btn");
  
  const isbn = isbnInput.value.trim();
  errorDiv.textContent = "";

  // Валидация
  if (!isbn) {
    errorDiv.textContent = "Введите ISBN";
    return;
  }
  if (!isValidIsbn(isbn)) {
    errorDiv.textContent = "Некорректный формат ISBN (допустимы цифры и дефисы, 10-17 символов)";
    return;
  }

  // Блокируем кнопку на время запроса
  parseBtn.disabled = true;
  parseBtn.textContent = "⏳ Поиск...";

  try {
    const parsedData = await tauriInvoke("search_parse_book", { isbn });
    
    // Заполняем форму
    const form = document.getElementById("add-book-form");
    const setFieldValue = (name, value) => {
      const field = form.elements[name];
      if (field && value) field.value = value;
    };

    setFieldValue("isbn", parsedData.isbn || isbn);
    setFieldValue("title", parsedData.title);
    setFieldValue("author", parsedData.author);
    setFieldValue("cover_url", parsedData.cover_url);
    setFieldValue("publisher", parsedData.publisher);
    setFieldValue("series", parsedData.series);
    setFieldValue("binding", parsedData.binding);
    setFieldValue("page_count", parsedData.page_count);
    setFieldValue("section", parsedData.section);

    errorDiv.textContent = "";
    errorDiv.style.color = "#a6e3a1"; // Зелёный цвет для успеха
    errorDiv.textContent = "✅ Данные успешно загружены!";
    setTimeout(() => {
      errorDiv.textContent = "";
      errorDiv.style.color = "";
    }, 3000);

  } catch (err) {
    errorDiv.textContent = typeof err === 'string' ? err : "Ошибка при поиске книги";
  } finally {
    parseBtn.disabled = false;
    parseBtn.textContent = "Найти в сети";
  }
}

// ==========================================
// Добавление книги
// ==========================================

/**
 * Обработчик отправки формы добавления книги
 */
async function handleAddBookSubmit(e) {
  e.preventDefault();
  
  const formData = new FormData(e.target);
  const submitBtn = e.target.querySelector('.submit-btn');

  // Собираем данные (ВАЖНО: имена полей в snake_case для Rust)
  const payload = {
    isbn: formData.get("isbn")?.trim(),
    title: formData.get("title")?.trim(),
    author: formData.get("author")?.trim(),
    status: formData.get("status")?.trim() || "не прочитано",
    coverUrl: formData.get("cover_url")?.trim(),  // Rust примет как cover_url
    publisher: formData.get("publisher")?.trim() || null,
    series: formData.get("series")?.trim() || null,
    binding: formData.get("binding")?.trim() || null,
    pageCount: formData.get("page_count")?.trim() || null,  // Rust примет как page_count
    section: formData.get("section")?.trim() || null
  };

  // Валидация
  if (!payload.isbn || !payload.title || !payload.author) {
    alert("Заполните обязательные поля: ISBN, Название, Автор");
    return;
  }

  if (!payload.coverUrl) {
    alert("Укажите ссылку или путь к обложке");
    return;
  }

  // Блокируем кнопку
  submitBtn.disabled = true;
  submitBtn.textContent = "⏳ Сохранение...";

  try {
    const result = await tauriInvoke("insert_book", payload);
    alert(result || "Книга успешно добавлена!");

    // Закрываем модалку и сбрасываем форму
    closeModal("add-book-modal");
    e.target.reset();
    document.getElementById("parse-isbn-input").value = "";
    document.getElementById("parse-error").textContent = "";

    // Обновляем фильтры и список книг
    await initFilters();
    await loadBooks(true);

  } catch (err) {
    console.error("Ошибка добавления книги:", err);
    alert("Ошибка: " + (typeof err === 'string' ? err : JSON.stringify(err)));
  } finally {
    submitBtn.disabled = false;
    submitBtn.textContent = "Сохранить в базу";
  }
}

// ==========================================
// Управление модальными окнами
// ==========================================

function closeModal(modalId) {
  const modal = document.getElementById(modalId);
  if (modal) {
    modal.classList.remove("active");
    document.body.style.overflow = "";
  }
}

function openModal(modalId) {
  const modal = document.getElementById(modalId);
  if (modal) {
    modal.classList.add("active");
    document.body.style.overflow = "hidden";
  }
}

// ==========================================
// Intersection Observer (бесконечный скролл)
// ==========================================

function setupIntersectionObserver() {
  const observer = new IntersectionObserver(
    (entries) => {
      entries.forEach(entry => {
        if (entry.isIntersecting && !isLoading && hasMore) {
          loadBooks(false);
        }
      });
    },
    {
      root: document.querySelector(".main-content"),
      rootMargin: "200px",
      threshold: 0.1
    }
  );

  observer.observe(loadingTrigger);
}

// ==========================================
// Все обработчики событий
// ==========================================

function setupEventListeners() {
  // Поиск с debounce
  let debounceTimer;
  searchInput.addEventListener("input", (e) => {
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      currentQuery = e.target.value;
      loadBooks(true);
    }, 400);
  });

  // Фильтры (делегирование событий)
  dynamicFiltersContainer.addEventListener("change", (e) => {
    if (e.target.tagName === "INPUT" && e.target.type === "checkbox") {
      const group = e.target.dataset.group;
      const value = e.target.value;

      if (!selectedFilters[group]) {
        selectedFilters[group] = [];
      }

      if (e.target.checked) {
        if (!selectedFilters[group].includes(value)) {
          selectedFilters[group].push(value);
        }
      } else {
        selectedFilters[group] = selectedFilters[group].filter(item => item !== value);
      }

      loadBooks(true);
    }
  });

  // Сброс фильтров
  document.getElementById("reset-filters-btn").addEventListener("click", () => {
    selectedFilters = {
      status: [],
      publisher: [],
      series: [],
      binding: [],
      section: []
    };

    // Снимаем все чекбоксы
    const checkboxes = dynamicFiltersContainer.querySelectorAll("input[type='checkbox']");
    checkboxes.forEach(cb => cb.checked = false);

    loadBooks(true);
  });

  // Открытие модалки добавления
  document.getElementById("open-add-modal-btn").addEventListener("click", () => {
    openModal("add-book-modal");
  });

  // Закрытие модалок (кнопки ✕)
  document.querySelectorAll(".close-btn").forEach(btn => {
    btn.addEventListener("click", () => {
      const modalId = btn.dataset.modal;
      closeModal(modalId);
    });
  });

  // Закрытие модалок по клику на фон
  window.addEventListener("click", (e) => {
    if (e.target.classList.contains("modal")) {
      closeModal(e.target.id);
    }
  });

  // Закрытие по Escape
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape") {
      document.querySelectorAll(".modal.active").forEach(modal => {
        closeModal(modal.id);
      });
    }
  });

  // Кнопка парсинга ISBN
  document.getElementById("parse-btn").addEventListener("click", handleParseIsbn);

  // Отправка формы добавления
  document.getElementById("add-book-form").addEventListener("submit", handleAddBookSubmit);
}