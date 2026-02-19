# Требования к иконкам

## Иконка трея macOS

### Технические требования

| Параметр | Значение |
|----------|----------|
| Формат | PNG с прозрачным фоном (alpha channel) |
| Размеры | 32x32 px (отображается как 16x16 на обычных экранах, 16x16@2x на Retina) |
| Цветовая модель | RGBA |
| Стиль | Template image — монохромный чёрный (#000000) |

### Рекомендации по дизайну

1. **Простота** — иконка должна быть узнаваема в размере 16x16 px
2. **Монохромность** — только чёрный цвет (#000000), macOS сам инвертирует для тёмной темы
3. **Padding** — 1-2 px отступа по краям
4. **Без градиентов** — только сплошной цвет
5. **Чёткие линии** — толщина линий не менее 1 px

### Концепции

**Вариант 1: Облако + стрелка вниз**
```
     ╭───────╮
   ╭─┤       ├─╮
  │             │
  │      ↓      │
  ╰─────────────╯
```

**Вариант 2: Облако + рамка скриншота**
```
     ╭───────╮
   ╭─┤       ├─╮
  │   ┌─────┐   │
  │   │     │   │
  ╰───┴─────┴───╯
```

**Вариант 3: Камера в облаке**
```
     ╭───────╮
   ╭─┤   ◉   ├─╮
  │   ╭─────╮   │
  │   ╰─────╯   │
  ╰─────────────╯
```

### Текущая проблема

Текущая иконка многоцветная (облако #75A4D5, шестерёнки #A4A743) — не работает как template image, отображается белым квадратом в светлой теме macOS.

---

## Иконка приложения (Dock, Windows taskbar)

| Параметр | Значение |
|----------|----------|
| Размер | 1024x1024 px |
| Формат | PNG на прозрачном фоне |
| Стиль | Цветная, в духе macOS Big Sur |

---

## Файлы для замены

```
src-tauri/icons/
├── 32x32.png          # трей macOS (монохромная)
├── 128x128.png        # приложение
├── 128x128@2x.png     # Retina
├── icon.icns          # macOS app bundle
└── icon.ico           # Windows
```

## Генерация icns из PNG 1024x1024

```bash
mkdir icon.iconset
sips -z 16 16   icon_1024.png --out icon.iconset/icon_16x16.png
sips -z 32 32   icon_1024.png --out icon.iconset/icon_16x16@2x.png
sips -z 32 32   icon_1024.png --out icon.iconset/icon_32x32.png
sips -z 64 64   icon_1024.png --out icon.iconset/icon_32x32@2x.png
sips -z 128 128 icon_1024.png --out icon.iconset/icon_128x128.png
sips -z 256 256 icon_1024.png --out icon.iconset/icon_128x128@2x.png
sips -z 256 256 icon_1024.png --out icon.iconset/icon_256x256.png
sips -z 512 512 icon_1024.png --out icon.iconset/icon_256x256@2x.png
sips -z 512 512 icon_1024.png --out icon.iconset/icon_512x512.png
sips -z 1024 1024 icon_1024.png --out icon.iconset/icon_512x512@2x.png
iconutil -c icns icon.iconset -o icon.icns
rm -rf icon.iconset
```
