use std::path::{Path, PathBuf};

use dt_lib::map::convert::{convert_map, ArmyTroopsData, PatrolData, ToBool};
use clap::*;
use zerocopy::FromZeros;
#[derive(Parser)]
struct Args {
	/// A DTm file to parse
	#[arg()]
	file: PathBuf,
	/// To show armies info
	#[arg(short, default_value_t = false)]
	armies: bool,
	/// To show map
	#[arg(short, default_value_t = false)]
	map: bool
}

fn main() {
	let args = Args::parse();
	let Ok(mut data) = convert_map(&args.file) else { return; };
    if args.map {
		let map = data.map;
		let (map_height, map_width) = (data.map_size.0 as usize, data.map_size.1 as usize);
		let terr_ascii = [
			"0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "A", "B", "C", "D", "E", "F",
		];
		for i in 0..map_height {
			let string = map
				.iter()
				.skip(map_width * i)
				.take(map_width)
				.filter_map(|x| terr_ascii.get(*x as usize))
				.cloned()
				.map(|string| string.to_string())
				.collect::<String>()
				+ "\n";
			print!("{}", string);
		}
	}
	println!("text_size 0x{:X?} ({})", data.text_size, data.text_size);
	println!("surface_size 0x{:X?} ({})", data.surface_size, data.surface_size);
	println!("objects_size 0x{:X?} ({})", data.objects_size, data.objects_size);
	println!("objects {}", data.objects_size / 6);
	println!("buildings_size 0x{:X?} ({})", data.buildings_size, data.buildings_size);
	println!("buildings {}", data.buildings_size / 358);
	println!("armies_size 0x{:X?} ({})", data.armies_size, data.armies_size);
	println!("armies {}", data.armies_size / 89);
	println!("lanterns_size 0x{:X?} ({})", data.lanterns_size, data.lanterns_size);
	println!("lanterns {}", data.lanterns_size / 99);
	println!("map_size {}/{}", data.map_size.0, data.map_size.1);
	loop {
		if !args.armies || data.armies.is_empty() {
			break;
		}
		let army = data.armies.remove(0);
		let (x, y) = (army.x, army.y);
		let id = army.id;
		let map_model = army.map_model;
		let tactic_cost = (army.tactic_cost, army.tactic_cost_part2);
		let speed_correction = army.speed_correction;
		let xp_like_player = army.xp_like_player.to_bool();
		let gold_income = army.gold_income;
		let xp_add = army.xp_add;
		let start_building_id = army.start_building_id;
		let troops = army.troops;
		let items_ids = army.items_ids;
		let named_unit_id = army.named_unit_id;
		let patrol = army.patrol;
		let units_without_money = army.units_without_money.to_bool();
		let activity = army.activity;
		let group_type = army.group_type;
		let relations = army.relations;
		let aggression = army.agression;
		let revive_time = army.revive_time;
		let xp_correction = army.xp_correction;
		let ship_type = army.ship_type;
		let ignores_ai_armys = army.ignores_ai_armys.to_bool();
		let goes_towards_player = army.goes_towards_player.to_bool();
		let forbid_random_targets = army.goes_towards_player.to_bool();
		let forbid_talks = army.forbid_talks.to_bool();
		let not_interested_in_buildings = army.not_interested_in_buildings.to_bool();
		let garrison_power_in_buildings = army.garrison_power_in_buildings;
		let revive_everyone = army.revive_everyone.to_bool();
		let applied_spell = army.applied_spell;
		let action_model = army.action_model;
		let (empty0, empty1, empty2, empty3, empty4, empty5, empty6, empty7) = (army._empty, army._empty0, army._empty1, army._empty2, army._empty3, army._empty4, army._empty5, army._empty6);
		println!("Pos {}/{}", x, y);
		println!("Id {}", id);
		if tactic_cost != (0, 0) {
			println!("Tactic cost {}/{}", tactic_cost.0, tactic_cost.1);
		}
		if speed_correction != 0 {
			println!("Speed correction: {}", speed_correction);
		}
		if xp_like_player {
			println!("XP like player: true");
		}
		if gold_income != 0 {
			println!("Gold income: {}", gold_income);
		}
		if xp_add != 0 {
			println!("XP add: {}", xp_add);
		}
		println!("Start building id: {}", start_building_id);
		if troops != ArmyTroopsData::new_zeroed() {
			dbg!(troops);
		}
		if items_ids != [0, 0, 0] {
			println!("Items: {:?}", items_ids);
		}
		if named_unit_id != 0 {
			println!("Named unit id: {}", named_unit_id);
		}
		if patrol != PatrolData::new_zeroed() {
			println!("Patrol radius: {} (exists? {})", patrol.radius, patrol.exists);
		}
		if units_without_money {
			println!("Units without money: true")
		}
		if activity != 0 {
			println!("Activity: {}", activity);
		}
		println!("Group type: {}", group_type);
		println!("Relations: {}|{}|{}|{}", relations.a, relations.b, relations.c, relations.d);
		if aggression != 0 {
			println!("Aggression: {}", aggression);
		}
		if revive_time != 0 {
			println!("Revive time: {}", revive_time);
		}
		if xp_correction != 10 {
			println!("XP correction: {}", xp_correction);
		}
		if ship_type != 0 {
			println!("Ship: {}", ship_type);
		}
		if ignores_ai_armys {
			println!("Ignores ai armys: true");
		}
		if goes_towards_player {
			println!("Goes towards player: true");
		}
		if forbid_random_targets {
			println!("Forbid random targets: true");
		}
		if forbid_talks {
			println!("Forbid talks: true");
		}
		if not_interested_in_buildings {
			println!("Not interested in buildings: true");
		}
		if garrison_power_in_buildings != 0 {
			println!("Garrison power in buildings: {}", garrison_power_in_buildings);
		}
		if revive_everyone {
			println!("Revive everyone: true");
		}
		if applied_spell != 0 {
			println!("Applied spell: {}", applied_spell);
		}
		println!("Action model: {}", action_model);
		if empty0 != [0, 0, 0, 0, 0] {
			println!("Empty0 is: {:?}", empty0);
		}
		if empty1 != [0, 0] {
			println!("Empty1 is: {:?}", empty1);
		}
		if empty2 != [0, 0, 0, 0] {
			println!("Empty2 is: {:?}", empty2);
		}
		if empty3 != [0, 0, 0, 0] {
			println!("Empty3 is: {:?}", empty3);
		}
		if empty4 != [0, 0, 0, 0, 0] {
			println!("Empty4 is: {:?}", empty4);
		}
		if empty5 != 0 {
			println!("Empty5 is: {:?}", empty5);
		}
		if empty6 != 0 {
			println!("Empty6 is: {:?}", empty6);
		}
	}
}
