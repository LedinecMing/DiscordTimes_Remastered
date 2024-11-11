use std::path::{Path, PathBuf};

use dt_lib::map::convert::{convert_map, ArmyTroopsData, BuildingData, GarrisonUnit, HeroInfoData, PatrolData, ToBool};
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
	/// To show buildings info
	#[arg(short, default_value_t = false)]
	buildings: bool,
	/// To show map
	#[arg(short, default_value_t = false)]
	map: bool
}
fn if_not_zero<T: FromZeros + PartialEq>(obj: T, f: impl Fn(T)) {
	if obj != T::new_zeroed() {
		f(obj);
	}
}
fn main() {
	let args = Args::parse();
	let Ok(mut data) = convert_map(&args.file) else { return; };
    if args.map {
		let map = data.map;
		let (map_height, map_width) = (data.settings.size_x as usize, data.settings.size_y as usize);
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
	let settings = data.settings;
	println!("text_start 0x{:X?} ({})", settings.text_start + 0, settings.text_start + 0);
	println!("surface_size 0x{:X?} ({})", data.settings.surface_size + 0, data.settings.surface_size + 0);
	println!("deco_size 0x{:X?} ({})", data.settings.deco_size + 0, data.settings.deco_size + 0);
	println!("decorations {}", data.settings.deco_size / 6);
	println!("buildings_size 0x{:X?} ({})", data.settings.buildings_size + 0, data.settings.buildings_size + 0);
	println!("buildings {}", data.settings.buildings_size / 358);
	println!("armies_size 0x{:X?} ({})", data.settings.armies_size + 0, data.settings.armies_size + 0);
	println!("armies {}", data.settings.armies_size / 89);
	println!("lanterns_size 0x{:X?} ({})", data.settings.lanterns_size + 0, data.settings.lanterns_size + 0);
	println!("lanterns {}", data.settings.lanterns_size / 99);
	println!("map_size {}/{}", data.settings.size_x + 0, data.settings.size_y + 0);

	fn print_hero_info(hero: HeroInfoData) {
		println!("Pos: {}/{}", {hero.x}, {hero.y});
		if_not_zero(hero.gold, |gold| println!("Gold: {}", gold));
		if_not_zero(hero.mana, |gold| println!("Mana: {}", gold));
		if_not_zero(hero.battle_xp, |gold| println!("Battle XP: {}", gold));
		println!("Start building: {}", hero.start_building);
		for unit in hero.army_data {
			if_not_zero(unit, |unit| println!("Unit: {}; Level: {}; Count: {}", unit.id, unit.level, unit.count));
		}
		if_not_zero(hero.items, |items| println!("Items: {:?}", items));
		if_not_zero(hero.spells, |spells| println!("Spells: {:?}", spells));
	}
	fn print_building_info(building: BuildingData) {
		println!("Pos: {}/{}", {building.x}, {building.y});
		if building.picture_variant != 0 {
			println!("Pic variant {} number {}", {building.picture_variant}, {building.picture_number});
		}
		println!("Variant: {}", {building.variant});
		for event in building.event_ids {
			if event != 0 {
				println!("Event: {}", event);
			}
		}
		if_not_zero(building.number_of_artifacts_for_sale, |num| println!("Number of artifacts for sale: {num}"));
		if_not_zero(building.min_artifact_price, |num| println!("Minimum item price: {num}"));
		if_not_zero(building.max_artifact_price, |num| println!("Maximum item price: {num}"));
		for item in building.artifact_ids {
			if item != 0 {
				println!("Item: {}", item);
			}
		}
		if_not_zero(building._empty_big, |val| println!("Empty big is: {:?}", val));
		if building.barracks_visibility.to_bool() {
			println!("Has barracks: true");
		}
		for recruit in building.recruits {
			if_not_zero(recruit, |recruit| println!("Recruit id {} amount {} max {}", recruit.id, recruit.amount, recruit.max_amount));
		}
		if_not_zero(building.gold_income, |income| println!("Gold income: {income}"));
		if_not_zero(building.max_gold_income, |income| println!("Max gold income(?): {income}"));
		if_not_zero(building.mana_income, |income| println!("Mana income: {income}"));
		if_not_zero(building.max_mana_income, |income| println!("Max mana income(?): {income}"));
		if_not_zero(building.event_amount, |events| println!("Events amount: {events}"));
		println!("Size: {}/{}", {building.size_x}, {building.size_y});
		if_not_zero(building.owner_army_id, |owner| {
			if owner == 0xFF { println!("No owner") }
			else { println!("Owner: {owner}") }
		});
		println!("Group: {}", {building.group});
		println!("Relations: {:?}", {building.relations});
		if_not_zero(building.knight_start_building, |start| println!("Knight start building: {start}"));
		if_not_zero(building.mage_start_building, |start| println!("Mage start building: {start}"));
		if_not_zero(building.ranger_start_building, |start| println!("Ranger start building: {start}"));
		if_not_zero(building.all_start_building, |start| println!("All start building: {start}"));
		if building.garrison_only_pc.to_bool() {
			println!("Garrison is only for PC");
		}
	}
	if_not_zero(settings.knight_data, |data| {
		println!("> Knight data:");
		print_hero_info(data);
	});
	if_not_zero(settings.mage_data, |data| {
		println!("> Mage data:");
		print_hero_info(data);
	});
	if_not_zero(settings.ranger_data, |data| {
		println!("> Ranger data:");
		print_hero_info(data);
	});
	loop {
		if !args.buildings || data.buildings.is_empty() {
			break;
		}
		print_building_info(data.buildings.remove(0));
	}
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
		if_not_zero([tactic_cost.0, tactic_cost.1], |tactic_cost| println!("Tactic cost {}/{}", tactic_cost[0], tactic_cost[1]));
		if_not_zero(speed_correction, |speed_correction| println!("Speed correction: {}", speed_correction));
		if xp_like_player {
			println!("XP like player: true");
		};
		if_not_zero(gold_income, |gold_income| println!("Gold income: {}", gold_income));
		if_not_zero(xp_add, |xp_add| println!("XP add: {}", xp_add));
		println!("Start building id: {}", start_building_id);
		if_not_zero(troops, |troops| {dbg!(troops);});
		if_not_zero(items_ids, |items_ids| println!("Items: {:?}", items_ids));
		if_not_zero(named_unit_id, |named_unit_id| println!("Named unit id: {}", named_unit_id));
		if_not_zero(patrol, |patrol| println!("Patrol radius: {} (exists? {})", patrol.radius, patrol.exists));
		if units_without_money {
			println!("Units without money: true")
		}
		if_not_zero(activity, |activity| println!("Activity: {}", activity));
		
		println!("Group type: {}", group_type);
		println!("Relations: {}|{}|{}|{}", relations.a, relations.b, relations.c, relations.d);
		if_not_zero(aggression, |aggression| println!("Aggression: {}", aggression));
		if_not_zero(revive_time, |revive_time| println!("Revive time: {}", revive_time));
		if_not_zero(xp_correction, |xp_correction| println!("XP correction: {}", xp_correction));
		if_not_zero(ship_type, |ship_type| println!("Ship: {}", ship_type));
		if_not_zero(garrison_power_in_buildings, |garrison_power| println!("Garrison power in buildings: {}", garrison_power));
		if_not_zero(applied_spell, |applied_spell| println!("Applied spell: {}", applied_spell));
		if revive_everyone {
			println!("Revive everyone: true");
		}
		if ignores_ai_armys {
			println!("Ignores AI armies: true");
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
		println!("Action model: {}", action_model);

		if_not_zero(empty0, |empty0| println!("Empty0 is: {:?}", empty0));
		if_not_zero(empty1, |empty1| println!("Empty1 is: {:?}", empty1));
		if_not_zero(empty2, |empty2| println!("Empty2 is: {:?}", empty2));
		if_not_zero(empty3, |empty3| println!("Empty3 is: {:?}", empty3));
		if_not_zero(empty4, |empty4| println!("Empty4 is: {:?}", empty4));
		if_not_zero(empty5, |empty5| println!("Empty5 is: {:?}", empty5));
		if_not_zero(empty6, |empty6| println!("Empty6/ is: {:?}", empty6));
		if_not_zero(empty7, |empty7| println!("Empty7/ is: {:?}", empty7));
	}
}
