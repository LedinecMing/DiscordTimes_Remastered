# ИИ-агенты: план проекта (детальный)

> Две подсистемы: **Battle AI** (ИИ-игрок в бою) и **Map AI** (автономные армии на карте).
> Ключевое ограничение владельца: это **чистый расчёт** (эвристики/поиск), без ЛЛМ и без
> эволюционных алгоритмов. Обе подсистемы строго настраиваются **правилами, заданными
> настройками карты** (конфиг карты → Policies → поведение ИИ).

## 0. Фундамент, который уже есть (не переизобретать)

- Бой: `BattleInfo` (army1/army2, active_unit, can_interact, move_order, winner);
  `search_interactions()` — все цели, в которые юнит может атаковать ПРЯМО СЕЙЧАС;
  `evaluate_position()` — **пустая заготовка** в battlefield.rs:399 (туда встаёт оценка);
  `handle_action()` — единая точка исполнения действия (по (pos, army));
  `unit::attack()` — чистая функция урона (детерминированная, идеальна для симуляций);
  `field_type`/Front/Back/Reserve — позиционирование; `possible_movement()`.
- Карта: `GameMap` (tilemap, hitmap с `army`/`building`/`passable`, buildings, armys,
  relations), `find_path()` (стоимость/достижимость), `Army.pos/path/stats`,
  `Relations { ally, neighbour, enemy: u8 }` — уже есть группировка «свой/чужой».
- Существующая настройка `Опция10 = Улучшенный интеллект противника` (локаль) —
  флаг качества ИИ уже предусмотрен игроком в UI; подключим к уровню сложности.

## 1. Общая архитектура

```
dt_lib/src/ai/
├── mod.rs            // фасад: BattleAi, MapAi, реестр policies
├── policy.rs         // AiPolicy — декларативные правила (serde), грузятся с карты
├── eval.rs           // чистые функции оценки: eval_battle_position, eval_map_target
├── battle.rs         // Battle AI: выбор действия активного юнита
├── map.rs            // Map AI: решения армий на карте (каждые N тиков)
└── tests/            // фикстуры + детерминированные тесты
```

Правило зависимостей: ИИ **не мутирует состояние сам** — он выдаёт **решение**
(enum-структуру), которое исполняется существующим кодом (`handle_action` в бою,
`message_handler(GoTo)` / атака-инициация на карте). Так ИИ тестируемо, PVP-безопасно
(сервер может запускать тот же ИИ для отката) и исключает двойные мутации.

## 2. Battle AI (бой)

### 2.1 Точка входа
- Триггер: когда `active_unit` принадлежит армии под ИИ (PVP: у открывшего комнату
  включён «ИИ за второго игрока»; сингл: враги всегда ИИ при `Опция10`).
- Сигнатура: `BattleAi::decide(battle: &BattleInfo, armies: &Vec<Army>, my: usize,
  policy: &BattlePolicy) -> AiDecision`:

```
enum AiDecision {
    Attack { target: BattleUnitPos },            // из search_interactions
    MoveAndAttack { to: usize, target: BattleUnitPos },
    Move { to: usize },                          // тактика сближения/удержания
    Skip,                                        // пропустить ход (Space-семантика)
    Cast { spell: usize, target: BattleUnitPos } // фаза 2 (если в комнате разрешены)
}
```

### 2.2 Оценка (eval.rs — чистые функции, одинаковые для обеих подсистем)
`expected_damage(me, foe) -> f64` — поверх `unit::attack` (сумма компонент:
hand/ranged/magic с защитами, pierce, true_damage — считается реально движком).
`threat_of(foe, me) -> f64` — ответный урон с учётом дистанции и инициативы врага
(`modified.speed`) — «кто ходит после меня и достанет ли».
`unit_value(troop) -> f64` — цена цели: `hp/max_hp` × вес архетипа (маг > стрелок >
рукопашный — веса в policy) + бонус за `is_main` (герой).
`eval_battle_position(battle, armies, my) -> f64` — сумма наших `unit_value` минус
сумма вражеских, плюс позиционные слагаемые (см. 2.4). Заменяет пустую
`evaluate_position()` (та остаётся фасадом для обратной совместимости).

### 2.3 Алгоритм (жадный 1-ply + опциональный 2-ply по правилам)
1. Собрать `search_interactions` → списки доступных атак.
2. Для каждой цели посчитать score:
   `score = expected_damage × unit_value(цель) − expected_retaliation × threat_weight`
   + добивания (`foe.hp <= dmg` → `kill_bonus` из policy).
3. Если целей с прямой атаки нет → перебор `possible_movement()`-достижимых клеток:
   для каждой клетки симулируем «встал сюда» и пересчитываем доступные атаки —
   `MoveAndAttack` с максимальным score (симуляция дешёвая: attack — чистая функция).
4. Нет выгодной атаки → `Move` к ближайшему врагу по «дорожной» эвристике
   (минимизировать манхэттен-дистанцию до цели с наименьшим hp/наибольшей ценностью),
   либо `Skip` по правилу удержания (стрелки/маги держат дистанцию — флаг в policy).
5. **2-ply lookahead** (вкл. в policy: `lookahead: u8`): после нашего лучшего хода
   симулировать лучший ответ врага (тем же жадным алгоритмом) и вычесть из score.
   Глубина 2 по умолчанию выкл (скорость), вкл. при `Опция10/impossible`.

Детерминированность: `unit::attack` чистый, random только если в уроне есть
рандом-компонента — при наличии seed'ить от (battle tick, unit id).

### 2.4 Позиционные правила (весовые, из policy)
- `keep_backline`: стрелок/маг не должен стоять в Front (`field_type`) — штраф.
- `protect_mages`: своя mage-цель под угрозой — штраф позиции, возле которой враг.
- `flank_awareness`: бонус за атаку с фланга (движок уже считает фланг: dist>1 по x).
- `garrison_hold`: армия в своём строении (battle_ter) не выходит за стены —
  удержание резервной линии.
- `hp_threshold_retreat`: юнит с hp < X% не лезет в размен (отходит в Back/Reserve).

### 2.5 BattlePolicy (serde, грузится из настроек карты)
```
BattlePolicy {
  aggression: f32,            // 0..1: вес атаки над самосохранением
  kill_bonus: f32,
  threat_weight: f32,
  target_priority: Vec<Weighted> { mage: f32, ranged: f32, melee: f32, main: f32 },
  keep_backline: bool,
  hp_threshold_retreat: f32,  // 0 = не отступать
  lookahead: u8,              // 0/1/2
  cast_spells: bool,
  skip_when_no_gain: bool,
}
```
Дефолты — `impl Default` (агрессивный сбалансированный бот); карта может
переопределить (см. §4 «настройки на карте»).

## 3. Map AI (армии на карте)

### 3.1 Задача
Автономная армия (нейтралы/враги/спарринг-партнёр игрока) сама решает:
стоит ли атаковать армию игрока в своём радиусе восприятия, кого именно, когда
отступать/патрулировать. Строго по правилам карты.

### 3.2 Цикл решения (каждый `Executor::tick` с периодом `think_every_ticks`)
Для каждой армии с `control = Ai`:
1. **Радиус восприятия**: собрать армии в радиусе `sight_radius` клеток
   (BFS по проходимым тайлам через `find_path`, max_cost = radius).
2. **Оценка боя (кто-кого)**: симуляция силы:
   `power = Σ unit_value(troop) × hp/max_hp` (unit_value из §2.2);
   `my_power` против `their_power` с поправками policy:
   - `aggression` (порог power_ratio: атаковать если `my/their ≥ engage_ratio`);
   - модификатор обороны: армия в своём строении (`hitmap.building` + владелец)
     получает `defender_bonus` (гарнизон воюет эффективнее);
   - численный перевес в юнитах, типы (маги в составе — вес выше, т.к. урон магией).
3. **Решение**:
   - `ratio ≥ engage_ratio` → `Engage { target_army }` — `message_handler(GoTo(тайл
     рядом с врагом))`; после прибытия — инициация боя (уже существующий механизм).
   - `ratio < disengage_ratio` → `Retreat { to: SafeTile }` — к ближайшему своему
     строению/краю радиуса (иначе «танкует на месте», если `hold_ground`).
   - иначе `Patrol` — ходить по маршруту (waypoints из policy) или стоять.
   - Гистерезис: после `Engage` не отпускать цель до `ratio < disengage_ratio`
     (анти-дребезг решений).
4. **Дипломатия**: цель — только армии с `relations.enemy == group`; союзников
   (`ally`) игнорируем всегда; `neighbour` — только если policy `hostile_neutral: true`.

### 3.3 MapPolicy (serde, с карты)
```
MapPolicy {
  enabled: bool,
  sight_radius: u32,          // в клетках пути
  think_every_ticks: u32,
  engage_ratio: f32,          // 1.2 = атакуем при 20%+ перевесе
  disengage_ratio: f32,       // 0.8 = отступаем ниже
  defender_bonus: f32,        // множитель силы обороняющегося в строении
  hostile_neutral: bool,      // трогать ли нейтралов
  hold_ground: bool,          // не преследовать далеко от базы
  max_pursuit_radius: u32,
  waypoints: Option<Vec<(usize,usize)>>, // маршрут патруля
  pay_mercy: bool,            // фаза 2: брать золото вместо боя (Village-семантика)
}
```

### 3.4 «Строго по правилам, заданным настройками на карте»
- Носитель правил: `GameMap.start` (StartStats) расширяется опциональным
  `AiSettings { battle: BattlePolicy, map: MapPolicy }` (serde-JSON в мета карты;
  alkahest-поля добавлять осторожно — карту не ломаем: поле `#[unused]`+default).
- Отсутствие настроек = дефолты (ИИ у нейтралов выключен, у врагов — дефолтная
  агрессия) — обратная совместимость со всеми текущими картами.
- Валидация при загрузке карты (`editor-validators` связка!): валидатор
  AiPolicyBounds — проверка диапазонов (ratio > 0, радиусы > 0, waypoints в границах).
  Интеграция с редактором карт: форма «ИИ» в настройках карты (фаза после MVP редактора).

## 4. Интеграция

- **Сингл**: `Опция10` (улучшенный интеллект) → `lookahead=2`/полные веса; иначе
  жадный 1-ply.
- **PVP (связка с PVP_TECH_PLAN)**: `RoomConfig.game_modification` получает
  `AiOpponent { policy: BattlePolicy|MapPolicy }` — «играть против ИИ» в комнате;
  сервер тикает ИИ на месте отсутствующего игрока (полезно и для добива дисконнекта).
- **Редактор карт**: `editor-validators` получает AiPolicyBounds; UI-форма — позже.

## 5. Тесты (repo rule #2: тест до правки)

Battle:
- `ai_prefers_kill_over_damage` (добивание ценнее большего урона);
- `ai_keeps_archers_in_backline`;
- `ai_respects_hp_threshold` (раненый юнит не разменивается);
- `ai_decision_is_deterministic` (фикстура битвы → дважды decide → одинаково);
- `ai_uses_piercing_targeting` (цель с низкой защитой ценнее при равном hp).
Map:
- `ai_attacks_when_stronger` (ratio ≥ engage);
- `ai_retreats_when_weaker` (ratio < disengage → отходит, не лезет);
- `ai_ignores_allies` (союзник в радиусе — не цель);
- `ai_uses_garrison_bonus` (в строении порог атаки ниже);
- `ai_holds_ground_when_configured`;
- `ai_disabled_by_default_without_settings` (карта без AiSettings — нейтралы стоят).

## 6. Этапы

1. **MVP Battle** (eval.rs + жадный 1-ply + Skip): ИИ-враг в локальном бою, `Опция10`
   переключает lookahead. Тесты блока Battle.
2. **MoveAndAttack + позиционные правила** (backline/flank/retreat).
3. **MapPolicy инфраструктура**: AiSettings в карте, парсинг, дефолты, валидатор.
4. **MVP Map**: perceive→assess→engage/retreat с гистерезисом; отладочный оверлей
   в wgpu_ui (радиус, текущее решение, power-бары) — критично для настройки.
5. **2-ply + спарринг-прогон**: бот против бота на фикстурах — смоук-стабильность
   (нет зависаний/пинг-понга ходов), баланс весов.
6. **Интеграция с PVP-комнатами** (`AiOpponent`) и редактором (форма policy).

## 7. Открытые вопросы

- Скорость боя у ИИ (пауза между ходами бота в UI, чтобы человек видел действия).
- Показывать ли «зону восприятия» нейтралов игроку (прозрачность правил)?
- Не даёт ли `hold_ground` + `pay_mercy` абуз (фарм нейтралов золото-флагом)?
- Где хранить AiSettings в существующих .DTm — отдельный sidecar-файл рядом с картой
  (`map.ai.json`), чтобы не трогать бинарный формат?
