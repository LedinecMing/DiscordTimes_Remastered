use super::{deco::*, map::*};
use bufread::BzDecoder;
use bytes::*;
use bzip2::*;
use encoding_rs::*;
use std::{
    fs,
    fs::File,
    io,
    io::{Read, Write},
    path::Path,
};
use zerocopy::{FromBytes, FromZeros, IntoBytes, Unaligned};
use num_enum::{Default, FromPrimitive, IntoPrimitive};

pub trait ToBool {
	fn to_bool(self) -> bool;
}
impl ToBool for u8 {
	fn to_bool(self) -> bool {
		self == 0x1
	}
}
#[derive(IntoPrimitive, FromPrimitive, Debug)]
#[repr(u16)]
pub enum MapModel {
	None,
	Knight,
	Mage,
	Archer,
	Pheudal,
	Rogue,
	Peasant,
	Inactive,
	Light,
	#[num_enum(default)]
	Mine,
	Necro,
	Ghost,
	Zombie,
}
#[derive(FromBytes, Unaligned, Debug, Copy, Clone, PartialEq, Eq)]
#[repr(C)]
pub struct UnitData {
	pub id: u8,
	pub level: u8,
}
#[derive(FromBytes, Unaligned, Debug, Copy, Clone, PartialEq, Eq)]
#[repr(packed(1))]
pub struct ManyUnitsData {
	pub id: u8,
	pub amount: u8,
	pub level: u8,
}
#[derive(FromBytes, Unaligned, Debug, Copy, Clone, PartialEq, Eq)]
#[repr(packed(1))]
pub struct RecruitUnit {
    pub id: u8,
    pub amount: u8,
    pub max_amount: u8,
}
#[derive(FromBytes, Unaligned, Debug, Copy, Clone, PartialEq, Eq)]
#[repr(packed(1))]
pub struct GarrisonUnit {
    pub id: u8,
    pub level: u8,
    pub count: u8,
}
#[derive(FromBytes, Unaligned, Debug, Copy, Clone, PartialEq, Eq)]
#[repr(packed(1))]
pub struct ArmyTroopsData {
	pub main: UnitData,
	pub troops: [ManyUnitsData; 6]
}
#[derive(FromBytes, Unaligned, Debug, Copy, Clone, PartialEq, Eq)]
#[repr(C)]
pub struct PatrolData {
	pub exists: u8,
	pub radius: u8
}
#[derive(FromBytes, Unaligned, Debug, Copy, Clone)]
#[repr(C)]
pub struct RelationsData {
	pub a: u8,
	pub b: u8,
	pub c: u8,
	pub d: u8
}
#[derive(FromPrimitive)]
#[repr(u8)]
pub enum ShipData {
	#[num_enum(default)]
	Hero = 0,
	Pirate = 1,
	Trader = 2
}
#[repr(u8)]
pub enum ActionModel {
	Aggressive = 1,
	Passive = 2,
	Keeper = 3,
	Trading = 4
}
#[derive(FromBytes, Unaligned, Debug)]
#[repr(packed(1))]
pub struct ArmyData {
	pub x: u16, // широта  2
	pub y: u16, // долгота 4
	pub id: u8, // айди армии 5
	pub map_model: u8, // моделька на карте 6
	pub tactic_cost: u16, // тактическая стоимость 8 
	pub _empty: [u8; 5], // 5 пустых байт 13
	pub speed_correction: u8, // коррекция скорости 14
	pub xp_like_player: u8, // опыт как у игрока 15
	pub _empty0: [u8; 2], // 2 пустых байта 17
	pub gold_income: u16, // золотой доход 19
 	pub xp_add: u16, // добавляемый при найме опыт 21
	pub _empty1: [u8; 4], // 4 пустых байта 25
	pub start_building_id: u8, // айди стартового здания армии 26
	pub troops: ArmyTroopsData, // отряд игрока 46
	pub _empty2: [u8; 4], // 4 пустых байта 50
	pub items_ids: [u8; 3], // 3 айди артефактов 53
	pub _empty3: [u8; 5], // 5 пустых байт 58 
	pub named_unit_id: u8, // айди именного персонажа 59
	pub _empty4: u8, // 1 пустой байт 60
	pub patrol: PatrolData, // патруль 62
	pub units_without_money: u8, // персонажи без денег 63
	pub activity: u8, // активность армии 64
	pub group_type: u8, // группа армии 65
	pub relations: RelationsData, // отношения с группами 69
	pub agression: u8, // агрессия армии 70
	pub revive_time: u8, // время возрождения 71
	pub xp_correction: u8, // коррекция опыта 72
	pub ship_type: u8, // тип корабля 73
	pub _empty5: u8, // 1 пустой байт 74
	pub tactic_cost_part2: u16, // тактическая стоимость часть 2 76
	pub ignores_ai_armys: u8, // игнорирует армии ии 77
	pub goes_towards_player: u8, // идет к игроку 78
	pub forbid_random_targets: u8, // запрет случайных целей 79 
	pub forbid_talks: u8, // запрет случайных разговоров 80
	pub _known: u8, // хуй пойми что 0x04 81 
	pub not_interested_in_buildings: u8, // не заинтересован в зданиях 82 
	pub garrison_power_in_buildings: u8, // сила гарнизона в зданиях 83
	pub revive_everyone: u8, // возрождение всего отряда 84
	pub applied_spell: u8, // примененное заклинание 85
	pub action_model: u8, // модель поведения 86
	pub _empty6: [u8; 3], // 3 пустых байта 89 
}

#[derive(FromBytes, Unaligned, Debug)]
#[repr(packed(1))]
pub struct EventData {
    pub event_color: u8, // 1
    pub event_type: u8, // 2 байт обозначает тип события(глобальное/локальное/задание/сплетни).
    pub event_date: u32, // 3-6 4 байта, гранящие дату срабатывания события в минутах (отсчет идет с 00 ч 01 д 01 м 0000 г; то есть 01 ч 01 д 01 м 0000 года будет иметь 32-битное значение 60)
    pub event_repeat: u16, // 7-8 2 байта под время повтора события - повтор каждый N день(также в минутах, в редакторе настраиваются дни)
    pub event_duration: u16, // 9-10 2 байта под длительность срабатывания события(в минутах, в редакторе настраиваются часы)
    pub hero_archetype: u8, // 11 байт архетип персонажа 00-для всех, 01-рыцарь, 02-архимаг, 03-следопыт.
    pub unit_in_squad_amount: i16, // 12-13 2 байта со значениям количества юнитов в отряде(число отрядов) 16-битное число со знаком
    pub army_strength: i16, // 14-15
    pub army_unactive_id: u8, // 16
    pub army_id_change_patrol: u8, // 17
    pub change_patrol: i8, // 18
    pub current_specifications_checkmark: u8, // 19
    pub current_level: i16, // 20-21 2 байта со значениями текущего уровня 16-битное число со знаком
    pub current_gold: i16, // 22-23
    pub _empty: [u8; 2], // 24-25
    pub current_mana: i16, // 26-27
    pub _empty1: [u8; 2], // 28-29
    pub buildings_ownership: u8, // 30
    pub building_id: [u8; 3], // 31-33
    pub building_ownership_group_id: [u8; 3], // 34-36 байт id первой группы(чье строение:  игрока\зеленых\синих\желтых\красных\не игрока) для принадлежности строений)
    pub nominal_squad_in_army_checkmark: u8, // 37
    pub unit_id: [u8; 3], // 38-40
    pub nominal_unit1_id: [u8; 3], // 41-43 байт id первого именного юнита(id юнита заменяется при этом на id юнита именного юнита)
    pub army_ownership_group_id: [u8; 3], // 44-46 байт id первой группы(чья армия:  игрока\зеленых\синих\желтых\красных\не игрока) для принадлежности юнитов(отрядов))
    pub existing_items: u8, // 47
    pub item_id: [u8; 3], // 48-50
    pub existing_item_group_id: [u8; 3], // 51-53 байт id первой группы(чей артефакт:  игрока\зеленых\синих\желтых\красных\не игрока) для принадлежности артефактов)
    pub enemy_defeat_checkmark: u8, // 54
    pub army_defeat_id: [u8; 2], // 55-56
    pub happened_event_answ_yes_checkmark: u8, // 57
    pub happened_event_answ_yes_id: [u16; 2], // 58-61 2 байта id первого события указанного в меню произошедшее событие, ответ "да"
    pub not_happened_event_checkmark: u8, // 62
    pub not_happened_event_id: [u16; 2], // 63-66 2 байта id первого события указанного в меню не произошедшее событие
    pub army_already_defeat: u8, // 67
    pub defeat_army_id: [u8; 2], // 68-69
    pub happened_event_answ_no_checkmark: u8, // 70
    pub happened_event_answ_no_id: [u16; 2], // 71-74 2 байта id первого события указанного в меню произошедшее событие, ответ "нет"
    pub army_meet_id: u8, // 75 байт id армии для встречи(встреча с армией)
    pub army_active_id: u8, // 76 байт id армии которая активна(проверка армия активна)
    pub confirm_question: u8, // 77
    pub relative_event: u16, // 78-79
    pub relative_event_time_in_hours: u16, // 80-81
    pub activated_spell_id: u8, // 82
    pub event_picture: u8, // 83 байт встроенная картинка к событию (200-поражение, 201 победа, или id юнита)
    pub change_xp: i16, // 84-85
    pub change_gold: i16, // 86-87
    pub _empty2: [u8; 2], // 88-89
    pub change_mana: i16, // 90-91
    pub _empty3: [u8; 2], // 92-93
    pub spell_learn_id: [u8; 4], // 94-97
    pub unit_add_id: [u8; 4], // 98-101
    pub unit_nominal_add_id: [u8; 4], // 102-105 байт id первого именного добавляемого юнита(не меняет id добавляемого юнита)
    pub unit_quit_id: [u8; 4], // 106-109 байт id первого уходящего юнита или  FE(добавленный юнит), FF(любой юнит)
    pub unit_nominal_quit_id: [u8; 4], // 110-113
    pub item_add_id: [u8; 4], // 114-117
    pub item_remove_id: [u8; 4], // 118-121
    pub army_activate_id: [u8; 2], // 122-123
    pub army_deactivate_id: u8, // 124
    pub event_quest_complete_id: u16, // 125-126
    pub event_delay_in_hours: u16, // 127-128
    pub light_activate_light: [u16; 4], // 129-136
    pub army_unit_leave_id: u8, // 137
    pub unit_change_hero_army: u8, // 138
    pub subordinate_event_id: u16, // 139-140
    pub subordinate_event_checkmark: u8, // 141 байт галочка подчиненное событие(0- выключено, 1 включено). Редактор при переключении обнуляет дату срабатывания события, его длительность, время повтора, ставит галочку многократное событие. 
    pub multiple_event: u8, // 142 байт галочка многократное событие(0-включено, 1-выключено).
    pub army_from_unit_leave_id: u8, // 143 байт id армии из которой уходит юнит
    pub move_to_hero_checkmark: u8, // 144
    pub shown_army_id: u8, // 145
    pub hero_have_only_1hp_checkmark: u8, // 146
    pub army_in_native_building_id: u8, // 147
    pub army_from_start_fight_id: u8, // 148
    pub army_not_met_checkmark: u8, // 149
    pub repeat_after_answ_yes_checkmark: u8, // 150
    pub _empty4: [u8; 14], // 151-163
    pub custom_picture_for_event: u16, // 164-???(165) кол-во байт кастомной картинки к событию, которая лежит в файле после текстовых данных.
    pub _empty5: [u8; 6], // 166-171

#[repr(packed(1))]
#[derive(FromBytes, Unaligned, Debug, Clone)]
pub struct BuildingData {
    pub x: u16, // широта 2
    pub y: u16, // долгота 4
    pub picture_number: u8, // номер картинки типа строения 5
    pub picture_variant: u8, // тип картинки строения 6
    pub variant: u8, // тип строения 7
	pub _empty0: [u8; 1], // 8
    pub event_ids: [u16; 64], // айди событий в максимальном количестве 64 штуки 136
    pub artifact_ids: [u16; 6], // на рынке 5 штук, в руинах 4 максимально 148
	pub _empty_big: [u8; 117], //
    pub recruits: [RecruitUnit; 6], // найм в казармах 282
    pub gold_income: u16, // золотой доход 284
	pub max_gold_income: u16, // максимальный золотой доход? 286
	pub _empty: [u8; 2], // 288
    pub event_amount: u8, // кол-во событий в строении 289
    pub size_x: u8, // размер по широте 290
    pub size_y: u8, // размер по долготе 291
	pub _empty1: [u8; 1], // 292
    pub owner_army_id: u8, // владелец строения FF - нет владельца, 00 - по-умолчанию 293
	pub _empty2: [u8; 1], // 294
    pub barracks_visibility: u8, // работают ли казармы? если найм не указан то 00 если да то 01 295
    pub number_of_artifacts_for_sale: u8, // кол-во артефактов на продажу 296
	pub _empty3: [u8; 12], // 308
    pub spell_ids: [u8; 5], // айди заклинаний 314 bytes (info about spells available for study)
    pub garrison_units: [GarrisonUnit; 6], // гарнизон 332 bytes
    pub additional_garrison_defense: u8, // дополнительная защита гарнизона 333
    pub min_artifact_price: u16, // минимальная стоимость артефактов 335,
	pub max_artifact_price: u16, // максимальная стоимость артефактов 337,
	pub group: u8, // союзник, сосед, враг в таком духе 338
	pub relations: RelationsData, // отношения здания 342
	pub _empty4: [u8; 8], // 350
	pub mana_income: u8, // приток маны 351
	pub max_mana_income: u8, // максимальный приток маны 352
	pub _empty5: [u8; 1], // 353
	pub knight_start_building: u8, // стартовое здание рыцаря 354
	pub mage_start_building: u8, // стартовое здание архимага 355
	pub ranger_start_building: u8, // стартовое здание следопыта 356
	pub all_start_building: u8, // общее стартовое строение 357
	pub garrison_only_pc: u8, // гарнизон только для ии? 358
}
#[derive(FromBytes, Unaligned, PartialEq, Eq, Debug, Copy, Clone)]
#[repr(packed(1))]
pub struct HeroInfoData {
	pub _empty1: [u8; 6], // 6
	pub battle_xp: u16, // 8
	pub gold: u16, // 10
	pub _empty2: [u8; 2], // 12
	pub mana: u16, // 14
	pub _empty0: [u8; 2], // 16
	pub start_building: u8, // 17
	pub _empty3: [u8; 2], // 19
	pub army_data: [GarrisonUnit; 6], // 37
	pub x: u16, // 39
	pub y: u16, // 41
	pub items: [u8; 3], // 44
	pub spells: [u8; 5], // 49
	pub _empty4: [u8; 1] // 50
}
#[derive(FromBytes, Unaligned, Debug, Copy, Clone)]
#[repr(packed(1))]
pub struct FractionRelationsData {
	pub a: RelationsData,
	pub b: RelationsData,
	pub c: RelationsData,
	pub d: RelationsData
}
#[derive(FromBytes, Unaligned, Debug, Clone, Copy)]
#[repr(packed(1))]
pub struct SettingsData {
	pub size_x: u32, // 4
	pub size_y: u32, // 8
	pub seed: u32, // 12
	pub text_start: u32, // в байтах 16
	pub surface_size: u32, // 20
	pub deco_size: u32, // 24
	pub buildings_size: u32, // 28
	pub armies_size: u32, // 32
	pub lanterns_size: u32, // 36
	pub events_size: u32, // 40
	pub _empty: [u8; 4], // 44
	pub start_time: u32, // в минутах 48
	pub knight_data: HeroInfoData, // 98
	pub mage_data: HeroInfoData, // 148
	pub ranger_data: HeroInfoData, // 198 Если вы читаете это то знайте что я заебался искать тут недостающий байт и он оказался просто в конце этой несчастной структурки
	pub winning_event_id: u16, // 200
	pub _empty1: [u8; 4], // 204
	pub losing_event_id: u16, // 206
	pub _empty2: [u8; 4], // 210
	pub global_relations: FractionRelationsData, // 226
	pub named_units_amount: u8, // 227
	pub names_units_ids: [u8; 32], // 259
	pub scenario_variant: u8, // 00 - одиночный 01 - начальный компании 02 - обычный компании 260
	pub save_money: u8, // сохранение денег при переходе на эту карту 261
	pub save_mana: u8, // сохранение маны при переходе на эту карту 262
	pub save_fame: u8, // сохранить что каво? тоннельщина 263
	pub save_xp_and_lvl: u8, // сохранить опыт и уровень 264
	pub save_own_items: u8, // сохранить личные артефакты 265
	pub save_all_items: u8, // сохранить все артефакты 266
	pub save_all_troops: u8, // сохранить всю армию 267
	pub _empty3: [u8; 5], // 272
	pub scenario_pic_size: [u16; 2], // размер картинки сценария 276
	pub scenario_pic_id: u8, // номер картинки к сценарию 277
	pub _empty4: [u8; 3], // 280
	pub map_version: u16 // кол-во сохранений в редакторе 282
}
#[derive(Debug, Clone, Copy, FromBytes)]
pub struct LightOrEvent {
	pub x: u16,
	pub y: u16,
	pub id: u8,
	pub map_model: u8,
	pub empty: [u8; 33],
	pub light_radius: u8,
}
pub struct MapData {
    pub settings: SettingsData,
	pub buildings: Vec<BuildingData>,
    pub map: Vec<u8>,
    pub decos: Vec<(u16, u16, u16)>,
	pub armies: Vec<ArmyData>,
}
pub fn convert_map(path: &Path) -> Result<MapData, ()> {
    let mut buf: bytes::Bytes = {
        let mut file = File::open(&path).map_err(|_| ())?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf);
        Bytes::copy_from_slice(&buf)
    };
    let mut header_buf_start = buf.copy_to_bytes(8);
    buf.advance(4);
    let bzip_buf = buf.copy_to_bytes(4);

    let bzip2_header_start = Bytes::from_static(b"\x41\x49\x70\x66\x0D\x0A\x13\x00");
    let bzip2_header = Bytes::from_static(b"\x42\x5A\x68\x39");
    let mut compressed = true;
    // Check if header is bzip2 and get file uncompressed
    let mut data = if header_buf_start == bzip2_header_start && bzip_buf == bzip2_header {
        let mut compressed_buf = Vec::new();
        compressed_buf.write(&bzip2_header);
        compressed_buf.write(&buf);
        let mut uncompressed_buf = Vec::new();
        BzDecoder::new(compressed_buf.as_slice()).read_to_end(&mut uncompressed_buf);
        Bytes::from(uncompressed_buf)
    } else {
        println!(
            "{:x?}; {:x?}",
            header_buf_start, b"\x41\x49\x70\x66\x0D\x0A\x13\x00"
        );
        println!("{:x?}; {:x?}", bzip_buf, b"\x42\x5A\x68\x39");
        compressed = false;
        buf
    };
    let file_size = dbg!(data.remaining());
    let mut header_buf = data.copy_to_bytes(12);
    if compressed
        && header_buf != Bytes::from_static(b"\x4D\x61\x70\x4C\x44\x56\x20\x56\x2E\x34\x0D\x0A")
    {
        panic!("Wrong uncompressed header {:X?}", header_buf);
    }
	let bytes = &mut data.copy_to_bytes(282);
    let settings = SettingsData::read_from_bytes(&bytes.as_bytes()).unwrap();
    let current_offset = file_size - data.remaining();
    let data_offset = 0x12F - 0xC - 0x8 - 0x4 * 0x6;
    println!("0x{:X?};0x{:X?}", current_offset, data_offset);
    data.advance(0x12F - current_offset);
    let mut surface_data = data.copy_to_bytes(settings.surface_size as usize);
    let mut objects_data = data.copy_to_bytes(settings.deco_size as usize);
    let mut buildings_data = data.copy_to_bytes(settings.buildings_size as usize);
    let mut armies_data = data.copy_to_bytes(settings.armies_size as usize);
    let mut lanterns_data = data.copy_to_bytes(settings.lanterns_size as usize);
	// let mut texts_data = data.copy_to_bytes(settings.text_size as usize);
    dbg!(surface_data.remaining());
    fn parse_by_2_bytes(mut bytes: Bytes) -> Vec<u8> {
        let mut map = vec![];
        while !&bytes.is_empty() {
            let data = &mut bytes.copy_to_bytes(2);
            let (thing, amount) = (data.get_u8(), data.get_u8() as i16 + 1);
            map.append(&mut (0..amount).map(|_| thing).collect());
        }
        map
    }
    fn parse_decos(mut bytes: Bytes) -> Vec<(u16, u16, u16)> {
        let mut decos = vec![];
        while !&bytes.is_empty() {
            let data = &mut bytes.copy_to_bytes(6);
            let (x, y, deco) = (data.get_u16(), data.get_u16(), data.get_u16());
            decos.push((x, y, deco));
        }
        decos
    }
	fn parse_armies(mut bytes: Bytes) -> Vec<ArmyData> {
		let mut armies = vec![];
        while !&bytes.is_empty() {
            let data = &mut bytes.copy_to_bytes(89);
            armies.push(ArmyData::read_from_bytes(data.as_bytes()).unwrap());
        }
        armies
	}
	fn parse_buildings(mut bytes: Bytes) -> Vec<BuildingData> {
		let mut buildings = vec![];
        while !&bytes.is_empty() {
            let data = &mut bytes.copy_to_bytes(358);
            buildings.push(BuildingData::read_from_bytes(data.as_bytes()).unwrap());
        }
        buildings
	}
	fn parse_text(mut bytes: Bytes) -> Vec<String> {
		let mut text_buffer = vec![];
		for i in bytes.utf8_chunks() {
			println!("{}", i.valid());
		}
        text_buffer
	}
    let mut map = parse_by_2_bytes(surface_data);
    let decos = parse_decos(objects_data);
	let armies = parse_armies(armies_data);
	let buildings = parse_buildings(buildings_data);
	//let texts = parse_text(texts_data);
    Ok(MapData {
		settings,
		buildings,
		decos,
        map,
		armies
    })
}
mod test {
    use std::{
        fs::{self, File},
        io::Write,
        os,
        path::Path,
    };

    #[test]
    fn test() {
        for path in fs::read_dir("../dt/Maps_Rus/").unwrap() {
            let path = path.as_ref().unwrap();
            if !path.file_name().to_str().unwrap().ends_with("DTm") {
                continue;
            }
            let data = super::convert_map(&path.path().as_path()).unwrap();
            let map = data.map;
            let (map_height, map_width) = (data.settings.size_x as usize, data.settings.size_y as usize);
            let terr_ascii = [
                "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "A", "B", "C", "D", "E", "F",
            ];
        }
    }
}
