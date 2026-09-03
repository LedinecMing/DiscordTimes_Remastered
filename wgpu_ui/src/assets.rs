// Порт Assets/load_assets из quad_ui main.rs (62-138) на TexId + fontdue.
use crate::files::DesktopFiles;
use crate::gfx::{Filter, Gfx, TexId};
use crate::text::{FontId, TextRenderer};
use ahash::RandomState;
use dt_lib::parse::{parse_items, parse_objects, parse_settings, parse_units, FileAccess};
use std::collections::HashMap;

pub const BENGUIAT: &str = "benguiat";
pub const GOTHIC: &str = "gothic";
pub const Z003: &str = "z003";

#[derive(Debug, Clone, Default)]
pub struct Assets {
    pub inner: HashMap<String, TexId, RandomState>,
    pub fonts: HashMap<&'static str, FontId>,
}

impl Assets {
    pub fn get(&self, key: &str) -> TexId {
        *self
            .inner
            .get(key)
            .unwrap_or_else(|| panic!("No asset {key}"))
    }
    pub fn get_font(&self, key: &str) -> FontId {
        *self.fonts.get(key).unwrap_or_else(|| panic!("No font {key}"))
    }
}

fn read_file(path: &str) -> Vec<u8> {
    std::fs::read(path).unwrap_or_else(|e| panic!("file read failed: {path}: {e}"))
}

/// Порт load_assets (quad_ui main.rs:109): req_assets_list = (префикс, [имена]);
/// ключ в Assets — имя без префикса (как в macroquad-версии). Отсутствующий файл
/// паникует (та же семантика, что load_texture(...).unwrap + collect_errors).
pub async fn load_assets(
    gfx: &mut Gfx,
    _text: &mut TextRenderer,
    req_assets_list: &[(&str, Vec<String>)],
    fonts: HashMap<&'static str, FontId>,
) -> Assets {
    let base = gfx.textures_count();
    let mut asset_names = Vec::new();
    for req_assets in req_assets_list {
        for asset in &req_assets.1 {
            let path = format!("{}/{}", req_assets.0, asset);
            let bytes = read_file(&path);
            asset_names.push(asset.clone());
            gfx.push_texture_bytes(&bytes, Filter::Linear);
        }
    }
    let inner = asset_names
        .into_iter()
        .zip((base..gfx.textures_count()).map(TexId))
        .collect::<HashMap<String, TexId, RandomState>>();
    Assets { inner, fonts }
}

/// Загрузка трёх шрифтов (порт load_ttf_font вызовов, quad_ui main.rs:190-200).
pub fn load_fonts(text: &mut TextRenderer) -> HashMap<&'static str, FontId> {
    let mut fonts = HashMap::new();
    let benguiat = text.add_font(&read_file("Benguiat Rus Regular.ttf"));
    let z003 = text.add_font(&read_file("Z003-MediumItalic.ttf"));
    let gothic = text.add_font(&read_file("Ru_Gothic.ttf"));
    fonts.insert(BENGUIAT, benguiat);
    fonts.insert(Z003, z003);
    fonts.insert(GOTHIC, gothic);
    fonts
}

/// Порт parse_settings::<QuadFiles>() — Settings из dt_lib::registry.
pub async fn parse_settings_files() -> dt_lib::registry::Settings {
    parse_settings::<DesktopFiles>().await
}
