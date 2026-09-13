# Встраивание редактора карт вкладкой в wgpu_ui — план (цель для следующего агента)

## Контекст (состояние на 2026-09-12, ветка working)

- `editor-core` — домен редактора БЕЗ UI (ТЗ §6): `command.rs` (Command/CommandHistory, undo/redo), `project.rs` (MapProject), `state.rs` (EditorState), `render.rs` (софтверная запечка tilemap/objects → RGBA через image).
- `editor-validators` — `run_all()` + отдельные проверки (patrol_bounds, radius_range).
- `editor-ui` — egui/eframe 0.33 ОКНО (`app.rs`, 609 строк: top menu, канвас, инструменты Brush/Deco/Building/Army, статус-бар, open/save dtm через rfd).
- `wgpu_ui` — игра: собственный immediate-mode UI (`ui.rs`, порт macroquad root_ui: button/window/text, InputState с mouse/keys), рендер wgpu **30**, диспетчер экранов `lib.rs:212` по `Menu` (state.rs:126), `map_view` уже умеет печь карту в текстуру (`RenderTextures`).
- Отдельный бинарь `editor` (editor-ui) работает, но: отдельное окно, дубль загрузки ассетов, egui-стек (eframe+wgpu 27) тяжёлый рядом с игрой.

## Почему НЕ «встроить egui» прямо в кадр игры

egui 0.33 требует wgpu 27, игра на wgpu 30. Одна поверхность окна не может
обслуживаться двумя мажорными версиями wgpu; апгрейд/даунгрейд — большой
рискованный churn. Зато ТЗ §6 изначально требует «UI — тонкий слой над
state»: значит встраиваем **ядро** (editor-core/validators), а шелл рисуем
родным `ui.rs`. Дубль не возникает: egui-шелл остаётся отдельным бинарём до
паритета, потом удаляется.

## Решение: «ядро общее — шелл родной»

Новый экран `Menu::Editor(EditorUi)` в wgpu_ui; вся логика — вызовы
editor-core. Портируем только представление (~с app.rs, он тонкий).

## Шаги (порядок = PR-размеры)

1. **editor-core: перенести шелл-агностичную логику из editor-ui**
   - Hotkeys/инструменты — уже enum Tool в app.rs; перенести `Tool` в
     editor-core (или продублировать 1:1 в wgpu_ui — выбрать по кол-ву usar).
   - Bake-кэш: `render.rs` печёт всё сразу; добавить dirty-rect (перепечь
     только изменённых тайлов) — иначе каждый мазок = полная запечка.
   - Тесты команд/валидаторов уже есть — не трогать.

2. **wgpu_ui: каркас вкладки**
   - `Menu::Editor(EditorUi)` в state.rs; EditorUi { project, history,
     state, tool, baked: TexId, cam: Camera, status }
     — по образцу PvpState/BuildingUi.
   - `editor_view.rs::editor_screen(ctx)` + ветка в диспетчере lib.rs.
   - Кнопка «Редактор карт» в main_menu (screens.rs).

3. **Канвас**
   - `bake_tilemap/bake_objects` → RgbaImage → wgpu-текстура (по образцу
     map_view bake.rs) → quad на весь канвас с зумом/паном через Camera.
   - Ховер/выделение тайла: обратное преобразование курсора → тайл; рамка
     выделения поверх канваса (уже есть паттерн goto_tile-подсветки).

4. **Инструменты и команды**
   - ЛКМ по канвасу → Command (PaintTile/PlaceDeco/PlaceBuilding/PlaceArmy)
     → history.execute(); Ctrl+Z/Ctrl+Shift+Z — undo/redo; после команды —
     dirty-rect перезапечь и обновить текстуру.
   - Панель инструментов слева: кнопки Tool::ALL (ui.rs::button).

5. **Top bar + статус-бар**
   - File: New/Open/Save/Save As (rfd уже в workspace — добавить в wgpu_ui
     deps; open/save через editor-core project + dtm parse/convert из
     map-editor мерджа), Edit: Undo/Redo, Lint: run_all().
   - Статус-бар: координаты курсора, активный инструмент, ошибки/варнинги
     валидаторов, счётчик истории.

6. **Точки роста (после паритета, отдельными фазами)**
   - egui_dock-фичи ТЗ (докинг, Command Palette) — вне скоупа вкладки.
   - Event Graph/List — когда появятся в editor-core.

## Не делать
- Не тянуть egui/eframe в wgpu_ui.
- Не менять editor-core API под egui-специфику (только расширения: dirty-rect).
- Не удалять editor-ui до полного паритета вкладки.

## Риски
- rfd в wgpu_ui: проверить, что файл-диалог не блокирует render-loop
  (rfd::FileDialog блокирующий; в editor-ui уже так — для игры обернуть в
  поток/AsyncFileDialog, иначе фриз кадра).
- Ввод: ui.rs InputState — забирать события мыши только когда курсор над
  канвасом (не конфликтовать со скроллом списка инструментов).
- Память: full-bake RGBA большой карты (512×512×4 байт = 1 MiB — ок;
  больше — см. dirty-rect выше).

## Верификация
- `cargo test -p editor-core -p editor-validators` зелёный (без изменений).
- Новый smoke в wgpu_ui недоступен (нет тест-раннера UI) → ручной чек-лист:
  вкладка открывается из главного меню; кисть красит тайл; undo/redo;
  Lint показывает счётчик; Save → Open → карта совпадает (round-trip тест
  dtm_writer уже зелёный).
- Скриншот вкладки владельцу.
