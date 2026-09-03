// Отрисовка боя: сетка карточек, статы, порядок ходов. Порт quad_ui main.rs
// unit_card_battle (717-979) и draw_battle (980-1145).
use crate::gfx::{colors, Gfx, WHITE};
use crate::Ctx;
use crate::assets::Assets;
use crate::state::{get_unit_texture, get_troop_texture, CARD_SIZE};
use crate::text::TextRenderer;
use crate::ui::{InputState, Rect};
use dt_lib::battle::battlefield::*;
use dt_lib::battle::troop::Troop;
use dt_lib::registry::GameInfo;
use dt_lib::units::unit::Unit;
use dt_lib::units::unitstats::ModifyUnitStats;
use dt_lib::battle::army::Army;

/// Прямоугольник клика + hover: бой рисуется в мире UI-камеры (1920x1080),
/// мышь конвертируется из физических пикселей (порт screen_to_world).
fn is_clicked(input: &InputState, r: Rect) -> bool {
    let s = input.screen_size();
    let m = [input.mouse[0] * crate::ui::UI_W / s[0], input.mouse[1] * crate::ui::UI_H / s[1]];
    input.mouse_button_released(0) && r.contains(m)
}

fn draw_stats(
    gfx: &mut Gfx,
    text: &mut TextRenderer,
    assets: &Assets,
    input: &InputState,
    troop: &Troop,
    mut pos: (f32, f32),
    draw_size: (f32, f32),
) {
    let bg_color = if troop.is_main {
        crate::gfx::rgba(168, 48, 48, 255)
    } else {
        crate::gfx::rgba(132, 46, 46, 255)
    };
    gfx.draw_rect(pos.0, pos.1, draw_size.0, draw_size.1, bg_color);
    let stats = &troop.unit.modified;
    let font_size = 26.;
    let font = assets.get_font(crate::assets::BENGUIAT);
    let line = |gfx: &mut Gfx,
                    text: &mut TextRenderer,
                    pos: (f32, f32),
                    icon: &str,
                    label: String| {
        let icon_tex = assets.get(icon);
        gfx.draw_texture(icon_tex, pos.0, pos.1, font_size, font_size, WHITE);
        text.draw_text(
            gfx,
            &label,
            pos.0 + font_size,
            pos.1 + font_size,
            font,
            26,
            1.,
            WHITE,
        );
    };
    line(
        gfx,
        text,
        pos,
        "hearts.png",
        format!("{}/{}", troop.unit.hp, stats.max_hp),
    );
    let start_pos = pos;
    if stats.damage.hand > 0 {
        pos.1 += font_size;
        line(
            gfx,
            text,
            pos,
            "crossed-swords.png",
            format!("{}", stats.damage.hand),
        );
    }
    if stats.damage.ranged > 0 {
        pos.1 += font_size;
        line(
            gfx,
            text,
            pos,
            "high-shot.png",
            format!("{}", stats.damage.ranged),
        );
    }
    if stats.damage.magic > 0 {
        pos.1 += font_size;
        line(
            gfx,
            text,
            pos,
            "wizard-staff.png",
            format!("{}", stats.damage.magic),
        );
    }
    if stats.defence.hand_units > 0 || stats.defence.ranged_units > 0 {
        if pos.1 - start_pos.1 >= font_size * 2. {
            pos.0 += text
                .measure(gfx, "32", font, 26, 1.)
                .width
                + 48.;
        } else {
            pos.1 += font_size;
        }
        line(
            gfx,
            text,
            pos,
            "breastplate.png",
            format!("{}/{}", stats.defence.hand_units, stats.defence.ranged_units),
        );
    }
    let label = format!("{}", stats.speed);
    let text_size = text.measure(gfx, &label, font, 26, 1.).width;
    let icon = assets.get("sprint.png");
    gfx.draw_texture(
        icon,
        start_pos.0 + draw_size.0 - font_size - text_size,
        start_pos.1,
        font_size,
        font_size,
        WHITE,
    );
    text.draw_text(
        gfx,
        &label,
        start_pos.0 + draw_size.0 - text_size,
        start_pos.1 + font_size,
        font,
        26,
        1.,
        WHITE,
    );
    let label = format!("Moves {}/{}", troop.unit.moves, stats.max_moves);
    let text_size = text.measure(gfx, &label, font, 26, 1.).width;
    text.draw_text(
        gfx,
        &label,
        start_pos.0 + draw_size.0 - text_size,
        start_pos.1 + font_size * 2.,
        font,
        26,
        1.,
        WHITE,
    );
    let _ = input;
}

/// Порт unit_card_battle. Возвращает true по клику на карточку.
#[allow(clippy::too_many_arguments)]
pub fn unit_card_battle(
    ctx: &mut Ctx,
    army: usize,
    unit_pos: usize,
    pos: (f32, f32),
    is_in_focus: bool,
    is_battle_active: bool,
    is_my_move: bool,
) -> bool {
    // Пустая клетка (нет войска): текстура поля + undercell + пульсирующая рамка.
    let armies = &ctx.game.executor.gamemap.armys;
    let Some(battle) = ctx.game.executor.battle.as_ref() else {
        return false;
    };
    if armies[army].get_troop(unit_pos).is_none() {
        let texture = ctx.assets.get(
            match field_type(unit_pos, 12) {
                Field::Back => "backyard.png",
                Field::Front => "front.png",
                Field::Reserve => "tent.png",
            },
        );
        let undercell = ctx.assets.get("undercell.png");
        ctx.gfx
            .draw_texture(texture, pos.0, pos.1, CARD_SIZE, CARD_SIZE, WHITE);
        ctx.gfx.draw_texture(
            undercell,
            pos.0,
            pos.1 + CARD_SIZE,
            CARD_SIZE,
            CARD_SIZE / 2.,
            WHITE,
        );
        if let Some(active_unit) = battle.active_unit {
            let old_pos = armies[active_unit.army].troops[active_unit.index].get().pos;
            if army == active_unit.army
                && (old_pos.0.abs_diff(unit_pos % (12 / 2)) < 2
                    || field_type(unit_pos, 12) == Field::Reserve
                    || field_type(old_pos.whole(6), 12) == Field::Reserve)
            {
                let outline_color = crate::gfx::rgba(
                    22,
                    22,
                    255,
                    (crate::time_secs().sin() * 128. + 64.) as u8,
                );
                ctx.gfx.draw_rect_lines(
                    pos.0,
                    pos.1,
                    CARD_SIZE,
                    CARD_SIZE,
                    15.,
                    outline_color,
                );
            }
        }
        return is_clicked(
            ctx.input,
            Rect::new(pos.0, pos.1, CARD_SIZE, CARD_SIZE),
        );
    }
    let troop_cell = armies[army].get_troop(unit_pos).unwrap();
    let troop = troop_cell.get();
    let unit_texture = get_unit_texture(ctx.assets, &troop.unit, &ctx.registry.units);
    // can_interact возвращает ЯКОРЬ многогексового юнита (pos.whole(columns));
    // карточка может быть второй клеткой — сверяем якорь её юнита. Якорь берём
    // из УЖЕ удержанного guard'а troop: повторный .get() по hitmap-индексу
    // захватывает тот же SendMut = дедлок.
    let anchor = Some(troop.pos.whole(ctx.registry.game_settings.max_troops / 2));
    let is_interactable = battle.can_interact.as_ref().is_some_and(|x| {
        anchor.is_some_and(|a| x.contains(&BattleUnitPos { army, pos: a }))
    });
    let info = &troop.unit.get_info(&ctx.registry.units);
    let size = info.size;
    let draw_size = (size.0 as f32 * CARD_SIZE, size.1 as f32 * CARD_SIZE);
    let draw_rect = Rect::new(pos.0, pos.1, draw_size.0, draw_size.1);
    let stats = &troop.unit.modified;
    let hp = 1. - troop.unit.hp as f32 / stats.max_hp as f32;
    // Свечение рисуется ПОД карточкой: радиальный квад с pad поверх фона,
    // затем спрайт юнита закрывает центр, оставляя мягкий ореол.
    if let Some(active_unit) = battle.active_unit {
        if is_my_move && is_interactable {
            let glow_color = if armies[army].hitmap[unit_pos]
                .is_some_and(|index| active_unit == BattleUnit { army, index })
            {
                [0.1, 1., 0.1]
            } else if active_unit.army != army {
                [1., 0.1, 0.1]
            } else {
                [0.1, 0.1, 1.]
            };
            // Период ~2.5с: 0.5-0.5*cos(2π t/T) — плавный цикл слабое -> сильное,
            // альфа 60..170 — никогда не нулевая.
            let pulse = 0.5 - 0.5 * (crate::time_secs() as f32 * std::f32::consts::TAU / 2.5).cos();
            let alpha = (60. + 110. * pulse) / 255.;
            let pad = 26.;
            let quad = [
                [pos.0 - pad, pos.1 - pad],
                [pos.0 + draw_size.0 + pad, pos.1 - pad],
                [pos.0 + draw_size.0 + pad, pos.1 + draw_size.1 + pad],
                [pos.0 - pad, pos.1 + draw_size.1 + pad],
            ];
            let uv = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
            ctx.gfx
                .draw_quad(ctx.glow_tex, quad, uv, [glow_color[0], glow_color[1], glow_color[2], alpha]);
        }
    }
    ctx.gfx
        .draw_texture(unit_texture, pos.0, pos.1, draw_size.0, draw_size.1, WHITE);
    draw_stats(
        ctx.gfx,
        ctx.text,
        ctx.assets,
        ctx.input,
        &troop,
        (pos.0, pos.1 + draw_size.1),
        (draw_size.0, CARD_SIZE / 2.),
    );
    ctx.gfx.draw_rect(
        pos.0,
        pos.1 + draw_size.1,
        draw_size.0,
        -draw_size.1 * hp,
        [hp, 0., 0., hp.min(0.9)],
    );
    if !is_battle_active && is_in_focus {
        ctx.gfx
            .draw_rect_lines(pos.0, pos.1, draw_size.0, draw_size.1, 15., colors::BLUE);
    }
    // Иконки предметов в верхней кромке карточки.
    let items = &troop.unit.inventory.items;
    for (i, item) in items.iter().enumerate() {
        if let Some(item) = item {
            let texture = ctx.assets.get(&item.get_info(&ctx.registry.items).icon);
            let item_pos = draw_size.0 / 4. * i as f32;
            ctx.gfx.draw_texture(
                texture,
                pos.0 + item_pos,
                pos.1,
                CARD_SIZE / 4.,
                CARD_SIZE / 4.,
                WHITE,
            );
        }
    }
    if is_clicked(ctx.input, draw_rect) {
        ctx.gfx
            .draw_rect_lines(pos.0, pos.1, draw_size.0, draw_size.1, 15., colors::BLACK);
        true
    } else {
        false
    }
}

/// Порт draw_battle. Возвращает winner.
pub fn draw_battle(ctx: &mut Ctx, is_battle_active: bool) -> Option<usize> {
    let is_my_move = if let crate::state::GameVariant::Online(online) = &ctx.game.variant {
        ctx.game
            .executor
            .battle
            .as_ref()
            .is_some_and(|battle| {
                battle
                    .active_unit
                    .is_some_and(|active| active.army == online.army)
            })
    } else {
        true
    };
    // Навигация фокуса (порт 992-1037).
    {
        let half = ctx.registry.game_settings.max_troops / 2;
        let (up, down, right, left) = (
            ctx.input.key_released(winit::keyboard::KeyCode::KeyW)
                || ctx.input.key_released(winit::keyboard::KeyCode::ArrowUp),
            ctx.input.key_released(winit::keyboard::KeyCode::KeyS)
                || ctx.input.key_released(winit::keyboard::KeyCode::ArrowDown),
            ctx.input.key_released(winit::keyboard::KeyCode::KeyD)
                || ctx.input.key_released(winit::keyboard::KeyCode::ArrowRight),
            ctx.input.key_released(winit::keyboard::KeyCode::KeyA)
                || ctx.input.key_released(winit::keyboard::KeyCode::ArrowLeft),
        );
        let Some(battle) = ctx.game.executor.battle.as_ref() else {
            return None;
        };
        let (army1, army2) = (battle.army1, battle.army2);
        let focus = &mut ctx.game.focus;
        if up || down {
            match (focus.pos, focus.army) {
                (pos, army) if pos >= half && army == army2 => {
                    if up {
                        focus.army = army1;
                    }
                    if down {
                        focus.pos -= half;
                    }
                }
                (pos, army) if pos >= half && army == army1 => {
                    if up {
                        focus.pos -= half;
                    }
                    if down {
                        focus.army = army2;
                    }
                }
                (pos, army) if pos <= half && army == army2 => {
                    if up {
                        focus.pos += half;
                    }
                    if down {
                        focus.army = army1;
                    }
                }
                (pos, army) if pos <= half && army == army1 => {
                    if up {
                        focus.army = army2;
                    }
                    if down {
                        focus.pos += half;
                    }
                }
                _ => {}
            }
        }
        if right {
            focus.pos = (focus.pos + 1) % (half * 2);
        }
        if left {
            focus.pos = (focus.pos.saturating_sub(1)) % (half * 2);
        }
    }
    let half_troops = 12 / 2;
    for army in 0..=1 {
        for row in 0..=1 {
            for i in 0..(12 / 2) {
                let unit_pos = i + (army as i64 - row as i64).abs() as usize * half_troops;
                let army_id = if army == 0 {
                    ctx.game.executor.battle.as_ref().unwrap().army1
                } else {
                    ctx.game.executor.battle.as_ref().unwrap().army2
                } as usize;
                // Многоклеточные юниты рисуются один раз (пропуск продолжений).
                let skip = {
                    let hitmap = &ctx.game.executor.gamemap.armys[army_id].hitmap;
                    let vertical = unit_pos >= 12 / 2 && hitmap[unit_pos] == hitmap[unit_pos - 12 / 2];
                    let horizontal = field_type(unit_pos, 12) != Field::Reserve
                        && hitmap[unit_pos] == hitmap[unit_pos - 1];
                    (vertical || horizontal) && hitmap[unit_pos].is_some()
                };
                if skip {
                    continue;
                }
                let pos = (
                    i as f32 * CARD_SIZE,
                    army as f32 * CARD_SIZE * 3. + row as f32 * CARD_SIZE * 1.5,
                );
                ctx.gfx
                    .draw_rect(pos.0, pos.1, CARD_SIZE, CARD_SIZE, crate::gfx::rgba(0, 0, 0, 1));
                let is_focused =
                    ctx.game.focus.army == army_id && ctx.game.focus.pos == unit_pos;
                if is_clicked(
                    ctx.input,
                    Rect::new(pos.0, pos.1, CARD_SIZE, CARD_SIZE),
                ) {
                    ctx.game.focus = BattleUnitPos {
                        pos: unit_pos,
                        army: army_id,
                    };
                }
                let clicked = unit_card_battle(
                    ctx,
                    army_id,
                    unit_pos,
                    pos,
                    is_focused,
                    is_battle_active,
                    is_my_move,
                );
                if clicked && ctx.game.executor.battle.as_ref().unwrap().winner.is_none() {
                    ctx.game.focus = BattleUnitPos {
                        pos: unit_pos,
                        army: army_id,
                    };
                    if !is_battle_active {
                        continue;
                    }
                    if let crate::state::GameVariant::Online(online) = &mut ctx.game.variant {
                        online.conn.send_action(dt_client::dt_server::Incoming::Action(
                            BattleUnitPos {
                                pos: unit_pos,
                                army: army_id,
                            },
                        ));
                    } else {
                        let battle = ctx.game.executor.battle.as_mut().unwrap();
                        let armies = &mut ctx.game.executor.gamemap.armys;
                        handle_action((unit_pos, army_id), battle, armies, ctx.registry);
                    }
                }
                if ctx.game.focus
                    == (BattleUnitPos {
                        army: army_id,
                        pos: unit_pos,
                    })
                    && !is_battle_active
                {
                    ctx.gfx.draw_rect_lines(
                        pos.0,
                        pos.1,
                        CARD_SIZE,
                        CARD_SIZE,
                        15.,
                        colors::BLUE,
                    );
                }
            }
        }
    }
    // Полоса порядка ходов (порт 1127-1143).
    let Some(battle) = ctx.game.executor.battle.as_ref() else {
        return None;
    };
    for (i, unit) in battle.move_order.iter().skip(1).take(10).enumerate() {
        let color = if unit.army == 0 { colors::BLUE } else { colors::RED };
        let pos = (i as f32 * 64., CARD_SIZE * 6.);
        let army = unit.army;
        let army_id = if army == 0 {
            battle.army1
        } else {
            battle.army2
        } as usize;
        let Some(texture) = get_troop_texture(
            ctx.assets,
            &ctx.game.executor.gamemap.armys,
            unit.index,
            army_id,
            &ctx.registry.units,
        ) else {
            continue;
        };
        ctx.gfx.draw_texture(texture, pos.0, pos.1, 64., 64., WHITE);
        ctx.gfx
            .draw_rect_lines(pos.0, pos.1, 64., 64., 10., with_alpha(color, 0.2));
    }
    ctx.game.executor.battle.as_ref().unwrap().winner
}

fn with_alpha(c: [f32; 4], a: f32) -> [f32; 4] {
    [c[0], c[1], c[2], a]
}

// Совместимость:<Unit> используется внутри draw_stats через Troop.
#[allow(dead_code)]
fn _unit_compat(_u: &Unit, _g: &GameInfo, _a: &Army, _gfx: &Gfx) {}
