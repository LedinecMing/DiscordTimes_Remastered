# NOTES_advini_parsing — как парсится INI

## Состав
- `advini` — рантайм: `parse_for_sections(&str) -> Vec<(имя_секции, IndexMap<ключ, значение>)>`
  (синхронный, ключи как в файле), трейты `Ini` (одно значение: `eat`/`vomit`) и `Sections`
  (структура из секции: `from_section`/`to_section`), blanket-impl для `Option<T>`, `Vec<T>`, чисел, `String`.
- `advini_derive` — proc-macro `derive(Sections)` и `derive(Ini)` (для энумов).
  Атрибуты: `#[alias = "ключ"]` (может быть несколько через повтор), `#[default_value = "expr"]`
  (строка-литерал, парсится как Expr!), `#[unused]`, `#[inline_parsing]`, `#[additional]`.

## ГЛАВНЫЙ ГРАБЕЛЬ (найден и исправлен, помни при правках)
Кодоген `Ini` для энумов (`match_quote` в advini_derive/src/lib.rs) матчит распарсенное
имя варианта, а не остаток ном-парсинга:
```rust
let (__input, res) = <String as advini::Ini>::eat(__input, _additional)?;
match res.as_str() { /* варианты */ _ => Err("Wrong variant!") }
```
История: раньше матчился `__input` (остаток после String::eat — пустая строка) → ЛЮБОЕ
энум-поле любого INI молча падало в `#[default_value]` (unit_type/magic_type/magic_direction —
все дефолты). Симптом был невидимый: from_section возвращает Ok, поле = default.
Если энум-поле снова «не парсится» — первым делом смотреть сюда.

Ещё грабли:
- Поле БЕЗ `#[default_value]` при ошибке разбора проваливает ВСЮ секцию (`?` в кодогене) →
  parse_* паникует. Держи default у опциональных полей.
- Значения энумов в INI должны буквально совпадать с именами вариантов
  (`Magic=ElementalMagic` ↔ `enum MagicType { ElementalMagic, ... }`).
- Ключи в секции регистрозависимы (alias'ы в derive пишутся lowercase: `#[alias = "magic"]`
  матчится с ключом `Magic` — сверяй фактическим прогоном, см. тест ниже).
- `parse_string_from_string`: значение без кавычек ест до запятой целиком; кавычки опциональны.
- Индекс-регистры (`Registry<T>`) индексируются ПОЗИЦИЕЙ в `inner` — эффекты в тестах и в
  Effects.ini должны совпадать по порядку (id — это индекс `inner`, не строковый `id: String`).

## Маппинг полей Units.ini → UnitInfo (проверен тестом units_ini_archmage_full_section_parses)
| Ключ INI | Поле |
|---|---|
| `GlobalIndex` / `IconIndex` | `icon_index` (оба алиасятся; побеждает первый найденный — сейчас GlobalIndex) |
| `Name` / `Descript` | `name` / `descript` |
| `Magic` | `magic_type: Option<MagicType>` (`#[alias="magic"]`; None, если ключа нет) |
| `MagicDirection` | `magic_direction` (default ToAll) |
| `Nature` | `unit_type` (`#[alias="nature"]`, default People) |
| `Hits` | `stats.max_hp` (alias "hits"); `stats.hp` — отдельный ключ "hp", в INI нет → дефолт 1 |
| `MagicPower` | `stats.damage.magic` (Power alias "magicpower"); ranged = "attackshot", hand = "attackblow" |
| `ProtectLife` / `ProtectDeath` / `ProtectElemental` | `stats.defence.life_magic/death_magic/elemental_magic` |
| `Initiative` | `stats.speed` |
| `Manevres` | `stats.max_moves` (alias "manevres"); `moves` — отдельный ключ |
| `d-Hits`, `d-MagicPower`, ... | `lvl: LevelUpInfo` (inline) — поуровневые модификаторы |
| `Surrender`, `Bonus` | `surrender: Option<u64>`, `bonus: Option<String>` |

## Тест-паттерн для парсинга
Секции копируются из реального INI дословно в `advini::parse_for_sections(ini)` (синхронно,
без Reader) → `UnitInfo::from_section(sec, Default::default())` (требует импорта `advini::Sections`).
Пример: `dt_lib/src/battle/tests.rs::units_ini_archmage_full_section_parses` — держать зелёным;
он фиксирует маппинг и моментально ловит регрессии кодогена.