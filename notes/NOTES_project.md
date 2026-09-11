# NOTES_project — архитектура, сборка, верификация

## Воркспейс (nightly Rust, resolver 2)
| Крейт | Роль |
|---|---|
| `quad_ui` | Старый клиент на macroquad. Заморожен как референс; новые фичи НЕ сюда |
| `wgpu_ui` | Новый клиент: wgpu + winit, свой immediate-mode UI. Целевой клиент |
| `dt_lib` | Игровое ядро: юниты (`units/unit.rs`), бой (`battle/battlefield.rs`), эффекты (`effects/effect.rs`), бонусы, карта (`map/`), парсинг INI (`parse.rs`), сеть |
| `dt_server` / `dt_client` | Центральный сервер (websocket, комнаты) и клиент подключения. `dt_client` реэкспортирует `dt_server` (`pub use dt_server`) |
| `advini` / `advini_derive` | Формат INI + derive (`Sections`, `Ini`). См. NOTES_advini_parsing |
| `dtm_editor`, `dtm_info`, `board-analysis`, `dt_launcher`, `math_thingies` | Инструменты, математика (`Percent`, `Modify` в math_thingies) |

Данные игры — `dt/`: `Units.ini`, `Objects.ini`, `Effects.ini`(?), `Maps_Rus/*.DTm`, `assets/`, локали.

## Сборка
- Nightly Rust (`rust-toolchain` не задан; текущий 1.100 nightly). Linux-линкер `lld`, Windows-cross mingw (`.cargo/config.toml`).
- `cargo build -p wgpu_ui` / `cargo test -p dt_lib --lib` — рабочие команды.
- Тесты идут быстро (~0.1–0.2 c) — если «виснут», это дедлок на `SendMut`, см. NOTES_battle_logic.

## Запуск и верификация UI
- Бинарник запускать **из `dt/`**: `cd dt && ../target/debug/wgpu_ui`. Иначе паника на отсутствующих ассетах (icon-64.png и т.д.).
- Fullscreen-окно 1920×1200 (мировые координаты UI — 1920×1080, Y-вниз).
- Headless-верификация (машина без интеракции): `xdotool` для кликов/клавиш по window id (`xdotool search --sync --name "DT REMASTERED"`), `import -window <id>` для скриншотов; просмотр через PIL thumbnail.
- Кнопки меню — только мышью (клавиатурной навигации нет). Известные точки клика: PvE «Битва с собой» ~(170,192); «Нажми если готов» в сетапе ~(1080,508).
- Android: SDK/NDK на машине нет (`~/Android` отсутствует) — entrypoint и метаданные cargo-apk в `wgpu_ui` готовы, но сборка не проверялась.

## Верификация игровой логики
- Контракт боевой логики фиксируется тестами `dt_lib/src/battle/tests.rs` — при добавлении правил сначала тест (красный), потом правка.
- `cargo test -p dt_lib --lib` — быстрый (0.1 с), никакого IO; `map::convert::test` ignored (нужны ассеты).

## Известные открытые задачи (на момент заметки)
1. **Камера карты не двигается (WASD/пан/зум)** — корень: `wgpu_ui/src/gfx.rs::end_frame` пишет ОДИН общий `cam_buf` между пассами, но все пассы уходят одним сабмитом → каждый пасс читает камеру последнего пасса (HUD-дефолт). Фикс: пер-пасс uniform-буфер + bind group (как сделано для vertex/index). Симптом: всё остальное работает (хоткеи R/G видны), двигается только камера.
2. `dbg!`-печати в `dt_lib/src/units/unit.rs` (`attack`/`apply_attack`) и `eprintln!`-трейсы в `battlefield.rs`/`screens.rs` — временные, убрать при чистке.
3. Мёртвый код в `wgpu_ui/src/map_view.rs::draw_map` (пустой `pics`/`draws`), мёртвая `!prerender`-ветка удалена.
4. `wgpu_ui/src/screens.rs`: часть экранов ещё держит старые helper-заглушки (`get_unit_texture_from` и др. уже реальные; проверять при рефакторинге).