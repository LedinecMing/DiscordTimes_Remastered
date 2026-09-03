// Запечка карты в offscreen RT: тайлы + CPU-генерация наплывов (блендинг) +
// слой декораций. Порт quad_ui main.rs:1164-1801.
use crate::camera::Camera;
use crate::gfx::{colors, Filter, Gfx, Rt, Target, TexId, WHITE};
use crate::state::{BlendMode, Game, SIZE};
use ahash::RandomState;
use dt_lib::map::deco::MapDeco;
use dt_lib::map::map::*;
use dt_lib::map::object::{MapBuildingdata, ObjectType};
use dt_lib::map::tile::TILES;
use dt_lib::registry::{GameInfo, Objects};
use std::collections::HashMap;

pub fn rand_unit(seed: u64, key: u64) -> f32 {
    let mut x = seed ^ key.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^= x >> 31;
    (x >> 40) as f32 * (1.0 / (1u64 << 24) as f32)
}
const DECO_JITTER: f32 = 0.2;

fn draw_decos(
    gfx: &mut Gfx,
    assets: &crate::assets::Assets,
    decomap: &Vec<MapDeco>,
    objects: &Objects,
    seed: u64,
) {
    for deco in decomap {
        let (i, j) = (deco.x, deco.y);
        let key = i as u64 * 100_003 + j as u64;
        let dx = (rand_unit(seed, key * 2 + 1) * 2. - 1.) * SIZE.0 * DECO_JITTER;
        let dy = (rand_unit(seed, key * 2 + 2) * 2. - 1.) * SIZE.1 * DECO_JITTER;
        if let Some(obj) = objects.inner.iter().find(|el| match el.obj_type {
            ObjectType::MapDeco { id } => id == deco.index,
            _ => false,
        }) {
            let texture = assets.get(&obj.path.clone());
            let size = gfx.tex_size(texture);
            gfx.draw_texture(
                texture,
                i as f32 * SIZE.0 - size.0 + SIZE.0 + dx,
                j as f32 * SIZE.1 - size.1 + SIZE.1 + dy,
                size.0 / 32. * SIZE.0,
                size.1 / 22. * SIZE.1,
                WHITE,
            );
        }
    }
}

fn draw_tiles(gfx: &mut Gfx, assets: &crate::assets::Assets, tilemap: &TileMap<usize>) {
    let tile_textures = TILES
        .iter()
        .map(|tile| assets.get(tile.sprite()))
        .collect::<Vec<_>>();
    for i in 0..tilemap.size {
        for j in 0..tilemap.size {
            let tile_index = tilemap[(j, i)];
            gfx.draw_texture(
                tile_textures[tile_index],
                i as f32 * SIZE.0,
                j as f32 * SIZE.1,
                SIZE.0,
                SIZE.1,
                WHITE,
            );
        }
    }
}

// «Наплывы» вместо «автотайлинга клеток»: базовые тайлы рисуются как раньше (без
// пересэмплирования, чётко), а поверх — только зоны смешения у границ клеток.
// Наплыв ОДНОСТОРОННИЙ по приоритету тайла: тайл с большим приоритетом «наплывает»
// на соседа с меньшим (вода на всё, дорога на землю). Одинаковые спрайты не смешиваются.
const TILE_PRIORITY: [u8; 16] = [
    3, 3, 3, 1, 2, 1, 1, 1, 2, 2, 1, 1, 1, 1, 1, 1,
]; // вода 3 > дорога/болото 2 > земля 1

const OVERLAY_BAND: f32 = 0.2; // 20% длины/высоты тайла
const OVERLAY_A: (f32, f32) = (210., 255.);

// Плавное затухание: 1 у шва, 0 на расстоянии radius (косинус — мягкий градиент).
fn smooth_falloff(d: f32, radius: f32) -> f32 {
    let d = (d / radius).clamp(0., 1.);
    0.5 * (1. + (std::f32::consts::PI * d).cos())
}

// Узор соседа, ужать 256×242 → 32×22. Общий для всех клеток. Вход — пиксельный
// снапшот тайла (RgbaImage, замена macroquad Image из tile_pixels).
fn neighbor_pattern(neighbor: &image::RgbaImage) -> [u8; 32 * 22 * 3] {
    let mut out = [0u8; 32 * 22 * 3];
    let raw = neighbor.as_raw();
    let sw = neighbor.width() as usize;
    let sh = neighbor.height() as usize;
    let (mut rs, mut gs, mut bs, mut cnt);
    for py in 0..22 {
        for px in 0..32 {
            rs = 0;
            gs = 0;
            bs = 0;
            cnt = 0;
            for y in (py * sh / 22)..((py + 1) * sh / 22) {
                for x in (px * sw / 32)..((px + 1) * sw / 32) {
                    let i = (y * sw + x) * 4;
                    rs += raw[i] as u32;
                    gs += raw[i + 1] as u32;
                    bs += raw[i + 2] as u32;
                    cnt += 1;
                }
            }
            let o = (py * 32 + px) * 3;
            out[o] = (rs / cnt) as u8;
            out[o + 1] = (gs / cnt) as u8;
            out[o + 2] = (bs / cnt) as u8;
        }
    }
    out
}

// Оверлей-полоса. Цвет — срез узора соседа, прилегающий к шву; альфа cos-градиентом
// гаснет от шва до внутренней границы полосы. side: 0=верх, 1=низ, 2=лево, 3=право.
fn make_side_overlay(pattern: &[u8], side: usize, bw: f32, bh: f32, a: f32) -> Vec<u8> {
    let (w, h) = (SIZE.0 as usize, SIZE.1 as usize);
    let mut bytes = vec![0u8; w * h * 4];
    for py in 0..h {
        let vf = py as f32 + 0.5;
        for px in 0..w {
            let uf = px as f32 + 0.5;
            let (sx, sy, wgt) = match side {
                0 => {
                    if vf > bh {
                        continue;
                    }
                    (px, (h as f32 - bh + vf) as usize, smooth_falloff(vf, bh))
                }
                1 => {
                    if h as f32 - vf > bh {
                        continue;
                    }
                    (px, (vf - (h as f32 - bh)) as usize, smooth_falloff(h as f32 - vf, bh))
                }
                2 => {
                    if uf > bw {
                        continue;
                    }
                    ((w as f32 - bw + uf) as usize, py, smooth_falloff(uf, bw))
                }
                _ => {
                    if w as f32 - uf > bw {
                        continue;
                    }
                    ((uf - (w as f32 - bw)) as usize, py, smooth_falloff(w as f32 - uf, bw))
                }
            };
            let o = (sy * w + sx) * 3;
            let o4 = (py * w + px) * 4;
            bytes[o4..o4 + 3].copy_from_slice(&pattern[o..o + 3]);
            bytes[o4 + 3] = (a * wgt).round().clamp(0., 255.) as u8;
        }
    }
    bytes
}

// Угловой оверлей для Rounded-режимов: цвет = угол мини-узора диагонального соседа,
// альфа гаснет по обеим осям (произведение cos-фейдов).
// corner: 0=верх-право, 1=низ-право, 2=низ-лево, 3=верх-лево.
fn make_quad_overlay(pattern: &[u8], corner: usize, bw: f32, bh: f32, a: f32) -> Vec<u8> {
    let (w, h) = (SIZE.0 as usize, SIZE.1 as usize);
    let mut bytes = vec![0u8; w * h * 4];
    for py in 0..h {
        let vf = py as f32 + 0.5;
        for px in 0..w {
            let uf = px as f32 + 0.5;
            let (dx, dy) = match corner {
                0 => (w as f32 - uf, vf),
                1 => (w as f32 - uf, h as f32 - vf),
                2 => (uf, h as f32 - vf),
                _ => (uf, vf),
            };
            let wgt = smooth_falloff(dx, bw) * smooth_falloff(dy, bh);
            if wgt <= 0.001 {
                continue;
            }
            let sx = match corner {
                0 | 1 => (w as f32 - uf).clamp(0., w as f32 - 1.),
                _ => (w as f32 - bw + uf).clamp(0., w as f32 - 1.),
            } as usize;
            let sy = match corner {
                0 | 3 => (h as f32 - bh + vf).clamp(0., h as f32 - 1.),
                _ => (h as f32 - vf).clamp(0., h as f32 - 1.),
            } as usize;
            let o = (sy * w + sx) * 3;
            let o4 = (py * w + px) * 4;
            bytes[o4..o4 + 3].copy_from_slice(&pattern[o..o + 3]);
            bytes[o4 + 3] = (a * wgt).round().clamp(0., 255.) as u8;
        }
    }
    bytes
}

// Рисует наплывы поверх уже нарисованных тайлов. Возвращает число уникальных текстур.
pub fn draw_blend_overlays(
    gfx: &mut Gfx,
    tilemap: &TileMap<usize>,
    tile_pixels: &[image::RgbaImage],
    mode: BlendMode,
) -> usize {
    let size = tilemap.size;
    let strong = mode.strong();
    let a = if strong { OVERLAY_A.1 } else { OVERLAY_A.0 };
    let rounded = mode.rounded();
    let (band_w, band_h) = (SIZE.0 * OVERLAY_BAND, SIZE.1 * OVERLAY_BAND);
    let mut textures: Vec<TexId> = Vec::new();
    let mut edge_cache: HashMap<(usize, usize, usize), TexId, RandomState> =
        HashMap::with_hasher(RandomState::new());
    let mut quad_cache: HashMap<(usize, usize, usize), TexId, RandomState> =
        HashMap::with_hasher(RandomState::new());
    let mut pattern_cache: HashMap<usize, [u8; 32 * 22 * 3], RandomState> =
        HashMap::with_hasher(RandomState::new());
    let mut draws: Vec<(TexId, f32, f32)> = Vec::new();
    for i in 0..size {
        for j in 0..size {
            let id = tilemap[(j, i)];
            let same = |n: usize| n == id || TILES[n].sprite() == TILES[id].sprite();
            let pri = TILE_PRIORITY[id];
            let (cx, cy) = (i as f32 * SIZE.0, j as f32 * SIZE.1);
            // 4 стороны: вода/приоритетная сторона наплывает на соседа.
            let mut side = |di: isize, dj: isize, s: usize| {
                let (ni, nj) = (i as isize + di, j as isize + dj);
                if ni < 0 || nj < 0 || ni >= size as isize || nj >= size as isize {
                    return;
                }
                let n = tilemap[(nj as usize, ni as usize)];
                if same(n) || pri >= TILE_PRIORITY[n] {
                    return;
                }
                let key = (id, n, s);
                let tex = if let Some(&t) = edge_cache.get(&key) {
                    t
                } else {
                    let pat = *pattern_cache
                        .entry(n)
                        .or_insert_with(|| neighbor_pattern(&tile_pixels[n]));
                    let buf = make_side_overlay(&pat, s, band_w, band_h, a);
                    let t = gfx.push_texture_rgba(&buf, SIZE.0 as u32, SIZE.1 as u32, Filter::Nearest);
                    edge_cache.insert(key, t);
                    textures.push(t);
                    t
                };
                draws.push((tex, cx, cy));
            };
            side(0, -1, 0); // сверху
            side(0, 1, 1); // снизу
            side(-1, 0, 2); // слева
            side(1, 0, 3); // справа
            if rounded {
                let mut corner = |di: isize, dj: isize, c: usize| {
                    let (ni, nj) = (i as isize + di, j as isize + dj);
                    if ni < 0 || nj < 0 || ni >= size as isize || nj >= size as isize {
                        return;
                    }
                    let n = tilemap[(nj as usize, ni as usize)];
                    if same(n) || pri >= TILE_PRIORITY[n] {
                        return;
                    }
                    let key = (id, n, c);
                    let tex = if let Some(&t) = quad_cache.get(&key) {
                        t
                    } else {
                        let pat = *pattern_cache
                            .entry(n)
                            .or_insert_with(|| neighbor_pattern(&tile_pixels[n]));
                        let buf = make_quad_overlay(&pat, c, band_w, band_h, a);
                        let t =
                            gfx.push_texture_rgba(&buf, SIZE.0 as u32, SIZE.1 as u32, Filter::Nearest);
                        quad_cache.insert(key, t);
                        textures.push(t);
                        t
                    };
                    draws.push((tex, cx, cy));
                };
                corner(-1, -1, 3); // верх-лево
                corner(1, -1, 0); // верх-право
                corner(-1, 1, 2); // низ-лево
                corner(1, 1, 1); // низ-право
            }
        }
    }
    // Порядок по текстурам — как в macroquad-версии (батчинг подряд идущих).
    draws.sort_unstable_by_key(|d| d.0);
    for (tex, x, y) in draws {
        gfx.draw_texture(tex, x, y, SIZE.0, SIZE.1, WHITE);
    }
    textures.len()
}

// Запечка тайлов+наплывов в RT (порт bake_map_textures).
pub fn bake_map_textures(
    gfx: &mut Gfx,
    rt: &Rt,
    assets: &crate::assets::Assets,
    game: &Game,
    tile_pixels: &[image::RgbaImage],
    blend_mode: BlendMode,
) {
    let size = game.executor.gamemap.tilemap.size as f32;
    let camera = Camera::from_display_rect(0., SIZE.1 * size, SIZE.0 * size, -SIZE.1 * size);
    gfx.begin_pass(
        Target::Rt(rt.index),
        Some([0., 0., 0., 0.]),
        &camera,
    );
    let t_bake = std::time::Instant::now();
    draw_tiles(gfx, assets, &game.executor.gamemap.tilemap);
    let mut n_overlays = 0;
    if blend_mode != BlendMode::Off {
        n_overlays = draw_blend_overlays(gfx, &game.executor.gamemap.tilemap, tile_pixels, blend_mode);
    }
    gfx.end_pass();
    println!(
        "baked map {}x{}: tiles {}, overlays {}, mode {:?}, {:.1}ms",
        size,
        size,
        size * size,
        n_overlays,
        blend_mode,
        t_bake.elapsed().as_secs_f64() * 1000.
    );
}

// Слой декораций поверх карты (порт render_decos_layer): холмы всегда первыми,
// остальное сортируется по Y-базе.
#[allow(clippy::too_many_arguments)]
pub fn render_decos_layer(
    gfx: &mut Gfx,
    rt: &Rt,
    assets: &crate::assets::Assets,
    game: &Game,
    registry: &GameInfo,
    seed: u64,
    buildings_render: bool,
) {
    let size = game.executor.gamemap.tilemap.size as f32;
    let camera = Camera::from_display_rect(0., SIZE.1 * size, SIZE.0 * size, -SIZE.1 * size);
    gfx.begin_pass(Target::Rt(rt.index), Some([0., 0., 0., 0.]), &camera);

    let find_decos = |name: &'static str| -> Vec<MapDeco> {
        game.executor
            .gamemap
            .decomap
            .iter()
            .filter(|deco| {
                registry
                    .objects
                    .inner
                    .iter()
                    .find(|el| match el.obj_type {
                        ObjectType::MapDeco { id } => id == deco.index,
                        _ => false,
                    })
                    .is_some_and(|obj| obj.name.contains(name))
            })
            .cloned()
            .collect::<Vec<_>>()
    };
    let hills = find_decos("Hills");
    let mountains = find_decos("Mountain");
    let trees = find_decos("Tree");
    let rocks = find_decos("Rocks");
    let rest = game
        .executor
        .gamemap
        .decomap
        .iter()
        .filter(|deco| {
            [&hills, &mountains, &trees, &rocks]
                .iter()
                .all(|g| !g.contains(deco))
        })
        .cloned()
        .collect::<Vec<_>>();

    // Холмы под всем остальным.
    draw_decos(gfx, assets, &hills, &registry.objects, seed);

    let mut tex_cache: Vec<TexId> = Vec::new();
    let mut lookups: HashMap<String, usize, RandomState> = HashMap::with_hasher(RandomState::new());
    // (base_y, x, y, texture_idx, dest_w, dest_h)
    let mut items: Vec<(f32, f32, f32, usize, f32, f32)> = Vec::new();
    for g in [&mountains, &trees, &rocks, &rest] {
        for deco in g {
            let (i, j) = (deco.x, deco.y);
            if let Some(obj) = registry.objects.inner.iter().find(|el| match el.obj_type {
                ObjectType::MapDeco { id } => id == deco.index,
                _ => false,
            }) {
                let tex = match lookups.get(&obj.path) {
                    Some(&t) => t,
                    None => {
                        let t = tex_cache.len();
                        tex_cache.push(assets.get(&obj.path.clone()));
                        lookups.insert(obj.path.clone(), t);
                        t
                    }
                };
                let size = gfx.tex_size(tex_cache[tex]);
                let key = i as u64 * 100_003 + j as u64;
                let dx = (rand_unit(seed, key * 2 + 1) * 2. - 1.) * SIZE.0 * DECO_JITTER;
                let dy = (rand_unit(seed, key * 2 + 2) * 2. - 1.) * SIZE.1 * DECO_JITTER;
                items.push((
                    j as f32 * SIZE.1,
                    i as f32 * SIZE.0 - size.0 + SIZE.0 + dx,
                    j as f32 * SIZE.1 - size.1 + SIZE.1 + dy,
                    tex,
                    size.0 / 32. * SIZE.0,
                    size.1 / 22. * SIZE.1,
                ));
            }
        }
    }
    if buildings_render {
        for b in &game.executor.gamemap.buildings {
            let (i, j) = b.pos;
            if let Some(obj) = registry.objects.inner.iter().find(|el| el.index == b.id) {
                let tex = match lookups.get(&obj.path) {
                    Some(&t) => t,
                    None => {
                        let t = tex_cache.len();
                        tex_cache.push(assets.get(&obj.path.clone()));
                        lookups.insert(obj.path.clone(), t);
                        t
                    }
                };
                let size = gfx.tex_size(tex_cache[tex]);
                items.push((
                    j as f32 * SIZE.1,
                    i as f32 * SIZE.0 - size.0 + SIZE.0,
                    j as f32 * SIZE.1 - size.1 + SIZE.1,
                    tex,
                    size.0 / 32. * SIZE.0,
                    size.1 / 22. * SIZE.1,
                ));
            }
        }
    }
    // Порядок отрисовки: база ниже на экране — поверх; при равной базе крупнее — поверх.
    items.sort_unstable_by(|a, b| {
        let ycmp = a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal);
        if ycmp != std::cmp::Ordering::Equal {
            ycmp
        } else {
            gfx.tex_size(tex_cache[b.3])
                .1
                .partial_cmp(&gfx.tex_size(tex_cache[a.3]).1)
                .unwrap_or(std::cmp::Ordering::Equal)
        }
    });
    for (_base, x, y, tex, w, h) in items {
        gfx.draw_texture(tex_cache[tex], x, y, w, h, WHITE);
    }
    gfx.end_pass();
}

// Порт prepare_textures: запечь карту и декорации, вернуть TexId обоих слоёв.
pub fn prepare_textures(
    gfx: &mut Gfx,
    rts: (&Rt, &Rt),
    assets: &crate::assets::Assets,
    game: &Game,
    registry: &GameInfo,
    tile_pixels: &[image::RgbaImage],
) -> (TexId, TexId) {
    bake_map_textures(gfx, rts.0, assets, game, tile_pixels, BlendMode::Rounded);
    render_decos_layer(gfx, rts.1, assets, game, registry, 0, true);
    // NB: чтение RT (debug_check_gradient) — только ПОСЛЕ сабмита end_frame().
    (
        gfx.rt_as_texture(rts.0, Filter::Linear),
        gfx.rt_as_texture(rts.1, Filter::Linear),
    )
}

// ВРЕМЕННАЯ проверка: печатает RGB через границу вода→земля в запечённом RT —
// доказательство, что градиент наплыва реально есть (порт debug_check_gradient).
pub fn debug_check_gradient(
    gfx: &Gfx,
    rt: &Rt,
    game: &Game,
    tile_pixels: &[image::RgbaImage],
) {
    let img = gfx.read_rt(rt);
    let w = rt.size.0 as usize;
    let size = game.executor.gamemap.tilemap.size;
    let tilemap = &game.executor.gamemap.tilemap;
    let (tw, th) = (SIZE.0 as usize, SIZE.1 as usize);
    let mut found = 0usize;
    for j in 1..size {
        for i in 0..size {
            let id = tilemap[(j, i)];
            let up = tilemap[(j - 1, i)];
            if TILE_PRIORITY[up] > TILE_PRIORITY[id] && TILES[up].sprite() != TILES[id].sprite() {
                let (bx, by) = (i * tw, j * th);
                let base = |k: usize| {
                    let o = ((by + k) * w + bx + 16) * 4;
                    (img[o], img[o + 1], img[o + 2])
                };
                println!(
                    "boundary #{}: land {} at ({},{}) water {} above; rows px=16:",
                    found, id, i, j, up
                );
                for k in 0..7 {
                    let (r, g, b) = base(k);
                    println!("  row +{k}: rgb({r},{g},{b})");
                }
                let pat = neighbor_pattern(&tile_pixels[up]);
                for r in 18..22 {
                    let o = (r * tw + 16) * 3;
                    println!(
                        "  water mini row {r} col16: rgb({},{},{})",
                        pat[o],
                        pat[o + 1],
                        pat[o + 2]
                    );
                }
                let alpha0 = (OVERLAY_A.0 * smooth_falloff(0.5, SIZE.1 * OVERLAY_BAND)).round();
                let (bg, br, bb) = base(6);
                let o = (18 * tw + 16) * 3;
                let f = alpha0 / 255.;
                let er = (pat[o] as f32 * f + bg as f32 * (1. - f)) as u8;
                let eg = (pat[o + 1] as f32 * f + br as f32 * (1. - f)) as u8;
                let eb = (pat[o + 2] as f32 * f + bb as f32 * (1. - f)) as u8;
                println!(
                    "  alpha0={alpha0:.0} expected seam rgb({er},{eg},{eb}) vs measured rgb({},{},{})",
                    base(0).0,
                    base(0).1,
                    base(0).2
                );
                found += 1;
                if found >= 2 {
                    return;
                }
            }
        }
    }
    println!("no water-land boundaries found");
    let _ = colors::BLACK;
}
