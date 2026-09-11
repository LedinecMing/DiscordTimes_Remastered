# NOTES_battle_logic — контракт боевой логики

## Архитектура интеракции
```
клик по карточке (wgpu_ui::battle_view::draw_battle)
  → handle_action((pos_clicked, army_clicked), battle, armies, registry)   [battlefield.rs]
    → unit_interaction(battle, armies, (army, BattleUnitInfo::Pos(pos)), registry)
        • пустая клетка своей армии → Move (маневр)
        • свой юнит == активный     → селф-каст (self-heal, если direction позволяет)
        • иначе                     → let effects = unit::attack(me, target, &*armies, registry)?
                                      unit::apply_attack(effects, me, target, armies, battle, registry)
        • ход (-1 moves) списывается ТОЛЬКО если attack вернул Some
    → battle.after_single_move(armies, registry)
        recalc_hitmaps → check_win → check_row_fall → search_next_active
        → can_interact = search_interactions(...)   [пересчитывается после КАЖДОГО действия
         и на rollover хода — тест turn_rollover_recomputes_can_interact]
```

## Чистый `attack` / исполнитель `apply_attack` (dt_lib/src/units/unit.rs)
- `pub fn attack(me, target, armies: &Vec<Army>, registry) -> Option<ActionResult>` — **ЧИСТАЯ**: только
  валидация и классификация. Никаких мутаций armies/battle. Мутации — только в
  `apply_attack(effects, me, target, armies, &mut armies, battle, registry) -> u64` (исполнитель).
- `ActionResult` = { Buff, Debuff, MagicDamage, Melee, Ranged, Move }. `MagicDamage` — урон
  наносящая магия (StrikeOnly/фоллбеки), это НЕ Debuff (Debuff = проклятие).
- Классификация: melee → Melee; ranged → Ranged; союзная магия (heal/bless/elemental) → Buff;
  вражеское проклятие → Debuff; уроновая магия → MagicDamage.
- Пер-таргет ограничения внутри match: тип цели × тип магии × висящие эффекты
  (blessed/cursed/elemental_sup проверяются по **effect id**, см. ниже).
- Позиционные правила (актуальная семантика, подтверждена тестами):
  - `field_checks` («общий строй») нужен только melee/ranged.
  - Магия по врагам: `enemy_field == Front && is_path_empty && dist > 1` ИЛИ `enemy_field == Back && is_front_empty` ИЛИ маг в Back.
  - Магия по союзникам: без полевых ограничений (лечение из любого ряда).
  - **Резерв** (Field::Reserve = слоты {0, 5, 6, 11} при max=12):
    - враг в резерве недостижим ВСЕГДА (`if is_enemy && enemy_in_reserve { return None }`);
    - маг в резерве без перка `settings.attacks_from_reserve` действует ТОЛЬКО на союзников в резерве
      (`if me_in_reserve && !can_reserve && !(!is_enemy && enemy_in_reserve) { return None }`);
    - оба союзника в резерве — взаимодействовать можно (это field_checks).
  - Нельзя лечить полного по HP (CureOnly), нельзя повторно благословлять/проклинать (эффект уже висит).

## Жизненный цикл эффектов (dt_lib/src/effects/effect.rs + units/unit.rs)
- `StatusEffect { id, internal, lifetime { lifetime: Option<usize>, decay, remove_on_battle_end }, power, added_modify }`.
- `add_modify_effect(unit, modify, id, registry) -> bool`: сразу `unit.modify += modify` (статы меняются
  ТОЛЬКО после следующего `recalc`), затем `add_effect` (стакание/пуш). `added_modify: Some(modify)` —
  используется при откате.
- `Unit::tick` — ЕДИНСТВЕННОЕ место спада: lifetime `-decay`, истёкшие снимаются через
  `StatusEffect::removal()` (modify -= added_modify), затем recalc. Блесс/курс спадают на следующий ход.
- `Unit::on_battle_end` — снимает эффекты с флагом `remove_on_battle_end: true` с откатом; `Troop::on_battle_end` делегирует на юнит.
- Стакание: `stacks=true` → повторное наложение суммирует lifetime/power (add_effect возвращает true);
  `stacks=false` → второй add возвращает false, слот один (но `unit.modify += modify` в
  add_modify_effect применяется всё равно).
- `StatusEffect::on_tick` (decay + Turn-правила бонусов) НЕ вызывается из Unit::tick — Turn-правила
  бонусов тикают отдельно в `next_move_seq` Phase 2. Если понадобятся Turn-механики эффектов —
  подключать осознанно.

## Резерв-матрица (макс 12 слотов, columns=6; Reserve = {0, 5, 6, 11}, Back = 1–4, Front = 7–10)
| Маг | Цель | Доступно |
|---|---|---|
| в резерве, без перка `attacks_from_reserve` | союзник в резерве | ДА (оба в резерве — field_checks) |
| в резерве, без перка | союзник вне резерва / враг | НЕТ |
| в резерве, с перком | любые допустимые цели | ДА |
| вне резерва | враг в резерве | НЕТ никогда |

## id эффектов (конвенция, поддержана тестами)
0 = internal, 1 = mage_support (блесс, stacks=true), 2 = mage_curse (stacks=false в тестах
→ первый каст «ложится» в modify, повтор = None → фоллбек MagicDamage), 3 = elemental_support,
4 = abstract (стакающийся, для тестов спада). Классификация в `attack` смотрит эффекты через
`has_effect_id(1/2/3)` — при изменении Effects.ini-порядка синхронизировать make_test_registry.

## Дедлок-гигиена (SendMut)
- `SendMut<T> = Arc<Mutex<T>>`, `get()` — блокирующий. В `get()` стоит сторож: удержание >5 c
  → паника с бэктрейсом (не тихий дедлок). Два исторических сайта: guard во время `search_interactions`
  (лечится снапшотом позиций до вызова attack) и повторный `.get()` той же ячейки в UI.
- Тесты, держащие `troops[i].get()` через binding и вызывающие attack — дедлок. Отпускай guard'ы.

## Тестовые конвенции (dt_lib/src/battle/tests.rs)
- `make_test_registry()`: юниты 0–19 generic (1 = Undead, прочие People), 20 = melee (hand 30),
  21 = defender, 22 = Death-маг ToAll (magic 40), 23 = Life-маг, 24 = Elemental-маг ToEnemy (25).
  Эффекты: 0 internal, 1 mage_support, 2 mage_curse, 3 elemental_support, 4 abstract.
- `make_army_from_ids(&registry, &[(id, slot)])` — юниты на позициях hitmap (slot → UnitPos::from_index(slot, 6)).
- `battle_with_active(&armies, active, &registry)` — BattleInfo c active_unit и can_interact.
- `perform_attack` / `handle_action((pos, army), ...)` — полный путь интеракции.
- ВАЖНО: `modified` пересчитывается от реестровой базы (`recalc`), а не от крафтовых статов
  `make_test_unit` — base-статы в тестах брать из `registry.units[id].stats`.
- Не держать guard'ы (`troops[i].get()`) через вызовы attack/handle_action.