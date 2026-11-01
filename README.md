<h1 align="center">ONYX</h1>

<p align="center">
<b>Легковесный десктопный плеер для SoundCloud</b><br>
Без рекламы · Без капчи · Без региональных блоков
</p>

<p align="center">
<a href="https://github.com/qwezzxxx/soundcloud-desktop/releases/latest">
<img src="https://img.shields.io/github/v/release/qwezzxxx/soundcloud-desktop?style=for-the-badge&logo=github&color=000000&label=ВЕРСИЯ" alt="Version"/>
</a>
<a href="https://github.com/qwezzxxx/soundcloud-desktop/releases">
<img src="https://img.shields.io/github/downloads/qwezzxxx/soundcloud-desktop/total?style=for-the-badge&logo=download&color=222222&label=Скачивания" alt="Downloads"/>
</a>
<a href="LICENSE">
<img src="https://img.shields.io/badge/Лицензия-AGPLv3-0055ff?style=for-the-badge" alt="License: AGPLv3"/>
</a>
</p>

---

## Что это?

**ONYX** — нативное десктопное приложение для SoundCloud на базе Tauri 2 (Rust). Весит около 15 МБ, запускается мгновенно, не грузит систему и не требует браузер или Electron.

Работает на **Windows** и **Linux** (Arch, Ubuntu, Debian, Fedora).

---

## Возможности

### Прямой доступ к каталогу
Музыка и плейлисты загружаются напрямую: треки играют сразу без сбоев и дополнительных настроек.

### Никакой рекламы и капчи
Нет баннеров, промо-вставок между треками и проверок Cloudflare. Открыл приложение — слушаешь.

### Полный каталог без ограничений
Доступ ко всей музыке, ремиксам, сниппетам и плейлистам SoundCloud без региональной цензуры. Дополнительно встроен поиск по YouTube Music.

### Караоке-тексты песен
Синхронизированные тексты в реальном времени (подтягиваются через LRCLIB и Genius).

### 10-полосный эквалайзер
Встроенный эквалайзер с пресетами (Bass Boost, Vocal, Electronic, Rock) для настройки звука под свои наушники.

### Сохранение лайков без аккаунта
Можно пользоваться плеером гостем — лайки, плейлисты и история воспроизведения сохраняются локально на ПК и никуда не пропадут при перезапуске или выходе.

### Темные темы
Интерфейс под темную эстетику и OLED-экраны:
- **Pure OLED** (глубокий черный #000000)
- **Wine Glow** (темный винный оттенок)
- **Midnight, Emerald, Cyber Neon**

### Интеграция с системой и Discord
- **Discord Rich Presence** — показывает трек, исполнителя и обложку в твоем профиле Discord.
- **Работа в фоне и трей** — окно сворачивается в трей и не мешает при работе или в играх.
- **Медиа-клавиши** — управление плей/паузой и треками с клавиатуры.

---

## Скачать

| Платформа | Формат | Описание |
|---|---|---|
| **Windows** | [`.exe (Установщик)`](https://github.com/qwezzxxx/soundcloud-desktop/releases/latest) | Стандартная установка в систему |
| **Windows** | [`.zip (Portable)`](https://github.com/qwezzxxx/soundcloud-desktop/releases/latest) | Распаковал в любое место и запустил |
| **Linux (Arch / Любой)** | [`.AppImage`](https://github.com/qwezzxxx/soundcloud-desktop/releases/latest) | Запуск в один клик (`chmod +x` и старт) |
| **Linux (Ubuntu / Debian)** | [`.deb`](https://github.com/qwezzxxx/soundcloud-desktop/releases/latest) | Пакет для apt / dpkg |
| **Linux (Fedora / RHEL)** | [`.rpm`](https://github.com/qwezzxxx/soundcloud-desktop/releases/latest) | Пакет для dnf / rpm |

---

## Ветки разработки

- **`main`** — релизная ветка со стабильными сборками под Windows и Linux.
- **`dev`** — тестовая ветка для быстрой разработки (собирает только AppImage под Linux).

---

## Лицензия

Проект распространяется под свободной лицензией **GNU Affero General Public License v3.0 (AGPLv3)**. Подробности — в файле [LICENSE](LICENSE).

SoundCloud является зарегистрированным товарным знаком SoundCloud Ltd. Данный проект не аффилирован с SoundCloud.
