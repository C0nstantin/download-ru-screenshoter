# Project rules for AI agents

## Project context
- **Stack**: Rust + Tauri 2 (бэкенд), React 19 + TypeScript (фронтенд), Konva + Zustand (UI редактора)
- **Goal**: Десктопное приложение для скриншотов и видео с загрузкой на download.ru
- **Status**: beta — production-ready на macOS/Windows, **Linux в активной доработке**
- **Текущий host**: Ubuntu/GNOME (Wayland), AMD Strix Halo iGPU

## Architecture
- Main entry: `src-tauri/src/main.rs` → `src-tauri/src/lib.rs::run()`
- Tray + меню + протокол `localfile://`: `src-tauri/src/lib.rs`
- Команды Tauri: `src-tauri/src/commands/{screenshot,recording,upload,hotkeys,settings}.rs`
- React фронт: `src/pages/` (MainPage, OverlayPage, EditorPage, VideoResultPage)
- Конфиг сборки: `src-tauri/Cargo.toml` (target-specific deps уже есть)
- Lint/format: `cargo fmt`, `cargo clippy --target-dir target/clippy -- -D warnings`, `npm run lint`

## Conventions
- Платформо-зависимый код **только через `cfg!`/`#[cfg(target_os = "...")]`**, общий код в основном теле функции.
- Tauri commands — `async` где возможно, чтобы не блокировать main thread (см. beta.13 фикс).
- Логирование через `tracing` (уже инициализирован в `init_logging`). На Linux не вводить `eprintln!`.
- Не плодить новые крейты: сначала посмотри есть ли нужная функциональность в уже подключённых (`tauri`, `tracing`, `dirs`, `display-info`).
- Commit-messages в стиле существующих: `Fix X`, `Implement Y`, `Bump version`.

## Don't
- **Не трогать macOS-specific код** (cocoa/objc, ActivationPolicy, screencapture pickers) — он работает в проде.
- **Не вводить новых зависимостей в `[dependencies]` без явного запроса** — добавляй в `[target.'cfg(target_os = "linux")'.dependencies]`.
- **Не использовать `unwrap()` в production-путях** — `?` или `let _ =` с логом.
- Не делать рефакторинг вне scope. Tray-логика большая — править только нужное.
- Не запускать `cargo build` / `cargo run` в этой директории без явной команды (долго компилируется).

## Linux-specific notes
- Tray использует `libayatana-appindicator3` (см. README troubleshooting).
- На современном GNOME (40+) tray-иконки требуют расширение `AppIndicator and KStatusNotifierItem Support`.
- Скриншоты на Wayland: крейт `screenshots = "0.8"` уже подключён, но **не использует XDG portal** — на Wayland без X11-fallback может падать.
- Запись видео на Linux — TODO (см. README), не наша текущая задача.

## Notes directory
Persistent context lives in `.notes/`:
- `.notes/task.md` — текущая активная задача с приёмочными критериями
- `.notes/decisions.md` — архитектурные решения с обоснованием
- `.notes/progress.md` — что сделано / что не работает

Read these at session start. Append findings there instead of holding in chat context.
