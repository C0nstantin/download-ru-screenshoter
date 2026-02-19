# План реализации скриншотера для download.ru

## Обзор
Превратить минимальный скриншотер в полнофункциональное приложение с:
1. Выделением области экрана
2. Редактированием скриншота (стрелки, прямоугольники, текст, размытие)
3. Автоматической загрузкой на download.ru

## Архитектура

### Rust Backend (src-tauri/src/lib.rs)
- Захват экрана через `screenshots` crate
- HTTP клиент для загрузки на download.ru (`reqwest`)
- Глобальные горячие клавиши (`tauri-plugin-global-shortcut`)
- Управление окнами (overlay для выделения, editor для редактирования)

### React Frontend (src/)
- **SelectionOverlay** - полноэкранный overlay для выделения области
- **ImageEditor** - редактор с инструментами рисования (Canvas API)
- **UploadStatus** - статус загрузки и ссылка на результат

## API download.ru

Из анализа кода download.ru:

**Endpoint:** `POST /fast_upload` или `POST /files`

**Авторизация:** OAuth 2.0 (Doorkeeper)
- Authorization: Bearer <access_token>
- Или User-Agent: Greenshot (создаёт анонимного пользователя и папку "Скриншоты")

**Request:**
```
POST /fast_upload
Authorization: Bearer <token>
Content-Type: multipart/form-data
X-Content-Type: image/png

file[data][original_filename]=screenshot_123.png
file[data][sha1]=<sha1>
file[data][size]=<size>
file[data][crc32]=<crc32>
file[shared]=true
```

**Response (JSON):**
```json
{
  "object": {
    "id": "abc123",
    "name": "screenshot_123.png",
    "secure_url": "https://download.ru/g/abc123?e=...&s=...",
    "shared": true,
    ...
  }
}
```

## Этапы реализации

### Этап 1: Базовая инфраструктура
- [ ] Добавить зависимости в Cargo.toml (reqwest, sha1, crc32fast, base64)
- [ ] Добавить tauri-plugin-global-shortcut
- [ ] Обновить capabilities (global-shortcut, http)
- [ ] Создать структуру конфига для OAuth токена

### Этап 2: Overlay для выделения области
- [ ] Создать новое окно selection_overlay (fullscreen, transparent, always-on-top)
- [ ] React компонент SelectionOverlay с Canvas
- [ ] Рисование прямоугольника выделения мышью
- [ ] Отправка координат в Rust через invoke
- [ ] Захват области (crop из полного скриншота)

### Этап 3: Редактор изображений
- [ ] Создать окно editor
- [ ] React компонент ImageEditor с HTML5 Canvas
- [ ] Инструменты:
  - Стрелки (линии с наконечниками)
  - Прямоугольники (обводка)
  - Текст (input overlay)
  - Размытие (canvas blur filter на области)
- [ ] Кнопки: Сохранить локально / Загрузить на download.ru / Отмена

### Этап 4: Загрузка на download.ru
- [ ] Rust функция для вычисления sha1, crc32
- [ ] Rust функция upload_to_download(image_bytes, filename)
- [ ] Сохранение токена в конфиге приложения
- [ ] UI для ввода/сохранения OAuth токена
- [ ] Копирование ссылки в буфер обмена после загрузки

### Этап 5: Глобальные горячие клавиши
- [ ] Регистрация Ctrl+Shift+S для запуска выделения
- [ ] Print Screen как альтернатива
- [ ] Возможность настройки через UI

### Этап 6: Полировка
- [ ] Нотификации системные
- [ ] История скриншотов (локально)
- [ ] Настройки (токен, хоткей, качество)

## Структура файлов

```
src-tauri/
  src/
    lib.rs          # Основная логика, setup, tray
    commands.rs     # Tauri commands (capture, upload, etc.)
    upload.rs       # Загрузка на download.ru
    config.rs       # Конфигурация приложения
  Cargo.toml        # Зависимости

src/
  App.tsx           # Роутер между режимами
  components/
    SelectionOverlay.tsx  # Выделение области
    ImageEditor.tsx       # Редактор
    Toolbar.tsx           # Панель инструментов
    Settings.tsx          # Настройки
  hooks/
    useCanvas.ts          # Хуки для работы с canvas
  utils/
    drawing.ts            # Функции рисования
```

## Зависимости

### Rust (Cargo.toml)
```toml
reqwest = { version = "0.11", features = ["json", "multipart"] }
tokio = { version = "1", features = ["full"] }
sha1 = "0.10"
crc32fast = "1.3"
base64 = "0.21"
tauri-plugin-global-shortcut = "2"
```

### Capabilities (capabilities/default.json)
```json
{
  "permissions": [
    "core:default",
    "core:window:allow-create",
    "core:window:allow-close",
    "core:window:allow-set-fullscreen",
    "core:window:allow-set-decorations",
    "core:window:allow-set-always-on-top",
    "global-shortcut:allow-register",
    "global-shortcut:allow-unregister"
  ]
}
```

## Вопросы к пользователю
1. Нужен ли OAuth flow в приложении или будет использоваться готовый токен?
2. Какие инструменты редактирования приоритетны?
3. Нужна ли история скриншотов?
