// Запечка карты в offscreen RT: тайлы + CPU-генерация наплывов (блендинг) +
// слой декораций. Порт quad_ui main.rs:1164-1801.
use crate::camera::Camera;
use crate::gfx::{colors, Filter, Gfx, Rt, Target, TexId, WHITE};
use crate::state::{BlendMode, Game, SIZE};
use ahash::RandomState;
use dt_lib::map::deco::MapDeco;
use dt_lib::map::map::*;
use dt_lib::map::object::ObjectType;
use dt_lib::map::tile::TILES;
use dt_lib::battle::control::Control;
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
    for y in 0..tilemap.size {
        for x in 0..tilemap.size {
            // Конвенция (x=колонка, y=строка) — как hitmap/tile_at в игре.
            let tile_index = tilemap[(y, x)];
            gfx.draw_texture(
                tile_textures[tile_index],
                x as f32 * SIZE.0,
                y as f32 * SIZE.1,
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
// на соседа с меньшим (вода на всё, дорога на землю). Бленд есть у ЛЮБОЙ пары
// разных спрайтов; при равном приоритете направление — от большего id к меньшему.
// Одинаковые спрайты не смешиваются (полоса была бы невидимой).
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
/// Рисует наплывы поверх уже нарисованных тайлов. Возвращает число уникальных
/// текстур. Оверлей-текстуры кэшируются в gfx.overlay_cache ПЕРСИСТЕНТНО:
/// на пару тайлов 32×22-текстура создаётся один раз за сессию (локальный
/// кэш = до ~2000 текстур на каждый перезапек + утечка в gfx.textures).
// ---------------- Режим «Оригинал» (notes/тайлы, точная копия) ----------------
//
// Атлас тайла 256×242 = сетка 8×11 кусочков 32×22. Клетка (tx,ty) берёт
// кусочек по СВОЕЙ координате (периодический паттерн):
//   col_idx = ((tx+1)%8+8)%8; row_idx = ((ty+1)%11+11)%11
// Наплыв соседа: срез ТОЙ ЖЕ ориентации с атласа соседа, но ПОЛОВИНА
// кусочка (верх/низ — полвысоты, лево/право — полуширины), альфа —
// линейный градиент от шва BLEND_ALPHA=68 к 0 на середине клетки.
// Цвет + градиент в одну CPU-текстуру (эквивалент вертексной
// интерполяции DrawGradientQuad: оба линейны).

/// Линейная альфа (u8) по прогрессу 0..1 от шва: 68 → 0.
fn original_alpha(t: f32) -> u8 {
    (68. * (1. - t)).round().clamp(0., 255.) as u8
}

/// Оверлей «Оригинал» для клетки (tx,ty) и стороны: RGBA 32×22 (но
/// непусто только в половине клетки), цвет — кусок атласа СОСЕДА с
/// соответствующим сдвигом половины кусочка, альфа — линейный градиент.
/// side: 0=верх (верхняя половина, альфа 68 сверху), 1=низ (нижняя
/// половина, альфа 68 снизу), 2=лево (левая половина, 68 слева),
/// 3=право (правая половина, 68 справа).
fn make_original_overlay(
    neighbor_atlas: &image::RgbaImage,
    tx: usize,
    ty: usize,
    side: usize,
) -> Vec<u8> {
    let (w, h) = (SIZE.0 as usize, SIZE.1 as usize); // 32×22
    let mut bytes = vec![0u8; w * h * 4];
    // Кусочек СОСЕДА по координате ТЕКУЩЕЙ клетки (как в оригинале:
    // uv считаются от координаты клетки, куда наплывает сосед).
    let col_idx = ((tx + 1) % 8 + 8) % 8;
    let row_idx = ((ty + 1) % 11 + 11) % 11;
    let atlas = neighbor_atlas.as_raw();
    let sw = neighbor_atlas.width() as usize; // 256
    let sh = neighbor_atlas.height() as usize; // 242
    let half_h = h / 2; // 11
    let half_w = w / 2; // 16
    for py in 0..h {
        for px in 0..w {
            // Принадлежность клетки половине стороны.
            let (t, inside) = match side {
                0 => (py as f32 / half_h as f32, py < half_h),
                1 => (
                    (h - 1 - py) as f32 / half_h as f32,
                    py >= h - half_h,
                ),
                2 => (px as f32 / half_w as f32, px < half_w),
                _ => (
                    (w - 1 - px) as f32 / half_w as f32,
                    px >= w - half_w,
                ),
            };
            if !inside {
                continue;
            }
            // Пиксель кусочка атласа соседа: срез той же ориентации,
            // но полкубка (верх/низ/лево/право — по стороне).
            let (local_x, local_y) = match side {
                0 => (px, py), // верхняя половина кусочка
                1 => (px, py - (h - half_h)), // нижняя половина
                2 => (px, py), // левая половина по X
                _ => (px - (w - half_w), py), // правая половина по X
            };
            let ax = (col_idx * 32 + local_x) * sw / 256;
            let ay = (row_idx * 22 + local_y) * sh / 242;
            let src = (ay * sw + ax) * 4;
            let dst = (py * w + px) * 4;
            bytes[dst..dst + 3].copy_from_slice(&atlas[src..src + 3]);
            bytes[dst + 3] = original_alpha(t.clamp(0., 1.));
        }
    }
    bytes
}

/// Наплывы в стиле оригинала. Возвращает число использованных клеток-
/// оверлеев. Кэш: (neighbor_id, side, tx mod 8, ty mod 11) — периодика
/// атласа делает ключ полным.
pub fn draw_original_overlays(
    gfx: &mut Gfx,
    tilemap: &TileMap<usize>,
    tile_pixels: &[image::RgbaImage],
) -> usize {
    let size = tilemap.size;
    let mut draws: Vec<(TexId, f32, f32)> = Vec::new();
    for i in 0..size {
        for j in 0..size {
            let t_id = tilemap[(j, i)];
            let (cx, cy) = (i as f32 * SIZE.0, j as f32 * SIZE.1);
            // 4 прямых соседа; наплыв любого отличающегося (id<16 —
            // легальный terrain). Сортировка по id соседа — детерминизм.
            let mut edges: Vec<(usize, usize)> = Vec::new(); // (n_id, dir)
            let mut check = |ni: isize, nj: isize, dir: usize| {
                if ni >= 0 && nj >= 0 && (ni as usize) < size && (nj as usize) < size {
                    let n_id = tilemap[(nj as usize, ni as usize)];
                    if n_id != t_id && n_id < 16 {
                        edges.push((n_id, dir));
                    }
                }
            };
            check(i as isize, j as isize - 1, 0); // верх
            check(i as isize, j as isize + 1, 1); // низ
            check(i as isize - 1, j as isize, 2); // лево
            check(i as isize + 1, j as isize, 3); // право
            edges.sort_unstable_by_key(|e| e.0);
            for (n_id, dir) in edges {
                // Кэш-ключ: сосед, сторона, периодика атласа клетки.
                let key = (
                    n_id * 4 + dir,
                    (i + 1) % 8,
                    11 * 8 + (j + 1) % 11, // 88..98: ряд атласа, иная ветка ключей
                );
                let tex = if let Some(&t) = gfx.overlay_cache.get(&key) {
                    t
                } else {
                    let buf = make_original_overlay(&tile_pixels[n_id], i, j, dir);
                    let t =
                        gfx.push_texture_rgba(&buf, SIZE.0 as u32, SIZE.1 as u32, Filter::Nearest);
                    gfx.overlay_cache.insert(key, t);
                    t
                };
                draws.push((tex, cx, cy));
            }
        }
    }
    draws.sort_unstable_by_key(|d| d.0);
    for (tex, x, y) in draws {
        gfx.draw_texture(tex, x, y, SIZE.0, SIZE.1, WHITE);
    }
    gfx.overlay_cache.len()
}

/// Рисует наплывы поверх уже нарисованных тайлов. Возвращает число уникальных
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
    let mut pattern_cache: HashMap<usize, [u8; 32 * 22 * 3], RandomState> =
        HashMap::with_hasher(RandomState::new());
    let mut draws: Vec<(TexId, f32, f32)> = Vec::new();
    for i in 0..size {
        for j in 0..size {
            let id = tilemap[(j, i)];
            let same = |n: usize| n == id || TILES[n].sprite() == TILES[id].sprite();
            let pri = TILE_PRIORITY[id];
            // Наплыв: сосед «даёт край» на этот тайл. Направление — по
            // (приоритет, id): строго больший приоритет наплывает как раньше,
            // при равном — больший id; пары разных спрайтов не пропускаются.
            let bleeds = |n: usize| !same(n) && (TILE_PRIORITY[n], n) > (pri, id);
            let (cx, cy) = (i as f32 * SIZE.0, j as f32 * SIZE.1);
            // Кэш ключуется и по strong-режиму (альфа отличается):
            // side/corner кодируют 0..3, режим — битом 8.
            let mode_bit = if strong { 1 << 8 } else { 0 };
            // 4 стороны: вода/приоритетная сторона наплывает на соседа.
            let mut side = |di: isize, dj: isize, s: usize| {
                let (ni, nj) = (i as isize + di, j as isize + dj);
                if ni < 0 || nj < 0 || ni >= size as isize || nj >= size as isize {
                    return;
                }
                let n = tilemap[(nj as usize, ni as usize)];
                if !bleeds(n) {
                    return;
                }
                let key = (id, n, s | mode_bit);
                let tex = if let Some(&t) = gfx.overlay_cache.get(&key) {
                    t
                } else {
                    let pat = *pattern_cache
                        .entry(n)
                        .or_insert_with(|| neighbor_pattern(&tile_pixels[n]));
                    let buf = make_side_overlay(&pat, s, band_w, band_h, a);
                    let t = gfx.push_texture_rgba(&buf, SIZE.0 as u32, SIZE.1 as u32, Filter::Nearest);
                    gfx.overlay_cache.insert(key, t);
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
                    if !bleeds(n) {
                        return;
                    }
                    let key = (id, n, c | 4 | mode_bit);
                    let tex = if let Some(&t) = gfx.overlay_cache.get(&key) {
                        t
                    } else {
                        let pat = *pattern_cache
                            .entry(n)
                            .or_insert_with(|| neighbor_pattern(&tile_pixels[n]));
                        let buf = make_quad_overlay(&pat, c, band_w, band_h, a);
                        let t =
                            gfx.push_texture_rgba(&buf, SIZE.0 as u32, SIZE.1 as u32, Filter::Nearest);
                        gfx.overlay_cache.insert(key, t);
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
    gfx.overlay_cache.len()
}

// Запечка тайлов+наплывов в RT (порт bake_map_textures).
pub fn bake_map_textures(
    gfx: &mut Gfx,
    rt: &Rt,
    assets: &crate::assets::Assets,
    gamemap: &GameMap,
    tile_pixels: &[image::RgbaImage],
    blend_mode: BlendMode,
) {
    let size = gamemap.tilemap.size as f32;
    let camera = Camera::from_display_rect(0., SIZE.1 * size, SIZE.0 * size, -SIZE.1 * size);
    gfx.begin_pass(
        Target::Rt(rt.index),
        Some([0., 0., 0., 0.]),
        &camera,
    );
    let t_bake = std::time::Instant::now();
    draw_tiles(gfx, assets, &gamemap.tilemap);
    let mut n_overlays = 0;
    if blend_mode == BlendMode::Original {
        n_overlays = draw_original_overlays(gfx, &gamemap.tilemap, tile_pixels);
    } else if blend_mode != BlendMode::Off {
        n_overlays = draw_blend_overlays(gfx, &gamemap.tilemap, tile_pixels, blend_mode);
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

/// Тайловый слой карты РЕДАКТОРА: те же tile_pixels/BlendMode-наплывы, что в
/// игре, но запекание выполняет вызывающий (editor_view) в свой RT.
pub fn draw_editor_tiles(
    gfx: &mut Gfx,
    assets: &crate::assets::Assets,
    tilemap: &TileMap<usize>,
    tile_pixels: &[image::RgbaImage],
) {
    // Базовые спрайты — обязательный слой (без него пустая карта = пустой RT),
    // затем наплывы Rounded — как в игре (bake_map_textures).
    draw_tiles(gfx, assets, tilemap);
    let n = draw_blend_overlays(gfx, tilemap, tile_pixels, BlendMode::Rounded);
    let _ = n;
}

/// Запечка карты РЕДАКТОРА: тот же путь, что и игра (tile_pixels + GameMap),
/// но без реестра декора (проект Phase 1: кисть тайлов). RT — прозрачный,
/// размеры — SIZE×size. Вызывается из editor_view при bake_dirty.
pub fn bake_editor_map(
    gfx: &mut Gfx,
    rt: &Rt,
    gamemap: &GameMap,
    tile_pixels: &[image::RgbaImage],
) {
    let size = gamemap.tilemap.size as f32;
    let camera = Camera::from_display_rect(0., SIZE.1 * size, SIZE.0 * size, -SIZE.1 * size);
    gfx.begin_pass(Target::Rt(rt.index), Some([0., 0., 0., 0.]), &camera);
    let t_bake = std::time::Instant::now();
    let n_overlays = draw_blend_overlays(gfx, &gamemap.tilemap, tile_pixels, BlendMode::Rounded);
    gfx.end_pass();
    println!(
        "editor bake {}x{}: overlays {}, {:.1}ms",
        size,
        size,
        n_overlays,
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
    gamemap: &GameMap,
    registry: &GameInfo,
    seed: u64,
    buildings_render: bool,
    armies_render: bool,
) {
    let size = gamemap.tilemap.size as f32;
    let camera = Camera::from_display_rect(0., SIZE.1 * size, SIZE.0 * size, -SIZE.1 * size);
    gfx.begin_pass(Target::Rt(rt.index), Some([0., 0., 0., 0.]), &camera);

    let find_decos = |name: &'static str| -> Vec<MapDeco> {
        gamemap
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
    let rest = gamemap
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
        for b in &gamemap.buildings {
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
    if armies_render {
        for army in &gamemap.armys {
            if army.defeated {
                continue;
            }
            let (i, j) = army.pos;
            // Вода под армией → корабль по типу: феодальный/бандиты/торговый
            // (SHIP1/2/3 из assets_editor, предзагруженные вызывающим);
            // суша — моделька юнита как в игре (мертвяк/разбойник/феодал/
            // некромант/ГГследопыт), статичный кадр 40 (запек статичен).
            let on_water = {
                let t = gamemap.tilemap[(j, i)];
                dt_lib::map::tile::TILES[t].need_transport()
            };
            let unit_type = army
                .troops
                .get(0)
                .map(|tr| tr.get().unit.get_info(&registry.units).unit_type)
                .unwrap_or(dt_lib::units::unit::UnitType::People);
            let pic: String = if on_water {
                // Корабль по типу армии: Rogue → бандиты (SHIP2), иначе
                match unit_type {
                    dt_lib::units::unit::UnitType::Rogue => "SHIP2.png".into(),
                    _ => "SHIP1.png".into(),
                }
            } else {
                let model = match (&army.control, unit_type) {
                    (Control::Player(_), _) => "ГГследопыт",
                    (_, dt_lib::units::unit::UnitType::Undead) => "мертвяк",
                    (_, dt_lib::units::unit::UnitType::Rogue) => "разбойник",
                    (_, dt_lib::units::unit::UnitType::Hero
                    | dt_lib::units::unit::UnitType::People) => "феодал",
                    _ => "некромант",
                };
                format!("{model}/Unit_40.png")
            };
            let tex = match lookups.get(&pic) {
                Some(&t) => t,
                None => {
                    let t = tex_cache.len();
                    tex_cache.push(assets.get(&pic));
                    lookups.insert(pic.clone(), t);
                    t
                }
            };
            // 1×2 клетки, низ в клетке армии — как draw армий в map_view.
            items.push((
                j as f32 * SIZE.1,
                i as f32 * SIZE.0,
                j as f32 * SIZE.1 - SIZE.1,
                tex,
                SIZE.0,
                SIZE.1 * 2.,
            ));
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
    gamemap: &GameMap,
    registry: &GameInfo,
    tile_pixels: &[image::RgbaImage],
    armies_render: bool,
) -> (TexId, TexId) {
    bake_map_textures(gfx, rts.0, assets, gamemap, tile_pixels, BlendMode::Rounded);
    render_decos_layer(
        gfx,
        rts.1,
        assets,
        gamemap,
        registry,
        0,
        true,
        armies_render,
    );
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Наплыв: только пары разных спрайтов; при равном приоритете — больший id.
    #[test]
    fn bleeds_direction_by_priority_then_id() {
        let pri = |id: usize| TILE_PRIORITY[id];
        // равный приоритет: больший id наплывает на меньший
        assert!((pri(7), 7usize) > (pri(6), 6usize));
        assert!(!((pri(6), 6usize) > (pri(7), 7usize)));
        // вода (pri 3) наплывает на землю (pri 1)
        assert!((pri(1), 1usize) > (pri(6), 6usize));
    }
}
