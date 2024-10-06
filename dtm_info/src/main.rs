use std::path::{Path, PathBuf};

use dt_lib::map::convert::convert_map;
use clap::*;
#[derive(Parser)]
struct Args {
	#[arg()]
	file: PathBuf,
	#[arg(short, default_value_t = false)]
	map: bool
}
fn main() {
	let args = Args::parse();
	let Ok(data) = convert_map(&args.file) else { return; };
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
}
