//! Запекание карты в RGBA-буфер (софтверный рендер, Фаза 1).
//!
//! Источник геометрии/спрайтов — wgpu_ui::bake (draw_tiles/draw_decos):
//! тайл 32x22, спрайт декора/строения масштабируется из натуральных
//! пикселей (w/32, h/22), армия — юнит-спрайты Phase 1+. Здесь тот же
//! порядок слоёв: tilemap → decos → buildings; армия/фонарики поверх
//! рисует egui-слой редактора (клики/выделение).
//!
//! Производительность: наивная выборка по одному тайлу; 50×50 ≈ 0.8 Мп,
//! 100×100 ≈ 3.2 Мп — запекается один раз на изменение (кэш в EditorUI),
//! не в кадре. Оптимизация не требуется до появления данных.

use std::collections::HashMap;

use image::RgbaImage;

use dt_lib::map::object::{ObjectInfo, ObjectType};

/// Размер тайла в мировых пикселях (как в wgpu_ui::state::SIZE).
pub const TILE: (u32, u32) = (32, 22);

/// Загруженные спрайты: ключ — путь из Objects.ini / имя файла тайла.
pub struct SpriteCache {
    sprites: HashMap<String, RgbaImage>,
}

impl SpriteCache {
    pub fn new() -> Self {
        Self {
            sprites: HashMap::new(),
        }
    }
    fn get(&mut self, path: &str) -> Option<&RgbaImage> {
        if !self.sprites.contains_key(path) {
            let img = image::open(path).ok()?.to_rgba8();
            self.sprites.insert(path.to_string(), img);
        }
        self.sprites.get(path)
    }
}

/// Прозрачность: альфа ниже порога — не рисуем (у спрайтов нет полупрозрачных
/// краёв, как в оригинальной игре).
const ALPHA_THRESHOLD: u8 = 128;

fn blit(dst: &mut RgbaImage, src: &RgbaImage, dx: i64, dy: i64, dw: u32, dh: u32) {
    let (sw, sh) = src.dimensions();
    for oy in 0..dh {
        for ox in 0..dw {
            // Выборка: масштаб спрайта в целевой размер.
            let sx = (ox as u64 * sw as u64 / dw.max(1) as u64) as u32;
            let sy = (oy as u64 * sh as u64 / dh.max(1) as u64) as u32;
            let px = src.get_pixel(sx.min(sw - 1), sy.min(sh - 1));
            if px.0[3] < ALPHA_THRESHOLD {
                continue;
            }
            let x = dx + ox as i64;
            let y = dy + oy as i64;
            if x >= 0 && y >= 0 && (x as u32) < dst.width() && (y as u32) < dst.height() {
                dst.put_pixel(x as u32, y as u32, *px);
            }
        }
    }
}

/// Слои карты: последовательность запечки управляется вызывающей стороной
/// (кэш инвалидируется точечно — см. EditorUI).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BakeOptions {
    pub width: u32,
    pub height: u32,
}

/// Запечённая карта: RGBA8, premultiply не используется (egui ждёт прямой альфу).
#[derive(Clone)]
pub struct BakedMap {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// Собрать спрайты тайлов: `dt/assets/Terrain/{sprite()}` для 16 TILES.
pub fn tile_sprites(terrain_dir: &str) -> Vec<RgbaImage> {
    dt_lib::map::tile::TILES
        .iter()
        .map(|tile| {
            let path = format!("{terrain_dir}/{}", tile.sprite());
            image::open(&path)
                .unwrap_or_else(|e| panic!("tile sprite {path}: {e}"))
                .to_rgba8()
        })
        .collect()
}

/// Запечь tilemap (слой 1). Координаты тайлов: dt_lib TileMap индекс —
/// inner[y + x*size] (см. Index for TileMap).
pub fn bake_tilemap(tilemap: &dt_lib::map::map::TileMap<usize>, sprites: &[RgbaImage]) -> BakedMap {
    let (tw, th) = TILE;
    let (w, h) = (tilemap.size as u32 * tw, tilemap.size as u32 * th);
    let mut img = RgbaImage::new(w, h);
    for index in 0..tilemap.inner.len() {
        let x = index / tilemap.size;
        let y = index % tilemap.size;
        let tile = sprites
            .get(tilemap.inner[index])
            .unwrap_or(&sprites[0]);
        blit(&mut img, tile, (x as u32 * tw) as i64, (y as u32 * th) as i64, tw, th);
    }
    BakedMap {
        rgba: img.into_raw(),
        width: w,
        height: h,
    }
}

/// Запечь слой декораций и строений (слой 2): декор по `index` ищется в
/// реестре объектов (ObjectType::MapDeco), строение — по `id` (index).
/// Порядок внутри слоя: по базовой Y (низ экрана — поверх), при равенстве —
/// более высокий спрайт поверх (семантика wgpu_ui::bake).
pub fn bake_objects(
    map: &dt_lib::map::map::GameMap,
    objects: &[ObjectInfo],
    sprite_cache: &mut SpriteCache,
    assets_prefix: &str,
) -> BakedMap {
    let (tw, th) = TILE;
    let (w, h) = (map.tilemap.size as u32 * tw, map.tilemap.size as u32 * th);
    let mut img = RgbaImage::new(w, h);

    // Список отрисовки: (base_y, x, y, path, draw_w, draw_h).
    let mut items: Vec<(f32, i64, i64, String, u32, u32)> = Vec::new();
    for deco in &map.decomap {
        let Some(obj) = objects.iter().find(|el| match el.obj_type {
            ObjectType::MapDeco { id } => id == deco.index,
            _ => false,
        }) else {
            continue;
        };
        let Some(sprite) = sprite_cache.get(&format!("{assets_prefix}/{}", obj.path)) else {
            continue;
        };
        let (sw, sh) = sprite.dimensions();
        items.push((
            deco.y as f32 * th as f32,
            (deco.x as u32 * tw) as i64 - sw as i64 + tw as i64,
            (deco.y as u32 * th) as i64 - sh as i64 + th as i64,
            obj.path.clone(),
            sw / 32 * tw,
            sh / 22 * th,
        ));
    }
    for b in &map.buildings {
        let Some(obj) = objects.iter().find(|el| el.index == b.id) else {
            continue;
        };
        let Some(sprite) = sprite_cache.get(&format!("{assets_prefix}/{}", obj.path)) else {
            continue;
        };
        let (sw, sh) = sprite.dimensions();
        items.push((
            b.pos.1 as f32 * th as f32,
            (b.pos.0 as u32 * tw) as i64 - sw as i64 + tw as i64,
            (b.pos.1 as u32 * th) as i64 - sh as i64 + th as i64,
            obj.path.clone(),
            sw / 32 * tw,
            sh / 22 * th,
        ));
    }
    items.sort_unstable_by(|a, b| {
        a.0.partial_cmp(&b.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                b.4.cmp(&a.4) // более широкий спрайт — «выше»
            })
    });
    for (_base, x, y, path, dw, dh) in items {
        if let Some(sprite) = sprite_cache.get(&format!("{assets_prefix}/{path}")) {
            blit(&mut img, sprite, x, y, dw, dh);
        }
    }
    BakedMap {
        rgba: img.into_raw(),
        width: w,
        height: h,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::MapProject;

    fn gradient_tile() -> RgbaImage {
        let mut img = RgbaImage::new(4, 4);
        for y in 0..4 {
            for x in 0..4 {
                img.put_pixel(x, y, image::Rgba([(x * 60) as u8, (y * 60) as u8, 0, 255]));
            }
        }
        img
    }

    #[test]
    fn bake_tilemap_covers_full_canvas() {
        let project = MapProject::new(3, 0);
        let sprites = vec![gradient_tile()];
        let baked = bake_tilemap(&project.map.tilemap, &sprites);
        assert_eq!(baked.width, 3 * TILE.0);
        assert_eq!(baked.height, 3 * TILE.1);
        assert_eq!(baked.rgba.len(), (baked.width * baked.height * 4) as usize);
        // Клетка (1,1) opaque: альфа 255.
        let px = (TILE.1 + TILE.1 / 2) as usize * baked.width as usize * 4
            + (TILE.0 + TILE.0 / 2) as usize * 4
            + 3;
        assert_eq!(baked.rgba[px], 255);
    }

    #[test]
    fn bake_tilemap_uses_tile_index() {
        let mut project = MapProject::new(2, 0);
        project.map.tilemap[(1, 0)] = 1; // (x=1, y=0) — второй спрайт
        let mut second = RgbaImage::new(2, 2);
        second.put_pixel(0, 0, image::Rgba([10, 20, 30, 255]));
        let sprites = vec![gradient_tile(), second];
        let baked = bake_tilemap(&project.map.tilemap, &sprites);
        // Центр тайла (1,0): x in [32..64), y in [0..22).
        let px = 10 * baked.width as usize * 4 + 40 * 4;
        assert_eq!([baked.rgba[px], baked.rgba[px + 1], baked.rgba[px + 2]], [10, 20, 30]);
    }

    #[test]
    fn blit_respects_alpha() {
        let mut dst = RgbaImage::new(2, 2);
        let mut src = RgbaImage::new(2, 2);
        src.put_pixel(0, 0, image::Rgba([1, 2, 3, 0])); // прозрачный
        src.put_pixel(1, 1, image::Rgba([9, 8, 7, 255]));
        blit(&mut dst, &src, 0, 0, 2, 2);
        assert_eq!(dst.get_pixel(0, 0).0[3], 0);
        assert_eq!(dst.get_pixel(1, 1).0, [9, 8, 7, 255]);
    }

    #[test]
    fn objects_layer_unknown_refs_are_skipped() {
        let mut project = MapProject::new(2, 0);
        project.map.decomap.push(dt_lib::map::deco::MapDeco::new(99, 0, 0));
        project.map.buildings.push(crate::project::default_building(123, (1, 1)));
        let baked = bake_objects(&project.map, &[], &mut SpriteCache::new(), "/nonexistent");
        // Ничего не нарисовано, но буфер корректного размера.
        assert_eq!(baked.width, 2 * TILE.0);
        assert!(baked.rgba.iter().all(|&b| b == 0));
    }
}
