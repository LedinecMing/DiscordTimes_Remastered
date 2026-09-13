# Отчёт о проделанной работе: с «Configs and dependencies» до сентября 2026

Период: коммит `76e0b64` (Configs and dependencies, 22 мая) → текущий HEAD `working`.
Объём: ~45 коммитов в интеграционной ветке `working` + 6 фичеветок, влитых через мерджи.
Организация: главный агент координирует параллельных суб-агентов, каждый в своём
git worktree (`../DT_<имя>`), мерджи в `working` — только главным агентом после
зелёного прогона тестов (см. AGENTS.md, правила 1–11).

---

## 1. Бонусы юнитов (Bonus1–21)

Архитектура: бонусы — **данные**, не код. `BonusInfo.rules` (AbilityCondition →
Mechanic) + `Mechanic::apply`; боевые функции дают только точки диспетча
(BattleStart, Day, Attacked, Kill). Ноль if-бонусов в combat-функциях.

Сделано:
- **Bonus1 Щитоносец, Bonus2 Быстрота** — BattleStart-механики (37f653b).
- **Bonus3 Проникающий Удар** (b3407bf) — `compute_attack_damage` пробивает долю
  защиты цели рукой/стрелой/магией; add_modify-канал для модификаторов.
- **Bonus4 Лекарь** (6a0c081) — новый `AbilityCondition::Day` +
  `Executor::last_day/advance_day`: разовый хил на смену игрового дня;
  `percent_add` = доля от max_hp.
- **Bonus7/8 Кара/Гнев** (b3407bf) — true damage; попутно найден скрытый баг:
  true_damage никогда не потреблялся (старый тест проходил на min-1 уроне);
  добавлен тест +10 против защиты 1000.
- **Bonus14** (обе части), данные для 9/12/17.
- `parse_bonuses` — graceful-фолбэк без bonuses.json5 (6584b45); тестовые бонусы
  зеркалятся программно в `make_test_registry` (риск рассинхрона с диском
  известен и зафиксирован).

Отложено (с готовыми рецептами): Торговец (5), Deathcurse (6/9 — нужен
Attacked/Kill-диспатч), Гарнизон (15), поглощение (10/11), Нежить (18/19 —
данные: `percent_add` от нулевой базы даёт 0, нужно `add 70`), фланговый
модификатор (21), оплата (16).

## 2. Карта, армии, интеракция строений

- **Занятость клеток** (983a11c): `Executor::tick` не делает шаг на клетку с
  армией (`hitmap.army`); тесты `goto_onto_occupied_tile_builds_no_path`,
  `tick_does_not_step_onto_occupied_tile`.
- **Хитбоксы строений** — трёхэтапная сага, корень найден:
  1. `calc_hitboxes` искал размер строения **позицией в векторе реестра**, а
     `building.id` — это `ObjectInfo.index` (500+) → всем строениям давался
     размер 1×1 от заглушки Tree0 (bca08a0, тесты писаны до фикса).
  2. Спан хитбокса = ровно спан рендера: `[pos-w+1..pos]×[pos-h+1..pos]`
     (4c1acfd; промежуточная правка +1/+1 дала сдвиг — тест поймал).
  3. Обводка по клику на **любую** клетку хитбокса: `lmb_building` теперь
     ставится с hitmap-кликнутого тайла (раньше не ставился вовсе).

- **Дабл-клик меню строения не работал никогда** (8a6f208): гипотеза
  подтверждена кодом — `building_ui.last_click` нигде не устанавливался в
  `Some` (только сброс), ветки `double_open/double_go` были мёртвыми.
  Таймер не виноват (`time_secs` f64, окно 0.4 с корректно).
  Фикс + **F10** — дебаг-дамп цепочки клика в stderr.
- **F9** — слой принадлежности тайлов строениям (9d9e225 + фикс 69143a8):
  полупрозрачная заливка (α=0.35), цвет строения — детерминированный HSV-хеш
  по золотому сечению; первый рендер был зеркален по диагонали —
  flat-индексы hitmap читались как `(x,y)` вместо `(y,x)`.
- **Погоня за армией** (7dee3a9 + 42bad97): `Army.chasing` +
  `ClientMessage::Follow`; путь строится к **лучшему соседнему** тайлу цели
  (на клетку цели не заходим), repath в tick при смещении цели, контакт
  (Чебышёв ≤ 1) → бой + сброс; сброс при GoTo/прибытии/исчезновении цели.
  UI: одинарный и дабл-клик по чужой армии = Follow. 4 теста.
- **Антипанический свип Registry** (правило AGENTS.md п.10): падение
  `item.rs:138` (registry[id] на грязных id карт) — `Registry::get` +
  filter_map в окне строения; `set_unit_at` тоже переведён на `.get`.

## 3. Боевой ИИ

- **Battle AI MVP** (10352fe): `dt_lib/src/ai/{policy,eval,battle}` — жадный
  1-ply `BattleAi::decide` (Attack / MoveAndAttack / Move / Skip),
  позиционные правила; `evaluate_position()` как фасад; без LLM/эволюций.
- **Интеграция** (e0b1959): ИИ ходит в одиночном бою с паузой 0.4 с;
  смоук бот-против-бота.
- Отложено: Map AI (этап 4–6), 2-ply lookahead, AiOpponent для PVP.

## 4. PVP

- **Этап 1** (a6ea653): протокол комнат — `RoomConfig`/`PvpRoomSummary` DTO,
  `RoomManager`, `ClientMessage::Room`/`ServerMessage::Room`.
- **Этап 2** (804b2e7 + 77e220b): `coin_flip_initiative` по сиду,
  `your_army` в BattleInfo/ServerMessage; экраны PvpLobby/PvpRoomSetup/
  PvpRoom с рамками своей армии и линейкой ходов.
- Впереди (вне этого периода): websocket-транспорт, чат, elo, аккаунты.

## 5. Discord Rich Presence

- `wgpu_ui/src/rich_presence.rs` (aac208a): `PresenceState`, IPC-клиент,
  `build_activity` + тесты (сериализация Activity без подключения).
- Интеграция (be8c1e6): `State.rpc`, обновление статуса в кадре,
  `Game.recent_wins` — стрик побед (одиночка: winner==0; онлайн:
  winner==online.army), сброс на поражении.
- App id 1141007311771541524, ассет **"shlem"** — ждёт загрузки владельцем в
  Discord Developer Portal; без запущенного Discord — одна строка в stderr,
  игра работает.

## 6. Конвертер ассетов (lit2png)

- fe78a37: LIT/UGS/TGA/BMP → PNG; UGS-атласные форматы реверс-инжинирены;
  **4900+ PNG** из old_assets/Graphics → dt/assets; 13 тестов.
- `.spi` пропущен (формат не разобран).

## 7. Редактор карт (отдельный egui — до паритета)

- editor-core (без UI): Command/CommandHistory (undo/redo), MapProject,
  EditorState; editor-validators: run_all (patrol_bounds, radius_range).
- editor-ui (egui): canvas-инструменты Brush/Deco/Building/Army, сетка,
  выделение, статус-бар.
- **dtm в обе стороны** (f3b265f): `parse_dtm_map`/`convert_dtm_map` и
  `gamemap_to_dtm` (dt_lib/src/map/dtm_writer.rs) — Save возвращает Option,
  ошибка в статус-бар, без паник; rfd-диалоги.
- **Рендер атласов** (2d0942a): срез одного варианта 32×22 из Terrain-атласа
  8×11 по хешу (x,y) вместо ужатия всего атласа — ушло мыло/серость клеток.
- 29 тестов; в редактор идеи: notes/NEW_EGUI_MAP_EDITOR.md.

## 8. UI строений в игре

- `wgpu_ui/src/building_view.rs`: вкладки (Главный зал / Рынок / Казармы /
  Заклинания), карточки рынка + лог сделок + тултипы, сетка найма,
  лечение/воскрешение, книга заклинаний армии (`Army.spells` +
  `learn_spell` — «учится ≠ применяется», тест).
- Вход в строение: дабл-клик по карте + `pending_open`; ПКМ-попап
  (имя/владелец/отношения/описание/гарнизон).
- Рестайл под оригинальные ассеты `dt/assets/Window/` — **в работе**
  (Win-marble/Win-red, Corner_Frame, Btn1Up/Down, wb1/wb2/wb3-слоты,
  DownCorner-декор, QuestBorder для слухов, S_*-картинки строений).

## 9. Редактор вкладкой в wgpu_ui (ветка feature/editor-tab — на финале)

Решение: egui **сохраняется** как UI-фреймворк оболочки, но всё живёт внутри
крейта wgpu_ui; карта рендерится **единым bake-путём с игрой**.

- egui 0.36 + egui-wgpu 0.36 — первое поколение на **wgpu 30**, том же, что
  игра (смоук + cargo tree: одна версия wgpu в графе).
- `egui_layer.rs`: ручная конвертация InputState→RawInput, run/finish/render
  на том же device/queue; `render_egui` в Gfx — pass LoadOp::Load на frame
  view после игровых пассов (`forget_lifetime`).
- `editor_view.rs` (~650 строк): панели, канвас с зумом/паном/ховером,
  инструменты через `history.execute`, Ctrl+Z/Ctrl+Shift+Z, Lint run_all,
  Open/Save .dtm; `bake_editor_map` — запечка тем же tile_pixels-путём.
- Катовер: editor-ui (egui-окно) удалён из members и с диска;
  editor-core/render.rs (софтверный рендер) — долой; Tool переехал в
  editor-core.
- Починено по пути: ashpd-пин (rfd без zbus/gtk-тяги), предсломанные
  dt_launcher и quad_ui исключены из members (оба — эксперименты коммита
  552f77a «Пробуем»; quad_ui ещё и в default-members — заменён на wgpu_ui).

## 10. Процесс и инфраструктура

- **AGENTS.md, 11 правил** (8664390): worktree-изоляция суб-агентов +
  абсолютные пути (три утечки в основное дерево пойманы и откачены:
  editor-core/lib.rs, Cargo.toml, gfx.rs), собираемый HEAD, крейты в корне,
  `Registry::get` для данных карт, мерджи только главным агентом.
- **Тесты-сначала** для игровой логики (п.2) — соблюдено (hitbox-тесты,
  chase-тесты писались до фиксов и ловили баг красным).
- Состояние верификации `working`: dt_lib **125/0** (1 ignored),
  wgpu_ui 13/0, lit2png 13/0, editor-core 19/0.

## 11. Что дальше (очередь)

1. Финал `feature/editor-tab` (workspace build, коммит катовера) → мердж в
   `working`, полный прогон.
2. Финал рестайла UI строений (UiRestyle) → коммит в `working`.
3. Владельцу: загрузить "shlem" в Discord Portal; ручной чек-лист карты
   (обводка/меню/F9/F10/погоня — см. отчёты агентов выше).
4. Отложенные бонусы (п.1) — Attacked/Kill-диспатчи + Торговец + Гарнизон.
5. Map AI (этапы 4–6), 2-ply lookahead, PVP-транспорт/чат/elo.
6. Судьба quad_ui/dt_launcher (исключены из members — восстановить или
   удалить с диска).
