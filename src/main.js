// Извлекаем необходимые утилиты из глобального Tauri v2 контекста
const { invoke } = window.__TAURI__.core;
const { convertFileSrc } = window.__TAURI__.core;

// Храним базовый путь к приложению, если потребуется
let appDataDirUrl = "";

// При запуске приложения загружаем все книги
document.addEventListener('DOMContentLoaded', () => {
  loadAllBooks();
  
  // Живой поиск
  document.getElementById('search-input').addEventListener('input', (e) => {
    const query = e.target.value;
    if (query.trim().length > 0) {
      searchBooks(query);
    } else {
      loadAllBooks();
    }
  });
});

// Переключение экранов SPA
function showScreen(screenId) {
  document.querySelectorAll('.screen').forEach(s => s.classList.remove('active'));
  document.getElementById(screenId).classList.add('active');
  if (screenId === 'library-view') loadAllBooks();
}

// Показать все книги
async function loadAllBooks() {
  try {
    const books = await invoke('get_all_books');
    renderBooksGrid(books);
  } catch (err) {
    alert("Ошибка загрузки книг: " + err);
  }
}

// Поиск книг
async function searchBooks(query) {
  try {
    const books = await invoke('search_by_query_book', { query: query });
    renderBooksGrid(books);
  } catch (err) {
    console.error(err);
  }
}

// Отрендерить сетку карточек книг
function renderBooksGrid(books) {
  const grid = document.getElementById('books-grid');
  grid.innerHTML = '';
  
  if (books.length === 0) {
    grid.innerHTML = '<p>Книг пока нет или ничего не найдено.</p>';
    return;
  }

  books.forEach(book => {
    const card = document.createElement('div');
    card.className = 'book-card';
    card.onclick = () => openBookDetails(book.id);

    // Конвертируем абсолютный системный путь из Rust для тега <img>
    const imgSrc = book.cover_url ? convertFileSrc(book.cover_url) : 'icons/icon.png';

    card.innerHTML = `
      <img src="${imgSrc}" class="book-cover" alt="${book.title}" onerror="this.src='icons/32x32.png'">
      <div class="book-info">
        <h3>${book.title}</h3>
        <p>${book.author}</p>
        <span class="status-badge">${book.status}</span>
      </div>
    `;
    grid.appendChild(card);
  });
}

// Парсинг книги по ISBN через Буквоед (Команда Rust)
async function parseBookByIsbn() {
  const isbn = document.getElementById('isbn-search').value.trim();
  if (!isbn) return alert("Введите ISBN");

  const loader = document.getElementById('parse-loader');
  const form = document.getElementById('book-form');
  
  loader.classList.remove('hidden');
  form.classList.add('hidden');

  try {
    const parsedData = await invoke('search_parse_book', { isbn: isbn });
    
    // Автозаполнение полей формы из результатов парсинга
    document.getElementById('form-isbn').value = parsedData.isbn || isbn;
    document.getElementById('form-title').value = parsedData.title || '';
    document.getElementById('form-author').value = parsedData.author || '';
    document.getElementById('form-publisher').value = parsedData.publisher || '';
    document.getElementById('form-series').value = parsedData.series || '';
    document.getElementById('form-binding').value = parsedData.binding || '';
    document.getElementById('form-pages').value = parsedData.page_count || '';
    document.getElementById('form-section').value = parsedData.section || '';
    document.getElementById('form-cover').value = parsedData.cover_url || '';

    loader.classList.add('hidden');
    form.classList.remove('hidden'); // Показываем форму для подтверждения/редактирования
  } catch (err) {
    loader.classList.add('hidden');
    alert("Ошибка парсинга: " + err);
  }
}

// Отправка формы (сохранение книги через маппинг rename_all = "snake_case")
async function submitBook(event) {
  event.preventDefault();

  const payload = {
    isbn: document.getElementById('form-isbn').value,
    title: document.getElementById('form-title').value,
    author: document.getElementById('form-author').value,
    status: document.getElementById('form-status').value,
    cover_url: document.getElementById('form-cover').value,
    publisher: document.getElementById('form-publisher').value || null,
    series: document.getElementById('form-series').value || null,
    binding: document.getElementById('form-binding').value || null,
    page_count: document.getElementById('form-pages').value || null,
    section: document.getElementById('form-section').value || null
  };

  try {
    // Вызываем команду insert_book
    const message = await invoke('insert_book', payload);
    alert(message);
    document.getElementById('book-form').reset();
    form.classList.add('hidden');
    showScreen('library-view');
  } catch (err) {
    alert("Ошибка сохранения: " + err);
  }
}

// Детальный просмотр книги в модальном окне
async function openBookDetails(bookId) {
  try {
    const book = await invoke('get_id_book', { id: bookId });
    if (!book) return;

    const modal = document.getElementById('book-modal');
    const detailsContainer = document.getElementById('modal-book-details');
    
    const imgSrc = book.cover_url ? convertFileSrc(book.cover_url) : 'icons/icon.png';

    detailsContainer.innerHTML = `
      <div class="modal-detailed">
        <img src="${imgSrc}" alt="${book.title}" onerror="this.src='icons/32x32.png'">
        <div>
          <h2>${book.title}</h2>
          <p><strong>Автор:</strong> ${book.author}</p>
          <p><span class="status-badge">${book.status}</span></p>
          
          <div class="details-list">
            <p><strong>ISBN:</strong> ${book.isbn}</p>
            <p><strong>Издательство:</strong> ${book.publisher || '—'}</p>
            <p><strong>Серия:</strong> ${book.series || '—'}</p>
            <p><strong>Переплет:</strong> ${book.binding || '—'}</p>
            <p><strong>Страниц:</strong> ${book.page_count || '—'}</p>
            <p><strong>Раздел:</strong> ${book.section || '—'}</p>
          </div>
        </div>
      </div>
    `;

    modal.style.display = "flex";
  } catch (err) {
    alert("Не удалось загрузить данные книги: " + err);
  }
}

function closeModal() {
  document.getElementById('book-modal').style.display = "none";
}
