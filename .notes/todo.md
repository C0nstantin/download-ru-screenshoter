# Linux roadmap — что осталось

## Сделано в этой сессии (8 коммитов, см. git log)

- ✅ Linux self-diagnostic в `run_diagnostics` (Подзадача 1)
- ✅ Linux Health Check UI в MainPage с цветными индикаторами + i18n (2)
- ✅ SNI detection через DBus + adaptive launch (3)
- ✅ Wayland screenshot через `ashpd` portal (4a, 4b)
- ✅ ScreenCast portal session wrapper (5a)
- ✅ ffmpeg recorder для pipewire stream (5b)
- ✅ Полная интеграция Wayland video recording в `start_video_capture` (5c)
- ✅ Cleanup percent_decode helper (7)

## Осталось — в порядке приоритета

### High priority (UX-блокеры на Wayland)

#### TODO — Global Shortcuts portal (подзадача 6, разбита)

На Wayland `tauri-plugin-global-shortcut` не работает: нет глобальных хоткеев у unprivileged app. Решение — `org.freedesktop.portal.GlobalShortcuts` (xdg-desktop-portal 1.18+).

- [ ] **6a (lite)**: расширить `linux_env::is_global_shortcuts_portal_available()` через DBus — проверка по аналогии с `is_sni_watcher_available()`. Добавить поле в `DiagnosticReport` + строку в Health Check UI «Global hotkeys: portal supported / not supported».
- [ ] **6b (full)**: модуль `global_shortcuts_portal.rs` через `ashpd::desktop::global_shortcuts` — register session, listen for `Activated` events, mapping на наши hotkey IDs.
- [ ] **6c (full)**: ветка в `commands/hotkeys.rs` для Wayland — при `register_hotkeys` идти через portal вместо `tauri-plugin-global-shortcut`. UX caveat: пользователь должен сам забиндить hotkeys в System Settings → Keyboard (security ограничение portal).

### Medium priority (фичи)

- [ ] **5d**: region video recording на Linux — добавить overlay-flow как на macOS, передать coordinates в ffmpeg через `-filter:v "crop=W:H:X:Y"`.
- [ ] **5e**: window video recording на Linux — выбор окна сейчас невозможен через ScreenCast (portal сам показывает picker), но можно использовать `SourceType::Window` + ffmpeg запись.
- [ ] **i18n Linux notification** в `lib.rs::run()` setup — сейчас текст notification «Приложение запущено...» захардкожен. Прочитать локаль из stored config до запуска tray.
- [ ] **set_tray_recording_mode auto-recovery on Linux** — на macOS есть monitor-thread который следит когда screencapture завершится (юзер нажал Stop в системе) чтобы восстановить tray. На Linux надо сделать аналог: следить за ffmpeg child, при exit вернуть tray в normal mode.

### Low priority (cleanup / robustness)

- [ ] **Detection of portal availability runtime** — сейчас `screen_capture_portal::capture_via_portal` падает если portal недоступен. Можно делать `is_screenshot_portal_available()` check ДО вызова и красиво обрабатывать.
- [ ] **AppIndicator legacy fallback** — на старых дистрах с GTK3 + libappindicator (не libayatana) tray может не работать. Сейчас Tauri ожидает libayatana. В установочных инструкциях это должно быть.
- [ ] **Region screenshot на Wayland** — сейчас `start_region_capture` вызывает overlay window для выделения и потом `capture_fullscreen` + crop. На Wayland portal сам предлагает region selection. Можно использовать `interactive(true)` для пути «pure portal без своего overlay».
- [ ] **gnome_extensions: Vec → Option<Vec>** — fix из minor issues подзадачи 1.
- [ ] **parse_extensions_list** не понимает `@as []` (gsettings empty marker).
- [ ] **set_tray_recording_mode** ждать события ffmpeg-exit на Linux (см. выше).

### Compositor-specific исследования

- [ ] **Hyprland**: проверить работает ли `org.freedesktop.portal.ScreenCast` через `xdg-desktop-portal-hyprland`. Layer-shell для overlay окна.
- [ ] **Sway**: `xdg-desktop-portal-wlr` — частичная поддержка ScreenCast/Screenshot, нет GlobalShortcuts portal.
- [ ] **COSMIC** (System76): свой portal `xdg-desktop-portal-cosmic`, в активной разработке. Минимально проверить screenshot в их portal.
- [ ] **Pantheon (elementary)**: свой indicator API, не SNI. Значит наш SNI detection покажет «нет watcher» и приложение пойдёт в window-visible mode — что для elementary нормальный UX.

## Не делать в этом проекте (out of scope)

- Свой GNOME Shell extension для tray — это отдельный проект (JS+метаданные), пользователь устанавливает отдельно.
- Auto-install зависимостей через apt/dnf — security-wise плохая идея для desktop app.
- ScreenCast custom UI поверх portal'а — нарушает security model.
