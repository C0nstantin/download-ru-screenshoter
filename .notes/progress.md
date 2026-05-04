# Progress

## 2026-05-04 — Linux self-diagnostic (подзадача 1)

**Status**: done by opencode (qwen3-coder), reviewed.

**What was done**:
- `commands/settings.rs::DiagnosticReport` расширен 4 Linux-полями за `#[cfg(target_os = "linux")]`:
  - `gnome_shell_version: Option<String>`
  - `has_ayatana_appindicator: bool`
  - `gnome_extensions: Vec<String>`
  - `ubuntu_appindicators_enabled: bool`
- В `run_diagnostics` добавлен Linux-блок: вызовы `gnome-shell --version`, `ldconfig -p | grep ayatana-appindicator`, `gsettings get org.gnome.shell enabled-extensions`.
- Helper `parse_extensions_list` под `cfg(target_os = "linux")`.
- `cargo check` зелёный, `clippy` без warnings в `settings.rs`.

**Known minor issues** (TODO follow-up):
- `gnome_extensions: Vec<String>` — нельзя отличить «gsettings упало» от «пустой список». Перевести в `Option<Vec<String>>`.
- `parse_extensions_list` не понимает `@as []` (gsettings формат для пустых).
- Пять `tracing::info!` подряд избыточны — оставить один итоговый со структурированными полями.

**Не сделано** (вне scope этой подзадачи):
- Auto-install GNOME extension не реализован.
- Tray fallback на legacy X11 SNI не сделан.

## 2026-05-04 — Linux Health Check UI (подзадача 2)

**Status**: done by opencode.

**What was done**:
- `DiagnosticReport` тип добавлен в `MainPage.tsx`, заменяет `Record<string, unknown>`.
- Диагностика загружается при монтировании страницы через `useEffect`.
- Новая секция "Linux Health Check" перед "Copy Diagnostics", видна только на Linux:
  - GNOME Shell версия (серый info)
  - Ayatana appindicator — зелёный/красный + подсказка для установки
  - GNOME tray extension — зелёный/красный + подсказка
  - Счётчик включённых расширений GNOME
- Кнопка "Copy Diagnostics" теперь использует кэшированный report из state.
- Все тексты переведены через i18n (ru + en).
- `npx tsc --noEmit` чистый.

## 2026-05-04 — SNI detection + adaptive launch (подзадача 3)

**Status**: done.

**What was done**:
- Создан `src-tauri/src/linux_env.rs` с функцией `is_sni_watcher_available()` — вызывает `dbus-send` для проверки `org.kde.StatusNotifierWatcher` на session bus.
- В `lib.rs` подключён модуль за `#[cfg(target_os = "linux")]`.
- В `setup` блоке после создания tray добавлена Linux-логика: если SNI watcher недоступен — показываем главное окно и отправляем notification; если доступен — tray-only режим.
- `cargo check` — зелёный. Clippy: 0 новых ошибок (все ошибки pre-existing).

## 2026-05-04 — Wayland screenshot portal (подзадача 4a)

**Status**: done (compiles, not yet integrated).

**What was done**:
- `ashpd = "0.13"` добавлен в `[target.'cfg(target_os = "linux")'.dependencies]` с features `["screenshot"]` (ver. 0.13.10 — latest stable).
- Создан `src-tauri/src/screen_capture_portal.rs` с `capture_via_portal()` — async функция, вызывает `Screenshot::request().interactive(true).modal(true).send()`, парсит `file://` URI в путь, читает PNG-байты.
- Модуль подключен в `lib.rs` за `#[cfg(target_os = "linux")]`.
- `cargo check` — зелёный (новые crate-deps скомпилировались на cold cache).

**Known minor issues** (TODO):
- `percent_decode_str` дублирует существующий `percent_decode` в `lib.rs` (там приватный). При интеграции в 4b — экспортировать общий helper в утилитный модуль.
- Функция `capture_via_portal()` пока никем не вызывается (по плану 4b).

## Подзадача 4b — интеграция Wayland portal в capture_fullscreen

**Status**: done — Wayland ветка (XDG_SESSION_TYPE == "wayland") добавлена в начало `capture_fullscreen` в `screenshot.rs`. На Linux+Wayland вызывает `capture_via_portal()`, на X11/macOS/Windows — существующий код без изменений. `cargo check` зелёный.

## Подзадача 7 — cleanup: убрать дубликат percent_decode

**Status**: done.
- `lib.rs:344`: `fn percent_decode` → `pub fn percent_decode` (экспортирована из crate).
- `screen_capture_portal.rs`: удалена локальная `percent_decode_str` (-16 строк), вызов заменён на `crate::percent_decode`.
- `cargo check` — зелёный, 0 новых предупреждений.

## Подзадача 5a — ScreenCast portal session

**Status**: done (compiles, not yet integrated).

**What was done**:
- `Cargo.toml`: добавлен `"screencast"` в features `ashpd`.
- Создан `src-tauri/src/screencast_portal.rs` с `ScreencastSession` — асинхронная обёртка над `ashpd::desktop::screencast::Screencast`.
- `ScreencastSession::start()` выполняет полный цикл: `create_session` → `select_sources` (monitor/window, cursor embedded, single source) → `start` (показывает portal picker) → `open_pipe_wire_remote` → возвращает `ScreencastSession { proxy, session, pipewire_fd, stream_node_id }`.
- `ScreencastSession::close()` закрывает portal session.
- API сверен с `ashpd` 0.13 source code через context7/docs.rs:
  - `create_session(Default::default())` — принимает `CreateSessionOptions`
  - `select_sources` через `SelectSourcesOptions` builder (не raw params)
  - `start(&session, None, StartCastOptions::default()).await?.response()?`
  - `open_pipe_wire_remote(&session, OpenPipeWireRemoteOptions::default())`
  - `stream.pipe_wire_node_id()` — `u32`
- `lib.rs`: добавлен `mod screencast_portal` за `#[cfg(target_os = "linux")]`.
- `cargo check` — зелёный (все `dead_code` warnings pre-existing).

## Подзадача 5b — ffmpeg recording wrapper

**Status**: done (compiles, not yet integrated).

**What was done**:
- Создан `src-tauri/src/screencast_record.rs` с `start_ffmpeg_recording(node_id, output_path)` и `stop_ffmpeg_recording(child)`.
- `start_ffmpeg_recording`: спавнит `ffmpeg -f pipewire -i <node_id> -c:v libx264 -preset ultrafast -pix_fmt yuv420p -y <output>`.
- `stop_ffmpeg_recording`: шлёт `kill -INT` для graceful finalization MP4, затем `child.wait()`.
- Оба используют `tracing::info!` для start/stop.
- Подключён в `lib.rs` за `#[cfg(target_os = "linux")]`.
- `cargo check` — зелёный. No new deps. No `unsafe`.

