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
    pub text_size: i32,
    pub surface_size: i32,
    pub objects_size: i32,
    pub buildings_size: i32,
    pub armies_size: i32,
    pub lanterns_size: i32,
    pub map: Vec<u8>,
    pub map_size: (i32, i32),
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
    let (map_width, map_height) = (data.get_i32_le(), data.get_i32_le());
    data.advance(4);
    let (text_size, surface_size, objects_size, buildings_size, armies_size, lanterns_size) = (
        data.get_i32_le(),
        data.get_i32_le(),
        data.get_i32_le(),
        data.get_i32_le(),
        data.get_i32_le(),
        data.get_i32_le(),
    );
    let current_offset = file_size - data.remaining();
    let data_offset = 0x12F - 0xC - 0x8 - 0x4 * 0x6;
    println!("0x{:X?};0x{:X?}", current_offset, data_offset);
    data.advance(0x12F - current_offset);
    let mut surface_data = data.copy_to_bytes(surface_size as usize);
    let mut objects_data = data.copy_to_bytes(objects_size as usize);
    let mut buildings_data = data.copy_to_bytes(buildings_size as usize);
    let mut armies_data = data.copy_to_bytes(armies_size as usize);
    let mut lanterns_data = data.copy_to_bytes(lanterns_size as usize);
	// let mut texts_data = data.copy_to_bytes(text_size as usize);
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
	//let texts = parse_text(texts_data);
    Ok(MapData {
        surface_size,
        decos,
        armies_size,
        text_size,
        objects_size,
        lanterns_size,
        buildings_size,
        map,
        map_size: (map_height, map_width),
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
            let (map_height, map_width) = (data.map_size.0 as usize, data.map_size.1 as usize);
            let terr_ascii = [
                "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "A", "B", "C", "D", "E", "F",
            ];
        }
    }
}
