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
// Константы для SVG (вынесено для переиспользования)
// ==========================================
const PLACEHOLDER_SVG = 
  '<svg xmlns="http://www.w3.org/2000/svg" width="200" height="300" fill="%2311111b">' +
  '<rect width="200" height="300"/>' +
  '<text x="100" y="150" text-anchor="middle" fill="%237f849c" font-size="14">Нет обложки</text>' +
  '</svg>';

const ERROR_SVG = 
  '<svg xmlns="http://www.w3.org/2000/svg" width="200" height="300" fill="%2311111b">' +
  '<rect width="200" height="300"/>' +
  '<text x="100" y="150" text-anchor="middle" fill="%23f38ba8" font-size="12">Ошибка загрузки</text>' +
  '</svg>';

// ==========================================
// Утилиты
// ==========================================

/**
 * Преобразование пути к обложке в безопасный URL
 */
function formatCoverUrl(url) {
  if (!url) {
    return `data:image/svg+xml,${encodeURIComponent(PLACEHOLDER_SVG)}`;
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
  return `data:image/svg+xml,${encodeURIComponent(ERROR_SVG)}`;
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

  // Вешаем слушатель события ввода на весь контейнер фильтров
  if (dynamicFiltersContainer) {
    dynamicFiltersContainer.addEventListener("input", (event) => {
      if (event.target?.classList.contains("filter-search-input")) {
        const inputElement = event.target;
        const targetContainerId = inputElement.getAttribute("data-target");
        filterOptions(inputElement, targetContainerId);
      }
    });
  }
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
 * Отрисовка групп фильтров (по умолчанию все группы СВЕРНУТЫ)
 */
function renderFilterGroups(params) {
  if (!params) return;
  
  dynamicFiltersContainer.innerHTML = "";

  const groups = [
    { key: "status", label: "Статус", hasSearch: false },
    { key: "binding", label: "Переплёт", hasSearch: false },
    { key: "publisher", label: "Издательство", hasSearch: true },
    { key: "series", label: "Серия", hasSearch: true },
    { key: "section", label: "Раздел", hasSearch: true }
  ];

  params.status = ["Не прочитано", "В планах", "Читаю", "Прочитано", "Брошено", "Любимые"];

  groups.forEach(({ key, label, hasSearch }) => {
    const values = params[key];
    if (!values || values.length === 0) return;

    const groupDiv = document.createElement("div");
    groupDiv.className = "filter-group";

    const searchInputHtml = hasSearch 
      ? `<input type="text" 
            class="filter-search-input" 
            data-target="options-${key}" 
            placeholder="🔍 Поиск...">`
      : "";

    // ТАУРИ-ФИКС: Полностью убрали onclick="..." и onchange="..." из строки HTML
    groupDiv.innerHTML = `
      <div class="filter-group-header">
        <span class="filter-group-title">${label}</span>
        <span class="filter-toggle-arrow rotated">▼</span>
      </div>
      <div class="filter-group-content collapsed">
        ${searchInputHtml}
        <div id="options-${key}" class="filter-options">
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
      </div>
    `;

    // ТАУРИ-ФИКС: Находим заголовок внутри созданного элемента и вешаем клик через JS
    const header = groupDiv.querySelector(".filter-group-header");
    header.addEventListener("click", function() {
      toggleFilterGroup(this);
    });

    // ТАУРИ-ФИКС: Находим все чекбоксы в этой группе и вешаем событие изменения через JS
    const checkboxes = groupDiv.querySelectorAll(`input[data-group="${key}"]`);
    checkboxes.forEach(checkbox => {
      checkbox.addEventListener("change", function() {
        sortActiveFiltersTop(`options-${key}`);
      });
    });

    dynamicFiltersContainer.appendChild(groupDiv);
  });

  // Первичная сортировка активных элементов наверх при загрузке
  groups.forEach(({ key }) => {
    sortActiveFiltersTop(`options-${key}`);
  });

  // Установка display и повторная сортировка
  groups.forEach(({ key }) => {
    const container = document.getElementById(`options-${key}`);
    if (container) {
      const items = container.getElementsByClassName("filter-item");
      for (let i = 0; i < items.length; i++) {
        items[i].style.display = "flex";
      }
      sortActiveFiltersTop(`options-${key}`);
    }
  });
}


/**
 * Сортирует чекбоксы внутри фильтра: активные наверх, неактивные вниз.
 */
function sortActiveFiltersTop(containerId) {
  const container = document.getElementById(containerId);
  if (!container) return;

  const items = container.getElementsByClassName("filter-item");

  for (let i = 0; i < items.length; i++) {
    const item = items[i];
    const checkbox = item.querySelector("input[type='checkbox']");

    // FIX: Использовал тернарный оператор для краткости
    item.style.order = checkbox?.checked ? "1" : "2";
  }
}

/**
 * Поиск по фильтрам (регистронезависимый)
 */
function filterOptions(inputElement, targetContainerId) {
  const query = inputElement.value.toLowerCase().trim();
  const container = document.getElementById(targetContainerId);

  if (!container) return;
  const items = container.getElementsByClassName("filter-item");

  if (query === "") {
    for (let i = 0; i < items.length; i++) {
      items[i].classList.remove("hidden-by-search");
    }
    if (typeof sortActiveFiltersTop === "function") {
      sortActiveFiltersTop(targetContainerId); 
    }
    return;
  }

  for (let i = 0; i < items.length; i++) {
    const item = items[i];
    const span = item.querySelector("span");
    const text = span ? span.textContent.toLowerCase().trim() : "";

    item.style.order = "0";

    if (text.includes(query)) {
      item.classList.remove("hidden-by-search");
    } else {
      item.classList.add("hidden-by-search");
    }
  }
}

/**
 * Сворачивание/разворачивание группы фильтров
 */
function toggleFilterGroup(headerElement) {
  const content = headerElement.nextElementSibling;
  const arrow = headerElement.querySelector(".filter-toggle-arrow");

  if (content && arrow) {
    content.classList.toggle("collapsed");
    arrow.classList.toggle("rotated");
  }
}

// ==========================================
// Формирование payload для фильтров
// ==========================================

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
      books = await tauriInvoke("get_all_books", {
        filters: filtersPayload || null,
        limit: limit,
        offset: offset
      }) || []; // FIX: Добавлен fallback
    } else {
      books = await tauriInvoke("search_by_query_book", {
        query: currentQuery.trim(),
        filters: filtersPayload || null,
        limit: limit,
        offset: offset
      }) || []; // FIX: Добавлен fallback
    }

    if (books.length > 0) { // FIX: Убрал проверку books &&, т.к. теперь всегда массив
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
             onerror="this.src='data:image/svg+xml,${encodeURIComponent(ERROR_SVG)}'">
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

async function saveStatusInline() {
  const saveBtn = document.getElementById("save-status-btn");
  const statusSelect = document.getElementById("status-select");
  const statusCell = document.getElementById("detail-status");

  const bookId = parseInt(saveBtn.getAttribute("data-current-book-id"));
  const selectedStatus = statusSelect.value;
  
  if (!bookId) return;

  saveBtn.disabled = true;
  saveBtn.textContent = "⏳";
  statusSelect.disabled = true;

  try {
    await tauriInvoke("update_status_book", {
      status: selectedStatus,
      id: bookId
    });

    if (statusCell) {
      statusCell.textContent = selectedStatus;
    }

    showToastNotification(`✅ Статус изменен на "${selectedStatus}"`);
    toggleStatusEditMode(false);
  } catch (error) {
    console.error("Не удалось обновить статус:", error);
    alert("Ошибка при сохранении статуса в базу данных.");
  } finally {
    saveBtn.disabled = false;
    saveBtn.textContent = "Сохранить";
    statusSelect.disabled = false;
  }
}

async function openDetailsModal(id) {
  try {
    const book = await tauriInvoke("get_id_book", { id: parseInt(id) });
    if (!book) {
      alert("Книга не найдена");
      return;
    }

    const coverImg = document.getElementById("detail-cover-img");
    coverImg.src = formatCoverUrl(book.cover_url);
    // FIX: Добавлен обработчик ошибки загрузки
    coverImg.onerror = function() {
      this.src = `data:image/svg+xml,${encodeURIComponent(PLACEHOLDER_SVG)}`;
    };

    document.getElementById("detail-title").textContent = book.title || "Без названия";
    document.getElementById("detail-author").textContent = book.author || "Неизвестный автор";
    document.getElementById("detail-isbn").textContent = book.isbn || "—";
    document.getElementById("detail-status").textContent = book.status || "—";

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

    const saveBtn = document.getElementById("save-status-btn");
    if (saveBtn) {
      saveBtn.setAttribute("data-current-book-id", book.id);
    }

    toggleStatusEditMode(false);

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

function showToastNotification(message, clickCallback = null) {
  let container = document.getElementById('toast-container');
  if (!container) {
    container = document.createElement('div');
    container.id = 'toast-container';
    container.className = 'toast-container';
    document.body.appendChild(container);
  }

  const toast = document.createElement('div');
  toast.className = 'toast';
  toast.innerText = message;
  
  if (clickCallback) {
    toast.style.cursor = 'pointer'; 
    toast.onclick = function() {
      clickCallback();
      toast.remove();
    };
  }

  container.appendChild(toast);

  setTimeout(() => {
    if (toast.parentNode) {
      toast.remove();
    }
  }, 4000);
}

async function handleParseIsbn() {
  const isbnInput = document.getElementById("parse-isbn-input");
  const errorDiv = document.getElementById("parse-error");
  const parseBtn = document.getElementById("parse-btn");
  
  const isbn = isbnInput.value.trim();
  errorDiv.textContent = "";

  if (!isbn) {
    errorDiv.textContent = "Введите ISBN";
    return;
  }
  if (!isValidIsbn(isbn)) {
    errorDiv.textContent = "Некорректный формат ISBN (допустимы цифры и дефисы, 10-17 символов)";
    return;
  }

  parseBtn.disabled = true;
  parseBtn.textContent = "⏳ Поиск...";

  try {
    const parsedData = await tauriInvoke("search_parse_book", { isbn });
    
    const addBookModal = document.getElementById('add-book-modal');
    const isModalOpen = addBookModal?.classList.contains('active');

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

    if (isModalOpen) {
      errorDiv.style.color = "#a6e3a1"; 
      errorDiv.textContent = "✅ Данные успешно загружены!";
      setTimeout(() => {
        errorDiv.textContent = "";
        errorDiv.style.color = "";
      }, 3000);
    } else {
      const bookTitle = parsedData.title || "Новая книга";
      showToastNotification(
        `Книга "${bookTitle}" успешно спарсена в фоне!`, 
        () => openModal('add-book-modal')
      );
    }
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

async function handleAddBookSubmit(e) {
  e.preventDefault();
  
  const formData = new FormData(e.target);
  const submitBtn = e.target.querySelector('.submit-btn');

  const pageCountRaw = formData.get('page_count');

  let pageCount = null;
  if (pageCountRaw) {
    const digits = pageCountRaw.replace(/\D/g, '');
    if (digits) {
      pageCount = parseInt(digits, 10);
    } else {
      alert("Количество страниц должно быть числом");
      return;
    }
  }

  const payload = {
    isbn: formData.get("isbn")?.trim(),
    title: formData.get("title")?.trim(),
    author: formData.get("author")?.trim(),
    status: formData.get("status")?.trim() || "не прочитано",
    coverUrl: formData.get("cover_url")?.trim(),
    publisher: formData.get("publisher")?.trim() || null,
    series: formData.get("series")?.trim() || null,
    binding: formData.get("binding")?.trim() || null,
    pageCount: pageCount || null,
    section: formData.get("section")?.trim() || null
  };

  if (!payload.isbn || !payload.title || !payload.author) {
    alert("Заполните обязательные поля: ISBN, Название, Автор");
    return;
  }

  if (!payload.coverUrl) {
    alert("Укажите ссылку или путь к обложке");
    return;
  }

  submitBtn.disabled = true;
  submitBtn.textContent = "⏳ Сохранение...";

  try {
    const result = await tauriInvoke("insert_book", payload);
    alert(result || "Книга успешно добавлена!");

    closeModal("add-book-modal");
    e.target.reset();
    document.getElementById("parse-isbn-input").value = "";
    document.getElementById("parse-error").textContent = "";

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
  let debounceTimer;
  searchInput.addEventListener("input", (e) => {
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      currentQuery = e.target.value;
      loadBooks(true);
    }, 400);
  });

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

  document.getElementById("reset-filters-btn").addEventListener("click", () => {
    selectedFilters = {
      status: [],
      publisher: [],
      series: [],
      binding: [],
      section: []
    };

    const checkboxes = dynamicFiltersContainer.querySelectorAll("input[type='checkbox']");
    checkboxes.forEach(cb => cb.checked = false);

    const filterSearchInputs = dynamicFiltersContainer.querySelectorAll(".filter-search-input");
    filterSearchInputs.forEach(input => input.value = "");

    const filterItems = dynamicFiltersContainer.querySelectorAll(".filter-item");
    filterItems.forEach(item => {
      item.classList.remove("hidden-by-search");
      item.style.order = "0";
    });

    loadBooks(true);
  });

  document.getElementById("open-add-modal-btn").addEventListener("click", () => {
    openModal("add-book-modal");
  });

  document.querySelectorAll(".close-btn").forEach(btn => {
    btn.addEventListener("click", () => {
      const modalId = btn.dataset.modal;
      closeModal(modalId);
    });
  });

  window.addEventListener("click", (e) => {
    if (e.target.classList.contains("modal")) {
      closeModal(e.target.id);
    }
  });

  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape") {
      document.querySelectorAll(".modal.active").forEach(modal => {
        closeModal(modal.id);
      });
    }
  });

  document.getElementById("parse-btn").addEventListener("click", handleParseIsbn);
  document.getElementById("add-book-form").addEventListener("submit", handleAddBookSubmit);

  document.getElementById("btn-edit").addEventListener("click", () => {toggleStatusEditMode(true);});
  document.getElementById("save-status-btn").addEventListener("click", saveStatusInline);
}

// ==========================================
// Переключение режимов статуса
// ==========================================

function toggleStatusEditMode(isEdit) {
  const readModeDiv = document.getElementById("status-read-mode");
  const editModeDiv = document.getElementById("status-edit-mode");
  const currentStatus = document.getElementById("detail-status").textContent;

  if (isEdit) {
    readModeDiv.style.display = "none";
    editModeDiv.style.display = "flex";
    const statusSelect = document.getElementById("status-select");
    if (statusSelect) statusSelect.value = currentStatus;
  } else {
    readModeDiv.style.display = "flex";
    editModeDiv.style.display = "none";
  }
}