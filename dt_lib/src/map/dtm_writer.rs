use std::{fs, io::Write, path::Path};

use bzip2::{write::BzEncoder, Compression};
use encoding_rs::WINDOWS_1251;
use zerocopy::{FromZeros, IntoBytes};

use crate::{
    battle::{army::Army, control::Relations},
    items::Item,
    map::{
        convert::{
            ArmyData, BuildingData, Decoration, EventData, FractionRelationsData, GarrisonUnit,
            HeroInfoData, LightOrEvent, ManyUnitsData, MapData, RecruitUnitData, RelationsData,
            SettingsData, UnitData,
        },
        event::{Cmp, Event, ItemCheck, Location},
        map::{GameMap, ScenarioVariant},
        object::{BuildingVariant, MapBuildingdata, ObjectType},
    },
    registry::GameInfo,
    time::time::Data,
};

pub const MAP_MAGIC: [u8; 8] = *b"AIpf\r\n\x13\x00";
pub const BZIP2_MAGIC: [u8; 4] = *b"BZh9";
pub const MAP_HEADER: [u8; 12] = *b"MapLDV V.4\r\n";
pub const PADDING_AFTER_SETTINGS: usize = 0x12F - 12 - 282;
pub const TEXT_SECTION_START: [u8; 8] = *b"\x08>-Text-";
pub const TEXT_END_MARKER: &str = "LIT";

/// Максимально длинные серии тайлов, разбитые кусками по 256 (поле count - u8, n - 1).
pub fn encode_rle(map: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < map.len() {
        let tile = map[i];
        let mut run = 1;
        while i + run < map.len() && map[i + run] == tile && run < 256 {
            run += 1;
        }
        out.push(tile);
        out.push((run - 1) as u8);
        i += run;
    }
    out
}

fn texts_to_bytes(texts: &[String]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&TEXT_SECTION_START);
    for s in texts {
        out.extend_from_slice(&WINDOWS_1251.encode(s.as_str()).0);
        out.push(0);
    }
    out.extend_from_slice(WINDOWS_1251.encode(TEXT_END_MARKER).0.as_ref());
    out.push(0);
    out
}

pub fn mapdata_to_dtm(data: &MapData) -> Result<Vec<u8>, String> {
    let map = encode_rle(&data.map);
    let decos: Vec<u8> = data.decos.iter().flat_map(|d| d.as_bytes().to_vec()).collect();
    let buildings: Vec<u8> = data
        .buildings
        .iter()
        .flat_map(|b| b.as_bytes().to_vec())
        .collect();
    let armies: Vec<u8> = data
        .armies
        .iter()
        .flat_map(|a| a.as_bytes().to_vec())
        .collect();
    let lanterns: Vec<u8> = data
        .lanterns
        .iter()
        .flat_map(|l| l.as_bytes().to_vec())
        .collect();
    let events: Vec<u8> = data
        .events
        .iter()
        .flat_map(|e| e.as_bytes().to_vec())
        .collect();

    let mut settings = data.settings.clone();
    settings.surface_size = map.len() as u32;
    settings.deco_size = decos.len() as u32;
    settings.buildings_size = buildings.len() as u32;
    settings.armies_size = armies.len() as u32;
    settings.lanterns_size = lanterns.len() as u32;
    settings.events_size = events.len() as u32;
    settings.text_start = (0x12F
        + map.len()
        + decos.len()
        + buildings.len()
        + armies.len()
        + lanterns.len()
        + events.len()
        + 8) as u32;

    let mut payload = Vec::new();
    payload.extend_from_slice(&MAP_HEADER);
    payload.extend_from_slice(settings.as_bytes());
    payload.extend_from_slice(&[0u8; PADDING_AFTER_SETTINGS]);
    payload.extend_from_slice(&map);
    payload.extend_from_slice(&decos);
    payload.extend_from_slice(&buildings);
    payload.extend_from_slice(&armies);
    payload.extend_from_slice(&lanterns);
    payload.extend_from_slice(&events);
    payload.extend_from_slice(&texts_to_bytes(&data.text));

    let mut encoder = BzEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(&payload).map_err(|_| "write payload")?;
    let compressed = encoder.finish().map_err(|_| "finish bzip2")?;

    let mut out = Vec::with_capacity(16 + compressed.len());
    out.extend_from_slice(&MAP_MAGIC);
    out.extend_from_slice(&[0u8; 4]);
    out.extend_from_slice(&compressed);
    Ok(out)
}

pub fn mapdata_to_file(data: &MapData, path: &Path) -> Result<(), String> {
    fs::write(path, mapdata_to_dtm(data)?).map_err(|e| e.to_string())
}

fn relations_to_data(r: &Relations) -> RelationsData {
    RelationsData {
        a: r.player as u8,
        b: r.ally as u8,
        c: r.neighbour as u8,
        d: r.enemy as u8,
    }
}

fn to_u8_id(id: usize, offset: usize) -> Option<u8> {
    u8::try_from(id + offset).ok().filter(|v| *v != 0)
}

fn troops_to_stacks(troops: &[(usize, u64)], units_len: usize) -> Option<[ManyUnitsData; 6]> {
    let mut stacks: Vec<(usize, u64, u64)> = Vec::new();
    for (id, level) in troops {
        if *id >= units_len || id + 1 > u8::MAX as usize {
            return None;
        }
        match stacks.iter_mut().find(|(s_id, s_level, _)| s_id == id && s_level == level) {
            Some((_, _, count)) => *count += 1,
            None => stacks.push((*id, *level, 1)),
        }
    }
    if stacks.len() > 6 {
        return None;
    }
    let mut out = [ManyUnitsData {
        id: 0,
        level: 0,
        amount: 0,
    }; 6];
    for (i, (id, level, count)) in stacks.into_iter().enumerate() {
        if count > 255 {
            return None;
        }
        out[i] = ManyUnitsData {
            id: (id + 1) as u8,
            level: level.min(u8::MAX as u64) as u8,
            amount: count as u8,
        };
    }
    Some(out)
}

fn army_to_data(army: &Army, id: usize, registry: &GameInfo) -> Option<ArmyData> {
    let mut data = ArmyData::new_zeroed();
    let (x, y) = army.pos;
    data.x = x.min(u16::MAX as usize) as u16;
    data.y = y.min(u16::MAX as usize) as u16;
    data.id = id.min(u8::MAX as usize) as u8;

    let main = army
        .troops
        .first()
        .map(|t| t.get().unit.id)
        .unwrap_or_default();
    if main >= registry.units.inner.len() {
        return None;
    }
    data.troops.main = UnitData {
        id: to_u8_id(main, 1)?,
        level: 0,
    };

    let rest: Vec<(usize, u64)> = army
        .troops
        .iter()
        .skip(1)
        .map(|t| {
            let troop = t.get();
            (troop.unit.id, troop.unit.lvl.lvl)
        })
        .collect();
    data.troops.troops = troops_to_stacks(&rest, registry.units.inner.len())?;

    for (i, slot) in army.inventory.iter().take(3).enumerate() {
        data.items_ids[i] = match slot {
            Some(item) => to_u8_id(item.index, 1)?,
            None => 0,
        };
    }

    data.activity = if army.active { 1 } else { 0 };
    Some(data)
}

fn hero_to_data(army: &Army, registry: &GameInfo) -> Option<HeroInfoData> {
    let mut data = HeroInfoData::new_zeroed();
    data.gold = army.stats.gold.min(u16::MAX as u64) as u16;
    data.mana = army.stats.mana.min(u16::MAX as u64) as u16;
    let rest: Vec<(usize, u64)> = army
        .troops
        .iter()
        .skip(1)
        .map(|t| {
            let troop = t.get();
            (troop.unit.id, troop.unit.lvl.lvl)
        })
        .collect();
    data.army_data = troops_to_stacks(&rest, registry.units.inner.len())?;
    let (x, y) = army.pos;
    data.x = x.min(u16::MAX as usize) as u16;
    data.y = y.min(u16::MAX as usize) as u16;
    for (i, slot) in army.inventory.iter().take(3).enumerate() {
        data.items[i] = match slot {
            Some(item) => to_u8_id(item.index, 1)?,
            None => 0,
        };
    }
    Some(data)
}

fn cmp_to_i16(v: &Option<Cmp<u64>>) -> Option<i16> {
    match v {
        None => Some(0),
        // from_dtm (to_cmp) encodes a negative i16 as `raw as u64` = 2^64 + v;
        // recover that original negative i16 exactly.
        Some(Cmp::G(x)) if *x >= u64::MAX - i16::MAX as u64 => Some(*x as i64 as i16),
        Some(Cmp::G(x)) => i16::try_from(*x).ok(),
        _ => Some(0),
    }
}

fn building_to_data(b: &MapBuildingdata, registry: &GameInfo) -> Option<BuildingData> {
    let mut data = BuildingData::new_zeroed();
    let (x, y) = b.pos;
    data.x = x.min(u16::MAX as usize) as u16;
    data.y = y.min(u16::MAX as usize) as u16;

    if let Some(obj) = registry
        .objects
        .inner
        .iter()
        .find(|o| o.index == b.id && matches!(o.obj_type, ObjectType::Building { .. } | ObjectType::Bridge { .. }))
    {
        match obj.obj_type {
            ObjectType::Building { group, variant } | ObjectType::Bridge { group, variant } => {
                data.picture_variant = group;
                data.picture_number = variant;
            }
            _ => {}
        }
        data.size_x = obj.size.0;
        data.size_y = obj.size.1;
    }

    data.variant = match &b.variant {
        BuildingVariant::Town => 1,
        BuildingVariant::Village(_) => 2,
        BuildingVariant::Castle => 3,
        BuildingVariant::Fort => 4,
        BuildingVariant::Tavern => 5,
        BuildingVariant::Market => 6,
        BuildingVariant::Church => 7,
        BuildingVariant::Forge => 8,
        BuildingVariant::Port => 9,
        BuildingVariant::Altar => 10,
        BuildingVariant::Mine => 11,
        BuildingVariant::Ruins(_) => 12,
        BuildingVariant::StoneBridge => 13,
        BuildingVariant::WoodenBridge => 14,
    };

    if b.events.len() > 64 {
        return None;
    }
    for (i, e) in b.events.iter().enumerate() {
        data.event_ids[i] = u16::try_from(*e + 1).ok()?;
    }
    data.event_amount = b.events.len() as u8;

    let (items, max_items) = match &b.variant {
        BuildingVariant::Ruins(items) => (items.clone(), 0),
        _ => match &b.market {
            Some(market) => (market.items.clone(), market.max_items),
            None => (Vec::new(), 0),
        },
    };
    if items.len() > 6 {
        return None;
    }
    for (i, item) in items.iter().enumerate() {
        data.artifact_ids[i] = u16::try_from(item.index + 1).ok()?;
    }
    data.number_of_artifacts_for_sale = max_items.min(u8::MAX as usize) as u8;
    if let Some(market) = &b.market {
        data.min_artifact_price = market.itemcost_range.0.min(u16::MAX as u64) as u16;
        data.max_artifact_price = market.itemcost_range.1.min(u16::MAX as u64) as u16;
    }

    data.gold_income = b.gold_income.min(u16::MAX as u64) as u16;
    data.mana_income = b.mana_income.min(u8::MAX as u64) as u8;
    data.max_gold_income = match &b.variant {
        BuildingVariant::Village(v) => v.max_gold.min(u16::MAX as u64) as u16,
        _ => 0,
    };
    data.max_mana_income = match &b.variant {
        BuildingVariant::Village(v) => v.max_mana.min(u8::MAX as u64) as u8,
        _ => 0,
    };

    if let Some(recruitment) = &b.recruitment {
        if recruitment.units.len() > 6 {
            return None;
        }
        for (i, unit) in recruitment.units.iter().enumerate() {
            data.recruits[i] = RecruitUnitData {
                id: to_u8_id(unit.unit, 1)?,
                amount: unit.count.min(u8::MAX as usize) as u8,
                max_amount: if unit.count == 0 { 1 } else { unit.count.min(u8::MAX as usize) as u8 },
            };
        }
    }

    let mut garrison: Vec<(usize, u64)> = Vec::new();
    for &id in &b.garrison {
        if id > 100 {
            return None;
        }
        match garrison.iter_mut().find(|(g_id, _)| *g_id == id) {
            Some((_, count)) => *count += 1,
            None => garrison.push((id, 1)),
        }
    }
    if garrison.len() > 6 || garrison.iter().any(|(_, count)| *count > 254) {
        return None;
    }
    for (i, (id, count)) in garrison.into_iter().enumerate() {
        data.garrison_units[i] = GarrisonUnit {
            id: (id + 1) as u8,
            level: 1,
            count: (count + 1) as u8,
        };
    }

    data.additional_garrison_defense = b.additional_defense.min(u8::MAX as u64) as u8;
    data.owner_army_id = match b.owner {
        None => 0xFF,
        Some(owner) => owner.min(u8::MAX as usize) as u8,
    };
    for (i, spell) in b.spells_to_learn.iter().take(5).enumerate() {
        data.spell_ids[i] = to_u8_id(*spell, 1)?;
    }
    data.group = b.group.min(u8::MAX as usize) as u8;
    data.relations = relations_to_data(&b.relations);
    Some(data)
}

fn event_to_data(event: &Event) -> Option<EventData> {
    let mut data = EventData::new_zeroed();
    data.event_type = match event.location {
        Location::Global => 1,
        Location::Local => 2,
        Location::Quest => 3,
        Location::Talks => 4,
        Location::Place => return None,
    };

    let conds = &event.conditions;
    let res = &event.result;

    data.subordinate_event_checkmark = if conds.sub { 1 } else { 0 };

    let year2000 = Data::YEAR as u64 * 2000;
    let relative_time = conds.activation_time.minutes >= year2000;
    if conds.relative_time && !relative_time {
        data.event_date = u32::try_from(year2000 + conds.activation_time.minutes).ok()?;
    } else {
        data.event_date = u32::try_from(conds.activation_time.minutes).ok()?;
    }

    data.multiple_event = if conds.repeat.is_some() { 1 } else { 0 };
    data.event_repeat = conds.repeat.as_ref().map(|t| t.minutes).unwrap_or_default().min(u16::MAX as u64) as u16;

    let yes_raw: Vec<u16> = conds
        .if_event_answ
        .as_ref()
        .map(|v| v.iter().filter(|(_, kind)| *kind == 0).map(|(id, _)| *id as u16).take(2).collect())
        .unwrap_or_default();
    let no_raw: Vec<u16> = conds
        .if_event_answ
        .as_ref()
        .map(|v| v.iter().filter(|(_, kind)| *kind == 1).map(|(id, _)| *id as u16).take(2).collect())
        .unwrap_or_default();
    let exec = conds.if_event_executed.is_some();

    data.happened_event_answ_yes_checkmark = if !yes_raw.is_empty() || exec { 1 } else { 0 };
    for i in 0..2 {
        data.happened_event_answ_yes_id[i] = yes_raw.get(i).copied().unwrap_or(0);
    }
    data.happened_event_answ_no_checkmark = if !no_raw.is_empty() { 1 } else { 0 };
    for i in 0..2 {
        data.happened_event_answ_no_id[i] = no_raw.get(i).copied().unwrap_or(0);
    }

    if let Some(ids) = &conds.if_event_not_executed {
        data.not_happened_event_checkmark = 1;
        for (i, id) in ids.iter().take(2).enumerate() {
            data.not_happened_event_id[i] = u16::try_from(*id + 1).ok()?;
        }
    }

    data.army_meet_id = match conds.army_meet {
        Some(x) => to_u8_id(x, 1)?,
        None => 0,
    };
    match &conds.armies_active {
        Some(v) if v.len() == 1 => data.army_active_id = to_u8_id(v[0], 1)?,
        Some(_) => return None,
        None => {}
    }
    match &conds.armies_inactive {
        Some(v) if v.len() == 1 => data.army_unactive_id = to_u8_id(v[0], 1)?,
        Some(_) => return None,
        None => {}
    }

    if let Some(ownership) = &conds.building_ownership {
        data.buildings_ownership = 1;
        for (i, (building, group)) in ownership.iter().take(3).enumerate() {
            data.building_id[i] = to_u8_id(*building, 1)?;
            data.building_ownership_group_id[i] = to_u8_id(*group, 1)?;
        }
    }

    if let Some(items_check) = &conds.items_check {
        data.existing_items = 1;
        for (i, (check, id)) in items_check.iter().take(3).enumerate() {
            data.item_id[i] = to_u8_id(*id, 1)?;
            data.existing_item_group_id[i] = match check {
                ItemCheck::PlayerHasNo => 6,
                _ => 0,
            };
        }
    }

    if let Some(ids) = &conds.armies_defeated_by_player {
        data.enemy_defeat_checkmark = 1;
        for (i, id) in ids.iter().take(2).enumerate() {
            data.army_defeat_id[i] = to_u8_id(*id, 1)?;
        }
    }
    if let Some(ids) = &conds.armies_defeated {
        data.army_already_defeat = 1;
        for (i, id) in ids.iter().take(2).enumerate() {
            data.defeat_army_id[i] = to_u8_id(*id, 1)?;
        }
    }

    data.current_level = cmp_to_i16(&conds.xp_req)?;
    data.current_gold = cmp_to_i16(&conds.gold_req)?;
    data.current_mana = cmp_to_i16(&conds.mana_req)?;
    data.unit_in_squad_amount = cmp_to_i16(&conds.army_req)?;
    data.army_strength = cmp_to_i16(&conds.power_req)?;
    data.hero_have_only_1hp_checkmark = if conds.hero_has_1_hp { 1 } else { 0 };
    data.hero_archetype = conds.archetype_req.map(|x| x.min(u8::MAX as usize) as u8).unwrap_or(0);
    if conds.in_building.is_some() {
        return None;
    }

    if let Some(ids) = &res.lit_lights {
        for (i, id) in ids.iter().take(4).enumerate() {
            data.light_activate_light[i] = u16::try_from(*id + 1).ok()?;
        }
    }
    data.event_delay_in_hours = res.delay.0.minutes.min(u16::MAX as u64) as u16;
    if let Some(sub_event) = &res.sub_event {
        data.subordinate_event_id = u16::try_from(sub_event.first().copied().unwrap_or(0) + 1).ok()?;
    }
    if let Some(delayed) = &res.delayed_event {
        data.relative_event = u16::try_from(delayed.event + 1).ok()?;
        data.relative_event_time_in_hours = (delayed.time.minutes / 60).min(u16::MAX as u64) as u16;
    }
    if let Some(ids) = &res.minus_items {
        for (i, id) in ids.iter().take(4).enumerate() {
            data.item_remove_id[i] = to_u8_id(*id, 1)?;
        }
    }
    if let Some(ids) = &res.plus_items {
        for (i, id) in ids.iter().take(4).enumerate() {
            data.item_add_id[i] = to_u8_id(*id, 1)?;
        }
    }
    if res.question.is_some() {
        data.confirm_question = 1;
    }
    if let Some(ids) = &res.activate_armies {
        for (i, id) in ids.iter().take(2).enumerate() {
            data.army_activate_id[i] = to_u8_id(*id, 1)?;
        }
    }
    match &res.deactivate_armies {
        Some(v) if v.len() == 1 => data.army_deactivate_id = to_u8_id(v[0], 1)?,
        Some(_) => return None,
        None => {}
    }
    data.army_from_start_fight_id = match res.start_battle_with {
        Some(x) => to_u8_id(x, 1)?,
        None => 0,
    };
    data.event_quest_complete_id = match res.complete_quest {
        Some(x) => u16::try_from(x + 1).ok()?,
        None => 0,
    };
    if let Some(ids) = &res.learn_spells {
        for (i, id) in ids.iter().take(4).enumerate() {
            data.spell_learn_id[i] = to_u8_id(*id, 1)?;
        }
    }
    data.change_xp = i16::try_from(res.change_xp).ok()?;
    data.change_gold = i16::try_from(res.change_gold).ok()?;
    data.change_mana = i16::try_from(res.change_mana).ok()?;
    if let Some(ids) = &res.add_units {
        for (i, id) in ids.iter().take(4).enumerate() {
            data.unit_add_id[i] = to_u8_id(*id, 1)?;
        }
    }
    if let Some(ids) = &res.remove_units {
        for (i, id) in ids.iter().take(4).enumerate() {
            data.unit_quit_id[i] = to_u8_id(*id, 1)?;
        }
    }
    if res.change_personality.is_some() {
        return None;
    }

    Some(data)
}

pub fn gamemap_to_mapdata(mapa: &GameMap, events: &[Event], registry: &GameInfo) -> Option<MapData> {
    let (normal, heroes) = if mapa.armys.len() >= 3 {
        mapa.armys.split_at(mapa.armys.len() - 3)
    } else {
        (mapa.armys.as_slice(), &[][..])
    };

    let mut settings = SettingsData::new_zeroed();
    let size = mapa.tilemap.size;
    settings.size_x = size.min(u32::MAX as usize) as u32;
    settings.size_y = size.min(u32::MAX as usize) as u32;
    settings.seed = mapa.start.seed.min(u32::MAX as usize) as u32;
    settings.start_time = mapa.time.minutes.min(u32::MAX as u64) as u32;
    settings.winning_event_id = mapa.start.winning_event_id.min(u16::MAX as usize) as u16;
    settings.losing_event_id = mapa.start.losing_event_id.min(u16::MAX as usize) as u16;
    settings.scenario_variant = match &mapa.start.scenario {
        ScenarioVariant::Single => 0,
        ScenarioVariant::Start(_) => 1,
        ScenarioVariant::Series(_) => 2,
    };
    settings.global_relations = FractionRelationsData {
        a: RelationsData { a: 0, b: 0, c: 0, d: 0 },
        b: relations_to_data(&mapa.relations.ally),
        c: relations_to_data(&mapa.relations.neighbour),
        d: relations_to_data(&mapa.relations.enemy),
    };
    if let Some(hero) = heroes.first() {
        settings.knight_data = hero_to_data(hero, registry)?;
    }
    if let Some(hero) = heroes.get(1) {
        settings.mage_data = hero_to_data(hero, registry)?;
    }
    if let Some(hero) = heroes.get(2) {
        settings.ranger_data = hero_to_data(hero, registry)?;
    }

    let armies: Vec<ArmyData> = normal
        .iter()
        .enumerate()
        .map(|(id, army)| army_to_data(army, id, registry))
        .collect::<Option<_>>()?;

    let buildings: Vec<BuildingData> = mapa
        .buildings
        .iter()
        .map(|b| building_to_data(b, registry))
        .collect::<Option<_>>()?;

    let events_data: Vec<EventData> = events.iter().map(event_to_data).collect::<Option<_>>()?;

    let mut map = Vec::with_capacity(size * size);
    for x in 0..size {
        for y in 0..size {
            map.push(mapa.tilemap.inner[y + x * size].min(u8::MAX as usize) as u8);
        }
    }

    let decos: Vec<Decoration> = mapa
        .decomap
        .iter()
        .map(|d| Decoration {
            x: d.x.min(u16::MAX as usize) as u16,
            y: d.y.min(u16::MAX as usize) as u16,
            id: d.index.min(u16::MAX as usize) as u16,
        })
        .collect();

    let mut lanterns = Vec::new();
    for (i, events_at) in mapa.eventmap.inner.iter().enumerate() {
        if events_at.is_empty() {
            continue;
        }
        if events_at.len() > 32 {
            return None;
        }
        let mut ids = [0u8; 32];
        for (k, id) in events_at.iter().enumerate() {
            ids[k] = to_u8_id(*id, 1)?;
        }
        let x = i / size;
        let y = i % size;
        lanterns.push(LightOrEvent {
            x: x.min(u16::MAX as usize) as u16,
            y: y.min(u16::MAX as usize) as u16,
            id: 0,
            map_model: 9,
            events: ids,
            light_radius: 0,
            _empty1: [0; 60],
        });
    }

    let mut text = Vec::new();
    text.push(mapa.start.name.clone());
    text.push(mapa.start.description.clone());
    text.push(String::new());
    text.push(match &mapa.start.scenario {
        ScenarioVariant::Single => String::new(),
        ScenarioVariant::Start(next) | ScenarioVariant::Series(next) => next.clone(),
    });
    for building in &mapa.buildings {
        text.push(building.name.clone());
        text.push(building.desc.clone());
        text.push(building.owner_name.clone());
    }
    for army in normal {
        text.push(army.stats.army_name.clone());
        text.push(String::new());
        text.push(String::new());
    }
    for event in events {
        text.push(event.name.clone());
        text.push(event.result.question.as_ref().map(|q| q.0.clone()).unwrap_or_default());
        text.push(event.message.clone().unwrap_or_default());
    }

    Some(MapData {
        settings,
        buildings,
        map,
        decos,
        armies,
        lanterns,
        events: events_data,
        text,
    })
}

pub fn gamemap_to_dtm(mapa: &GameMap, events: &[Event], registry: &GameInfo) -> Option<Vec<u8>> {
    mapdata_to_dtm(&gamemap_to_mapdata(mapa, events, registry)?).ok()
}

#[cfg(test)]
mod test {
    use bytes::Bytes;

    use crate::{
        map::{
            convert::{convert_dtm_map, parse_dtm_map_by_bytes},
            dtm_writer::{gamemap_to_mapdata, mapdata_to_dtm},
            event::Event,
            map::GameMap,
        },
        parse::{parse_objects, parse_units, FileAccess},
        registry::GameInfo,
    };

    struct StaticReader;

    impl FileAccess for StaticReader {
        async fn read(path: &str) -> Vec<u8> {
            match path {
                "Units.ini" => include_bytes!("../../../dt/Units.ini").to_vec(),
                "Objects.ini" => include_bytes!("../../../dt/Objects.ini").to_vec(),
                _ => Vec::new(),
            }
        }
    }

    fn assert_game_maps_eq(a: &GameMap, b: &GameMap) {
        assert_eq!(a.start, b.start);
        assert_eq!(a.time, b.time);
        assert_eq!(a.tilemap.size, b.tilemap.size);
        assert_eq!(a.tilemap.inner, b.tilemap.inner);
        assert_eq!(a.decomap, b.decomap);
        assert_eq!(a.eventmap.size, b.eventmap.size);
        assert_eq!(a.eventmap.inner, b.eventmap.inner);
        assert_eq!(a.buildings, b.buildings);
        assert_eq!(a.relations, b.relations);
        assert_eq!(a.pause, b.pause);
        assert_eq!(a.armys.len(), b.armys.len());
        for (i, (x, y)) in a.armys.iter().zip(b.armys.iter()).enumerate() {
            assert_eq!(x.pos, y.pos, "army {i} pos");
            assert_eq!(x.active, y.active, "army {i} active");
            assert_eq!(x.control, y.control, "army {i} control");
            assert_eq!(x.stats.army_name, y.stats.army_name, "army {i} name");
            assert_eq!(x.stats.gold, y.stats.gold, "army {i} gold");
            assert_eq!(x.stats.mana, y.stats.mana, "army {i} mana");
            assert_eq!(x.inventory, y.inventory, "army {i} inventory");
            assert_eq!(x.troops.len(), y.troops.len(), "army {i} troops len");
            for (j, (t1, t2)) in x.troops.iter().zip(y.troops.iter()).enumerate() {
                let (a, b) = (t1.get(), t2.get());
                assert_eq!(a.unit, b.unit, "army {i} troop {j} unit");
            }
        }
    }

    fn assert_events_eq(a: &[Event], b: &[Event]) {
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(b.iter()) {
            assert_eq!(x.name, y.name);
            assert_eq!(x.location, y.location);
            assert_eq!(x.conditions, y.conditions);
            assert_eq!(x.result, y.result);
            assert_eq!(x.message, y.message);
            assert_eq!(x.id, y.id);
        }
    }

    #[test]
    fn mapdata_roundtrip_is_identity() {
        let buf = include_bytes!("../../../dt/Maps_Rus/Другой берег.DTm");
        let data = parse_dtm_map_by_bytes(Bytes::copy_from_slice(buf)).unwrap();
        let bytes = mapdata_to_dtm(&data).unwrap();
        assert_eq!(&bytes[..4], b"AIpf");
        assert_eq!(&bytes[12..16], b"BZh9");
        let parsed = parse_dtm_map_by_bytes(Bytes::copy_from_slice(&bytes)).unwrap();
        assert_eq!(parsed.settings, data.settings);
        assert_eq!(parsed.map, data.map);
        assert_eq!(parsed.decos, data.decos);
        assert_eq!(parsed.buildings, data.buildings);
        assert_eq!(parsed.armies, data.armies);
        assert_eq!(parsed.lanterns, data.lanterns);
        assert_eq!(parsed.events, data.events);
        assert_eq!(parsed.text, data.text);
    }

    #[tokio::test]
    async fn gamemap_roundtrip_via_binary() {
        let mut registry = GameInfo::new();
        parse_units::<StaticReader>(Some("Units.ini"), &mut registry)
            .await
            .expect("units");
        parse_objects::<StaticReader>(&mut registry).await;

        let buf = include_bytes!("../../../dt/Maps_Rus/Другой берег.DTm");
        let data = parse_dtm_map_by_bytes(Bytes::copy_from_slice(buf)).unwrap();
        let (mapa, events) = convert_dtm_map(data.clone(), &registry);
        let conv_data = gamemap_to_mapdata(&mapa, &events, &registry).expect("mapdata");
        let bytes = mapdata_to_dtm(&conv_data).unwrap();
        let data2 = parse_dtm_map_by_bytes(Bytes::copy_from_slice(&bytes)).unwrap();
        let (mapa2, events2) = convert_dtm_map(data2, &registry);

        assert_game_maps_eq(&mapa2, &mapa);
        assert_events_eq(&events2, &events);
    }
}