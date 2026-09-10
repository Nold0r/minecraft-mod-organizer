# Minecraft Mod Organizer

Desktop-приложение для организации Minecraft-модов без изменения физической структуры папки `mods`.

## Что уже заложено

- Rust domain/application/infrastructure разделение: доменное ядро не зависит от React/Tauri.
- Рекурсивные виртуальные категории в SQLite.
- Drag & drop:
  - категория внутрь категории;
  - категория до/после соседней категории;
  - мод внутрь категории;
  - мод до/после другого мода;
  - возврат модов/категорий в корень.
- Создание категорий кнопкой, `+` у категории и контекстным меню по категории/пустой области.
- Автоматическое сканирование `*.jar` и `*.jar.disabled`.
- Включение/выключение через `mod.jar <-> mod.jar.disabled`.
- Файловый watcher (`notify`) и событие `mods://changed` в React.
- Чтение метаданных из:
  - `fabric.mod.json`;
  - `quilt.mod.json`;
  - `META-INF/mods.toml`;
  - `META-INF/neoforge.mods.toml`.
- Отображение display name, версии, loader, mod id и иконки из JAR.

## Архитектура

```text
src-tauri/src/
  domain/             чистые сущности
  application/        use-cases и правила
  infrastructure/     SQLite, JAR scanner, filesystem watcher
  tauri_api/          команды Tauri — тонкий адаптер

src/
  api.ts              единственная точка IPC с Rust
  types.ts            DTO фронтенда
  components/         React UI
```

Категории намеренно не создают подпапки внутри `mods`. Это виртуальное дерево, хранящееся в `organizer.sqlite3` в app-data приложения.

## Запуск

Нужны Node.js и актуальный Rust stable (для зависимостей в этом каркасе лучше Rust >= 1.88).

```bash
npm install
npm run tauri dev
```

## Важные детали для следующего этапа

1. Добавить кэш метаданных по `(path, size, mtime)`, чтобы не перечитывать сотни JAR при каждом событии.
2. Для Forge/NeoForge обработать JAR с несколькими `[[mods]]` как несколько `ModDescriptor` внутри одного `ModArtifact`.
3. Разрешить конфликты одинаковых `modId` и несколько версий одного мода одновременно.
4. Сделать массовое включение/выключение категории.
5. Добавить поиск, фильтры loader/version и профили разных сборок.
6. Добавить безопасную обработку исчезновения/замены файла во время сканирования.
7. При желании заменить data-URL иконок на disk-cache + asset protocol для очень больших сборок.


## Сборка Windows

Самый простой локальный вариант: запустить `build-windows.ps1` из PowerShell.
Скрипт проверит Node/npm/Rust, установит JS-зависимости и выполнит production-сборку Tauri.
Готовый `.exe` появится в `src-tauri/target/release`, установщики — в `src-tauri/target/release/bundle`.

Workflow `.github/workflows/build-windows.yml` собирает Windows x64 на push в `main`/`master`, pull request и при ручном запуске. Он выполняет `npm ci`, `cargo check --locked` и production-сборку Tauri. EXE и установщики NSIS (`.exe`) / MSI доступны в артефакте `minecraft-mod-organizer-windows` на странице запуска Actions.

### Публикация релиза

Обновите версию одновременно в `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock` и `src-tauri/tauri.conf.json`, закоммитьте изменения и отправьте тег с этой версией:

```bash
git push origin main
git tag v0.1.3
git push origin v0.1.3
```

Push тега `v*` запускает сборку и после её успеха автоматически публикует GitHub Release с EXE и установщиками. Версия тега должна совпадать с версиями приложения. Используется стандартный `GITHUB_TOKEN`, отдельные секреты не нужны. Ручной запуск на ветке только собирает артефакты; для повторной публикации выберите тег или перезапустите его workflow.


### Windows icon

The project includes the Tauri Windows resource icon at `src-tauri/icons/icon.ico` plus PNG variants used by bundling.


## Font and category icons

The UI uses `JetBrainsMono Nerd Font` / `JetBrains Mono Nerd Font` from the operating system for both text and icon glyphs. Install a JetBrains Mono Nerd Font variant on Windows so private-use Nerd Font codepoints render correctly. Category icon values are stored as hexadecimal Unicode codepoints (for example `F07B`).

## 0.1.3

- Drag-and-drop больше не пересканирует все JAR: метаданные модов кэшируются в памяти, а DnD меняет только layout в SQLite.
- Layout всех модов читается одним SQL-запросом, порядок записывается одной транзакцией.
- Иконка `U+F0FFA` показывается только когда в физическом JAR обнаружено больше одного пакета.
- Заголовок tooltip: `Пакеты в этом jar`.
- Прокрутка списка пакетов стилизована под интерфейс приложения.
- Цвет категории наследуется вложенными категориями и модами внутри неё.
- Forge/NeoForge: учитываются все записи `[[mods]]`; поддерживается рекурсивное обнаружение вложенных JAR-in-JAR до 3 уровней.
- Иконка выбора цвета категории: Nerd Font glyph `U+E22B`.
