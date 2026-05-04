# Текущая задача: SNI detection + adaptive launch (подзадача 3)

> Подзадачи 1 (Linux fields в `run_diagnostics`) и 2 (UI Linux Health Check) уже сделаны. См. `.notes/progress.md`.

## Контекст и мотивация

Сейчас приложение всегда стартует со скрытым главным окном (`visible: false` в `tauri.conf.json`) и сидит в трее. На macOS это работает идеально. На Linux — это работает **только если в desktop окружении есть system tray** (StatusNotifierItem/AppIndicator).

Если SNI watcher отсутствует (например, GNOME 40+ без расширения AppIndicator, Hyprland/Sway без waybar c tray-модулем) — юзер вообще не понимает что приложение запустилось: окна нет, иконки нет, ничего не происходит.

**Цель**: при старте на Linux детектировать наличие SNI watcher и:
- **Если есть** → текущее поведение (tray-only, окно скрыто)
- **Если нет** → показать главное окно + бросить notification «Приложение запущено в фоне, хоткей: ...»

## Что нужно сделать

### 1. Новый Rust-модуль `src-tauri/src/linux_env.rs`

Создать публичный модуль (только для Linux — содержимое за `#![cfg(target_os = "linux")]` или весь `mod` подключать через `cfg`).

Функция:
```rust
/// Проверяет через DBus наличие StatusNotifierWatcher.
/// Возвращает true если watcher зарегистрирован в session bus.
pub fn is_sni_watcher_available() -> bool { ... }
```

**Реализация**: вызвать через `std::process::Command`:
```
dbus-send --session --dest=org.freedesktop.DBus --type=method_call --print-reply \
  /org/freedesktop/DBus org.freedesktop.DBus.NameHasOwner \
  string:org.kde.StatusNotifierWatcher
```

В stdout ищем `boolean true` — true. Иначе false. Если `dbus-send` не нашёлся (`Command` упал) — возвращаем false (treat as "нет SNI").

Логировать `tracing::info!` с результатом.

### 2. Подключить модуль в `lib.rs`

В начале `lib.rs` добавить:
```rust
#[cfg(target_os = "linux")]
mod linux_env;
```

### 3. В `setup` блоке `lib.rs::run()` добавить Linux-логику ПОСЛЕ создания tray

После `tracing::info!("tray icon created successfully");` (строка ~229) и **перед** регистрацией хоткеев — добавить:

```rust
#[cfg(target_os = "linux")]
{
    if !linux_env::is_sni_watcher_available() {
        tracing::warn!("SNI watcher not available — falling back to visible window mode");
        // Показать главное окно сразу
        show_settings_window(&handle);
        // Бросить notification
        use tauri_plugin_notification::NotificationExt;
        let _ = handle.notification().builder()
            .title("DownloadRu Screenshoter")
            .body("Приложение запущено. Хоткей Ctrl+Shift+4 — скриншот области.")
            .show();
    } else {
        tracing::info!("SNI watcher available — running in tray-only mode");
    }
}
```

Использовать существующий `show_settings_window` хелпер (он уже есть в `lib.rs`).

### 4. Не трогать i18n для notification

Для первой итерации **захардкодить** текст notification на русском — i18n notification сделаем отдельной задачей (потребует читать локаль из state на момент запуска, что сложнее). Просто комментарий `// TODO: i18n` рядом.

## Приёмочные критерии

- `cargo check --manifest-path src-tauri/Cargo.toml` — зелёный.
- `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` — без ошибок в **новых** файлах (`linux_env.rs`) и в **новом коде** в `lib.rs`. Существующие warnings в других файлах не трогать.
- На macOS/Windows: код нового модуля не должен компилироваться (`cfg(target_os = "linux")`), поведение `setup` не должно поменяться.
- Новых крейтов не добавлять — только `std::process::Command` и существующие зависимости (`tauri-plugin-notification` уже подключён).

## Definition of done

- 2 файла изменены: `src-tauri/src/linux_env.rs` (новый, ~30-50 строк), `src-tauri/src/lib.rs` (+5-15 строк).
- `cargo check` запущен и зелёный.
- В `.notes/progress.md` дописать секцию "## Подзадача 3" одной-двумя строками.

## Что НЕ делать в этой задаче

- Не реализовывать i18n для notification (отдельная задача).
- Не делать tray fallback (нет SNI → не пытаться показать tray вообще; tray создаётся как сейчас, просто игнорируется системой).
- Не трогать macOS-код (cocoa/objc, ActivationPolicy).
- Не править Windows-код.
- Не менять `tauri.conf.json` (`visible: false` оставить как есть).
- Не менять Frontend (`src/`).
- Не вводить `zbus` или другие DBus-крейты — `dbus-send` достаточно для первой итерации.
