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

## Иконка приложения (Desktop)

Используется в: Dock (macOS), Taskbar (Windows), Launcher (Linux).

> ⚠️ **На Windows сейчас стоит дефолтная иконка Tauri** — размытый серый квадрат. Нужна оригинальная иконка для нормального отображения в taskbar и при установке.

| Параметр | Значение |
|----------|----------|
| Размер | 1024x1024 px |
| Формат | PNG на прозрачном фоне |
| Стиль | Цветная, в духе macOS Big Sur |

Из этого PNG генерируются все остальные форматы (icns для macOS, ico для Windows).

---

## Иконка приложения (iOS)

Если планируется мобильная версия — иконки для iOS должны соответствовать [Apple Human Interface Guidelines](https://developer.apple.com/design/human-interface-guidelines/app-icons).

### Технические требования

| Параметр | Значение |
|----------|----------|
| Формат | PNG, **без прозрачности** (без alpha channel) |
| Цветовое пространство | sRGB или Display P3 |
| Форма | Квадрат, система сама скругляет углы (не рисовать скругления вручную!) |
| Основной размер | 1024x1024 px |

### Все необходимые размеры

| Назначение | Размер (px) | Файл |
|------------|-------------|------|
| App Store | 1024x1024 | `icon_1024.png` |
| iPhone App (@3x) | 180x180 | `icon_60x60@3x.png` |
| iPhone App (@2x) | 120x120 | `icon_60x60@2x.png` |
| iPad App (@2x) | 152x152 | `icon_76x76@2x.png` |
| iPad Pro App (@2x) | 167x167 | `icon_83.5x83.5@2x.png` |
| Spotlight (@3x) | 120x120 | `icon_40x40@3x.png` |
| Spotlight (@2x) | 80x80 | `icon_40x40@2x.png` |
| Settings (@3x) | 87x87 | `icon_29x29@3x.png` |
| Settings (@2x) | 58x58 | `icon_29x29@2x.png` |
| Notification (@3x) | 60x60 | `icon_20x20@3x.png` |
| Notification (@2x) | 40x40 | `icon_20x20@2x.png` |

### Рекомендации

1. **Без прозрачности** — iOS заполняет прозрачные области чёрным, будет плохо выглядеть
2. **Без скруглений** — система скругляет автоматически, не рисовать маску вручную
3. **Читаемость** — иконка должна быть узнаваемой в 29x29 (самый мелкий размер для Settings)
4. **Единый дизайн** — та же концепция что и на Desktop, адаптированная под квадрат без прозрачности
5. **Яркий фон** — нужен непрозрачный фон (не белый, лучше фирменный цвет)

---

## Иконка приложения (Android)

Для Android требуется адаптивная иконка (Adaptive Icon), состоящая из двух слоёв — фон + передний план.

### Технические требования

| Параметр | Значение |
|----------|----------|
| Формат | PNG |
| Цветовое пространство | sRGB |
| Adaptive Icon | Два слоя: foreground + background |
| Безопасная зона | Контент в центральных 66% (72x72 в сетке 108x108) |

### Все необходимые размеры

| Плотность | Размер иконки | Размер слоя (adaptive) | Папка |
|-----------|--------------|----------------------|-------|
| mdpi | 48x48 | 108x108 | `mipmap-mdpi/` |
| hdpi | 72x72 | 162x162 | `mipmap-hdpi/` |
| xhdpi | 96x96 | 216x216 | `mipmap-xhdpi/` |
| xxhdpi | 144x144 | 324x324 | `mipmap-xxhdpi/` |
| xxxhdpi | 192x192 | 432x432 | `mipmap-xxxhdpi/` |
| Play Store | 512x512 | — | `icon_512.png` |

### Структура Adaptive Icon

```
ic_launcher.xml:
  <adaptive-icon>
    <background android:drawable="@mipmap/ic_launcher_background"/>   ← фон (цвет или градиент)
    <foreground android:drawable="@mipmap/ic_launcher_foreground"/>   ← логотип
  </adaptive-icon>
```

### Рекомендации

1. **Безопасная зона** — весь значимый контент (логотип) должен быть в центральных 66%. Остальное может быть обрезано системой (круг, скруглённый квадрат, капля — зависит от лаунчера)
2. **Два слоя** — фон (background) может быть просто цветом. Передний план (foreground) — логотип с прозрачным фоном
3. **Не дублировать фон** — foreground слой должен быть на прозрачном фоне, фон задаётся отдельно
4. **Legacy иконка** — дополнительно нужна обычная 48x48...192x192 иконка для старых устройств (до Android 8)
5. **Play Store** — 512x512 PNG, полноцветная, без прозрачности

### Генерация

Проще всего использовать Android Studio → Image Asset Studio:
1. Открыть Android Studio → File → New → Image Asset
2. Загрузить PNG 1024x1024 (тот же что для Desktop)
3. Настроить отступы и фон
4. Studio автоматически сгенерирует все размеры и adaptive icon XML

---

## Общая рекомендация по дизайну

Все три платформы (Desktop, iOS, Android) должны использовать **один и тот же дизайн-концепт**, адаптированный под требования каждой:

| Платформа | Прозрачность | Скругление | Фон |
|-----------|-------------|-----------|-----|
| macOS/Windows/Linux | Прозрачный фон | Нет (система сама) | Прозрачный |
| iOS | **Без прозрачности** | Нет (система сама) | Непрозрачный (фирменный цвет) |
| Android | Два слоя | Нет (система сама) | Отдельный слой |
| macOS Tray | Прозрачный фон | Нет | Прозрачный, монохром |

**Исходник**: один PNG 1024x1024 с логотипом на прозрачном фоне. Для iOS и Android добавляется цветной фон.

---

## Файлы для замены (Desktop)

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

## Генерация iOS иконок из PNG 1024x1024

```bash
for size in 20 29 40 60 76 83.5; do
  for scale in 2 3; do
    px=$(echo "$size * $scale" | bc | cut -d. -f1)
    sips -z $px $px icon_1024.png --out "icon_${size}x${size}@${scale}x.png"
  done
done
cp icon_1024.png icon_1024_appstore.png
```

## Генерация Android иконок из PNG 1024x1024

```bash
for density_size in "mdpi:48" "hdpi:72" "xhdpi:96" "xxhdpi:144" "xxxhdpi:192"; do
  density="${density_size%%:*}"
  size="${density_size##*:}"
  mkdir -p "mipmap-${density}"
  sips -z $size $size icon_1024.png --out "mipmap-${density}/ic_launcher.png"
done
sips -z 512 512 icon_1024.png --out icon_512_playstore.png
```
