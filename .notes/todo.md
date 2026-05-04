# Linux roadmap — что осталось

## Сделано — Linux UX foundation

### Сессия 2026-05-04 (вечер 1)
- ✅ Linux self-diagnostic в `run_diagnostics` (Подзадача 1)
- ✅ Linux Health Check UI в MainPage с цветными индикаторами + i18n (2)
- ✅ SNI detection через DBus + adaptive launch (3)
- ✅ Wayland screenshot через `ashpd` portal (4a, 4b)
- ✅ ScreenCast portal session wrapper (5a)
- ✅ ffmpeg recorder для pipewire stream (5b)
- ✅ Полная интеграция Wayland video recording в `start_video_capture` (5c)
- ✅ Cleanup percent_decode helper (7)

### Сессия 2026-05-04 (вечер 2)
- ✅ GlobalShortcuts portal detection (6a lite)
- ✅ GlobalShortcuts portal full registration с listener task (6b/c)
- ✅ Monitor-thread для ffmpeg exit auto-recovery — `ae65c37`

## Осталось — в порядке приоритета

### Medium priority (фичи)

- [ ] **5d** — region video recording на Linux: overlay для выделения region (как на macOS) + ffmpeg `-filter:v "crop=W:H:X:Y"` от portal-stream.
- [ ] **5e** — window video recording на Linux: реализуется через `SourceType::Window` в ScreenCast portal — пользователь выберет окно через portal-picker, ffmpeg пишет полученный stream.
- [ ] **i18n Linux notification** в `lib.rs::run()` setup — сейчас текст «Приложение запущено...» захардкожен. Локаль читается из tauri-plugin-store, но в setup блоке плагин ещё может быть не готов — потребуется defer notification после store init.

### Low priority (cleanup / robustness)

- [ ] **Runtime portal availability check** — сейчас `screen_capture_portal::capture_via_portal` и `screencast_portal::ScreencastSession::start` падают с error если portal не доступен. Можно делать `is_screenshot_portal_available()` ДО вызова через DBus Introspect (как `is_sni_watcher_available`) и возвращать nice error frontend'у.
- [ ] **AppIndicator legacy fallback** — на старых дистрах с GTK3 + libappindicator (не libayatana) tray может не работать. Tauri ожидает libayatana. Документировать в README + проверять в diagnostics.
- [ ] **Region screenshot на Wayland через portal interactive** — сейчас `start_region_capture` вызывает свой overlay-window для выделения и потом `capture_fullscreen` + crop. На Wayland portal `interactive(true)` сам предлагает region selection — можно сделать "чистый portal путь" без нашего overlay.
- [ ] **GlobalShortcutsSession.close on shutdown** — сейчас session создаётся в setup, но никогда не закрывается явно. Tauri не имеет shutdown hook'а в Builder API; можно добавить close в `on_window_event` для main.

### Compositor-specific исследования

- [ ] **Hyprland**: проверить работает ли наш path через `xdg-desktop-portal-hyprland`. Layer-shell для overlay-окна (наш сейчас X11/wayland через WebViewWindowBuilder).
- [ ] **Sway**: `xdg-desktop-portal-wlr` — частичная поддержка ScreenCast/Screenshot, **нет GlobalShortcuts portal** — наш fallback (видимое окно + System Settings) корректен.
- [ ] **COSMIC** (System76): свой `xdg-desktop-portal-cosmic`, в активной разработке. Минимально проверить screenshot.
- [ ] **Pantheon (elementary)**: свой indicator API, не SNI. Наш SNI detection покажет «нет watcher» → приложение пойдёт в window-visible mode — для elementary нормальный UX.

## Не делать в этом проекте (out of scope)

- Свой GNOME Shell extension для tray — это отдельный проект (JS+метаданные), пользователь устанавливает отдельно.
- Auto-install зависимостей через apt/dnf — security-wise плохая идея для desktop app.
- ScreenCast custom UI поверх portal'а — нарушает security model.
