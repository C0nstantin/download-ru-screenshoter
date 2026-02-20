# Запись видео — ТЗ для Linux и Windows

На macOS используется встроенная утилита `screencapture -v -R x,y,w,h output.mov`.
Для Linux и Windows нужны другие решения.

---

## Linux

### Рекомендуемый подход: ffmpeg + x11grab / pipewire

**Вариант 1: ffmpeg + x11grab (X11)**

```bash
ffmpeg -video_size WxH -framerate 30 -f x11grab -i :0.0+X,Y output.mp4
```

Зависимости:
- `ffmpeg` (в репозиториях: `sudo apt install ffmpeg`)
- Работает только на X11, не на Wayland

**Вариант 2: ffmpeg + pipewire (Wayland / X11)**

Через PipeWire portal (XDG Desktop Portal):
- Запросить `org.freedesktop.portal.ScreenCast`
- Получить fd потока
- Передать в ffmpeg через `-f lavfi -i "pipeline=..."`
- Работает на Wayland (GNOME, KDE)

**Рекомендация:** Использовать `gstreamer` или `ffmpeg` в зависимости от наличия.
Проверять наличие на старте: `which ffmpeg || which gst-launch-1.0`.

### Реализация в Rust

```rust
// Проверить ffmpeg
// Запустить: ffmpeg -video_size {w}x{h} -framerate 30 -f x11grab -i :0.0+{x},{y} output.mp4
// Остановить: child.kill() → SIGINT
```

Дополнительные системные зависимости (Ubuntu):
```bash
sudo apt install ffmpeg libpipewire-0.3-dev
```

---

## Windows

### Рекомендуемый подход: ffmpeg + gdigrab

```
ffmpeg -f gdigrab -framerate 30 -offset_x X -offset_y Y -video_size WxH -i desktop output.mp4
```

**Вариант 2: Windows Graphics Capture API (WinRT)**

- Нативный API с `Windows.Graphics.Capture`
- Доступен с Windows 10 1803+
- Лучшее качество, поддержка HDR
- Requires `windows-rs` crate (`windows::Graphics::Capture`)

```toml
# Cargo.toml (только Windows)
[target.'cfg(target_os = "windows")'.dependencies]
windows = { version = "0.58", features = ["Graphics_Capture", ...] }
```

**Рекомендация:** Использовать `ffmpeg` с `gdigrab` как наиболее простое решение.
Для production — Windows Graphics Capture API.

### Поставка ffmpeg

На Windows нет системного ffmpeg. Варианты:
1. **Бандлить ffmpeg.exe** в приложение (~40 MB)
2. **Проверять наличие ffmpeg** в PATH и выводить инструкцию по установке
3. **Использовать WinRT** (нативно, без зависимостей)

---

## Режимы записи на macOS

На macOS реализованы три режима записи видео через нативную утилиту `screencapture`:

### 1. Запись экрана (полноэкранная)
- Трей: «Запись экрана»
- Команда: `start_video_capture` → `screencapture -v file.mov`
- Открывается нативный пикер Cmd+Shift+5 — пользователь выбирает экран/область/окно

### 2. Запись области
- Трей: «Запись области»
- Открывает overlay (`start_region_capture_overlay_video`) для выбора области
- Пользователь выделяет прямоугольник, подтверждает Enter / двойной клик
- Команда: `start_video_recording(x, y, w, h)` → `screencapture -v -R x,y,w,h file.mov`
- Координаты из overlay (CSS pixels) + `screen_offset` → screen points

### 3. Запись окна
- Трей: «Запись окна»
- Показывает нативный диалог выбора окна (через `osascript`/JXA + `CGWindowListCopyWindowInfo`)
- Команда: `start_video_capture_window` → `screencapture -v -l <windowID> file.mov`
- Флаг `-v -w` не поддерживается macOS, поэтому используется `-l <windowID>` для записи конкретного окна

## Общая архитектура

```
[Запись экрана]     → start_video_capture()           → screencapture -v
[Запись области]    → overlay → start_video_recording(x,y,w,h) → screencapture -v -R
[Запись окна]       → pick_window_id() → start_video_capture_window() → screencapture -v -l <wid>
                            ↓
                    RecordingPage (индикатор "REC" + кнопка "Стоп")
                            ↓
                    stop_video_recording() → путь к файлу
                            ↓
                    VideoResultPage (превью, трим, конвертация, загрузка)
```

Все три команды используют общий хелпер `launch_screencapture(args)` в `recording.rs`.

Для Linux/Windows — TODO заглушки в `start_video_recording()` и `start_video_capture_window()`.

---

## Приоритет

1. **Linux X11** — через ffmpeg, большинство дистрибутивов
2. **Windows** — через ffmpeg gdigrab или WinRT
3. **Linux Wayland** — через PipeWire portal (сложнее, нужен отдельный диалог)
