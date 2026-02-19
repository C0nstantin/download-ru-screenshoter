# Download Screenshoter

Приложение для создания скриншотов с редактированием и автоматической загрузкой на [download.ru](https://download.ru).

Работает в трее — без окон не занимает место в Dock и Cmd+Tab.

## Установка

### macOS

1. Скачай `.dmg` из [Releases](../../releases)
2. Перетащи приложение в `/Applications`
3. При первом запуске: правый клик → "Открыть" (из-за Gatekeeper)
4. Выдай разрешение на запись экрана: **Системные настройки → Конфиденциальность → Запись экрана**

### Windows

1. Скачай `.msi` или `.exe` из [Releases](../../releases)
2. Запусти установщик
3. При запросе UAC — подтверди установку

### Linux

1. Скачай `.AppImage` или `.deb` из [Releases](../../releases)
2. AppImage: `chmod +x Download-Screenshoter.AppImage && ./Download-Screenshoter.AppImage`
3. deb: `sudo dpkg -i download-screenshoter.deb`

> На Linux может потребоваться `xdotool` для захвата окон.

---

## Использование

После запуска приложение живёт в трее.

| Действие | Хоткей (по умолчанию) |
|---|---|
| Скриншот области | `Ctrl+Shift+4` |
| Скриншот экрана | `Ctrl+Shift+3` |
| Скриншот окна | `Ctrl+Shift+Alt+3` |

Хоткеи можно переназначить в настройках (двойной клик по иконке в трее).

### Первый запуск

1. Открой настройки через трей
2. Нажми **"Войти через download.ru"** — откроется окно авторизации
3. Войди в аккаунт, окно закроется автоматически
4. Делай скриншоты — ссылка копируется в буфер после загрузки

---

## Разработка

### Требования

- [Node.js](https://nodejs.org/) 18+
- [Rust](https://rustup.rs/) 1.70+
- [Tauri CLI](https://tauri.app/start/prerequisites/)

На macOS дополнительно: Xcode Command Line Tools (`xcode-select --install`)

### Запуск

```bash
npm install
npm run tauri dev
```

### Сборка

```bash
npm run tauri build
```

Собранные файлы появятся в `src-tauri/target/release/bundle/`.

---

## TODO

- [ ] **Мульти-монитор (Linux/Windows)** — на macOS overlay правильно растягивается на все экраны, на других платформах нужен отдельный overlay на каждый монитор
- [ ] **Запись видео** — захват видео с экрана с последующей загрузкой
- [ ] **Иконка** — нужна монохромная иконка для трея macOS (см. [ICONS.md](ICONS.md))

---

## Стек

- **Tauri 2** + **Rust** — бэкенд, системные API
- **React 19** + **TypeScript** — интерфейс
- **Konva** — редактор аннотаций на canvas
- **Zustand** — состояние редактора
