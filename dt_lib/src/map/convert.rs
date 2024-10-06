use super::{deco::*, map::*};
use bufread::BzDecoder;
use bytes::*;
use bzip2::*;
use encoding_rs::*;
use encoding_rs_io::*;
use std::{
    fs,
    fs::File,
    io,
    io::{Read, Result as IoResult, Seek, Write},
    path::Path,
};
pub fn convert_map(path: &Path) -> Result<(), ()> {
    let mut buf: bytes::Bytes = {
        let mut file = File::open(&path).map_err(|v| ())?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf);
        Bytes::copy_from_slice(&buf)
    };
    dbg!(buf.remaining());
    let mut header_buf_start = buf.copy_to_bytes(8);
    buf.advance(4);
    let bzip_buf = buf.copy_to_bytes(4);

    let bzip2_header_start = Bytes::from_static(b"\x41\x49\x70\x66\x0D\x0A\x13\x00");
    let bzip2_header = Bytes::from_static(b"\x42\x5A\x68\x39");

    // Check if header is bzip2 and get file uncompressed
    let mut data = if header_buf_start == bzip2_header_start && bzip_buf == bzip2_header {
        println!("Decompressing");
        let mut compressed_buf = Vec::new();
        compressed_buf.write(&bzip2_header);
        compressed_buf.write(&buf);
        let mut uncompressed_buf = Vec::new();
        BzDecoder::new(compressed_buf.as_slice()).read_to_end(&mut uncompressed_buf);
        print!("{:X?}00000000{:X?}", &header_buf_start, &bzip_buf);
        Bytes::from(uncompressed_buf)
    } else {
        println!(
            "{:x?}; {:x?}",
            header_buf_start, b"\x41\x49\x70\x66\x0D\x0A\x13\x00"
        );
        println!("{:x?}; {:x?}", bzip_buf, b"\x42\x5A\x68\x39");
        buf
    };
    let file_size = dbg!(data.remaining());
    let mut header_buf = data.copy_to_bytes(12);
    if header_buf != Bytes::from_static(b"\x4D\x61\x70\x4C\x44\x56\x20\x56\x2E\x34\x0D\x0A") {
        panic!("Wrong uncompressed header {:X?}", header_buf);
    }
    let (map_width, map_height) = (data.get_i32_le() as usize, data.get_i32_le() as usize);
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
    let terr_ascii = [
        "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "A", "B", "C", "D", "E", "F",
    ];
    let mut map = parse_by_2_bytes(surface_data);
	let decos = parse_decos(objects_data);
	
    let name = format!(
        "../dt/Maps_Rus/{}_formatted.ini",
        path.file_name().unwrap().to_str().unwrap()
    );
    fs::remove_file(&name);
    let mut file = File::create_new(&name).unwrap();
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
		file.write_all(string.as_bytes());
    }
    Ok(())
}
mod test {
    use std::{fs, os, path::Path};

    #[test]
    fn test() {
        for path in fs::read_dir("../dt/Maps_Rus/").unwrap() {
			if !path.as_ref().clone().unwrap().file_name().to_str().unwrap().ends_with("DTm") { continue; }
            println!("{:?}", &path.as_ref().unwrap().file_name());
            super::convert_map(path.unwrap().path().as_path());
        }
    }
}
