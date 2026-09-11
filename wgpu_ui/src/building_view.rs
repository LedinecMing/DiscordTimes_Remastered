// Окно строения (экран Menu::Building): карта рендерится под окном, ввод карты
// отключён. Дизайн — UI_BUILDING_DESIGN.md: левая колонка вкладок (Главный зал/
// Рынок/Казарма/Заклинания + Выход), X в рамке, Esc.
// Снапшоты данных — до UiCtx (короткие guard'ы SendMut); мутации игры — в
// apply() после отрисовки; ввод карты не читается (отдельный экран).
use crate::gfx::{colors, Target, TexId};
use crate::state::{get_unit_info_texture, get_unit_texture, BuildingTab, Menu, SIZE};
use crate::ui::{Rect, UiCtx};
use crate::Ctx;
use dt_lib::battle::army::Army;
use dt_lib::battle::battlefield::field_type;
use dt_lib::battle::troop::Troop;
use dt_lib::map::object::{heal_cost, heal_for_gold, resurrect, resurrect_cost, BuildingVariant};
use dt_lib::network::server::Executor;
use dt_lib::units::unit::{UnitInfo, UnitPos};
use winit::keyboard::KeyCode;

const WINDOW_X: f32 = 110.;
const WINDOW_Y: f32 = 40.;
const WINDOW_W: f32 = 1920. - 2. * 110.;
const WINDOW_H: f32 = 1080. - 2. * 40.;
const TAB_W: f32 = 250.;
const CARD: f32 = 100.;
/// Цена изучения заклинания (в данных цены не моделируются — номинал-константа).
const SPELL_PRICE: u64 = 300;
/// Шрифт мелких подписей (мини-статы, цены).
const SMALL_FONT: u16 = 22;

/// Что сделать после отрисовки кадра (мутации игры — вне UiCtx-заимствований).
#[derive(Default)]
struct Actions {
    close: bool,
    switch: Option<(usize, BuildingTab)>,
    /// Тоггл выбора товара рынка к покупке (индекс market.items).
    buy_pick: Option<usize>,
    /// Тоггл выбора позиции инвентаря к продаже.
    sell_pick: Option<usize>,
    /// Закрепить юнит найма на карте статов (id реестра).
    inspect: Option<usize>,
    /// Пачка: покупки market_pick + продажи player_pick, лог результата.
    deal_done: bool,
    /// Найм: индекс в recruitment.units.
    hire: Option<usize>,
    /// Перестановка юнита армии: (слот_откуда, слот_куда).
    move_unit: Option<(usize, usize)>,
    /// Клик-выбор слота армии (drag_from).
    select_slot: Option<usize>,
    /// Сброс выбора слота.
    drag_clear: bool,
    /// Лечение за всё золото армии (сколько хватает): troops-индекс.
    heal_budget: Option<usize>,
    /// Лечение до конца за полную цену: troops-индекс.
    heal_full: Option<usize>,
    /// Воскрешение: troops-индекс.
    resurrect_troop: Option<usize>,
    /// Изучение заклинания: индекс в building.spells_to_learn.
    buy_spell: Option<usize>,
}

/// Снапшот юнита армии для сетки (съём — коротким guard'ом).
struct TroopSnap {
    troops_index: usize,
    slot: usize,
    size: (usize, usize),
    is_dead: bool,
    name: String,
    portrait: TexId,
    /// Мини-статы «A/D/Ini» и «Mnv/Hits» для подписи под карточкой.
    lines: [String; 2],
    heal_price: u64,
    resurrect_price: u64,
}

/// Снапшот юнита найма (из реестра Units).
struct HireSnap {
    unit: usize,
    count: usize,
    price: u64,
    name: String,
    portrait: TexId,
    detail: Vec<String>,
}

/// Строка витрины рынка.
struct MarketCard {
    icon: TexId,
    name: String,
    cost: u64,
    desc: String,
}

/// Слот инвентаря армии игрока.
struct InvCard {
    icon: Option<TexId>,
    name: String,
    cost: u64,
    desc: String,
}

/// Заклинание на изучение.
struct SpellSnap {
    spell: usize,
    name: String,
    desc: String,
    learned: bool,
}

/// Кадр окна строения. Возвращает true, если окно надо закрыть.
pub fn building_screen(ctx: &mut Ctx) -> bool {
    let Menu::Building(building_id, tab) = ctx.ui.main else {
        return true;
    };
    draw_map_frozen(ctx);
    // Пасс поверх с UI-камерой (clear=None: LoadOp::Load — рисуем ПОВЕРХ карты).
    ctx.gfx.begin_pass(Target::Screen, None, &ctx.ui.camera);

    let building = ctx.game.executor.gamemap.buildings[building_id].clone();
    let army_idx = player_army(&ctx.game.executor);
    let army = &mut ctx.game.executor.gamemap.armys[army_idx];
    let gold = army.stats.gold;
    let mana = army.stats.mana;
    let max_troops = army.max_troops;
    let columns = max_troops / 2;

    // ---- Снапшоты для UI (guard'ы SendMut короткие, отпускаем сразу) ----
    let market_cards: Vec<MarketCard> = building
        .market
        .as_ref()
        .map(|m| {
            m.items
                .iter()
                .map(|item| {
                    let info = item.get_info(&ctx.registry.items);
                    MarketCard {
                        icon: ctx.assets.get(&info.icon),
                        name: info.name.clone(),
                        cost: info.cost,
                        desc: info.description.clone(),
                    }
                })
                .collect()
        })
        .unwrap_or_default();
    let inv_cards: Vec<InvCard> = army
        .inventory
        .iter()
        .map(|slot| match slot {
            Some(item) => {
                let info = item.get_info(&ctx.registry.items);
                InvCard {
                    icon: Some(ctx.assets.get(&info.icon)),
                    name: info.name.clone(),
                    cost: info.cost,
                    desc: info.description.clone(),
                }
            }
            None => InvCard { icon: None, name: String::new(), cost: 0, desc: String::new() },
        })
        .collect();

    let can_resurrect = matches!(
        building.variant,
        BuildingVariant::Town | BuildingVariant::Village(_) | BuildingVariant::Church
    );

    let mut troops: Vec<TroopSnap> = Vec::new();
    for (i, cell) in army.troops.iter().enumerate() {
        let guard = cell.get();
        let troop: &Troop = &guard;
        let stats = &troop.unit.modified;
        troops.push(TroopSnap {
            troops_index: i,
            slot: troop.pos.whole(columns),
            size: troop.unit.get_info(&ctx.registry.units).size,
            is_dead: troop.is_dead(),
            name: troop.unit.get_info(&ctx.registry.units).name.clone(),
            portrait: get_unit_texture(ctx.assets, &troop.unit, &ctx.registry.units),
            lines: [
                format!(
                    "A: {} D: {}/{} Ini: {}",
                    stats.damage.hand,
                    stats.defence.hand_units,
                    stats.defence.ranged_units,
                    stats.speed
                ),
                format!(
                    "Mnv: {}/{} Hits: {}/{}",
                    troop.unit.moves, stats.max_moves, troop.unit.hp, stats.max_hp
                ),
            ],
            heal_price: heal_cost(&*guard),
            resurrect_price: resurrect_cost(&*guard, ctx.registry),
        });
    }
    let known_spell_names: Vec<String> = army
        .spells
        .iter()
        .map(|id| ctx.registry.effects[*id].name.clone())
        .collect();

    let hires: Vec<HireSnap> = building
        .recruitment
        .as_ref()
        .map(|rec| {
            rec.units
                .iter()
                .map(|recruit| {
                    let info = &ctx.registry.units[recruit.unit];
                    HireSnap {
                        unit: recruit.unit,
                        count: recruit.count,
                        price: info.cost_hire,
                        name: info.name.clone(),
                        portrait: get_unit_info_texture(ctx.assets, info),
                        detail: hire_detail_lines(info),
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    let known_ids = army.spells.clone();
    let spells: Vec<SpellSnap> = building
        .spells_to_learn
        .iter()
        .map(|spell| {
            let info = &ctx.registry.effects[*spell];
            SpellSnap {
                spell: *spell,
                name: info.name.clone(),
                desc: info.desc.clone(),
                learned: known_ids.contains(spell),
            }
        })
        .collect();

    // Слухи/события строения: events → executor.events[i].message.
    let rumors: Vec<String> = building
        .events
        .iter()
        .filter_map(|e| ctx.game.executor.events.get(*e).and_then(|ev| ev.message.clone()))
        .collect();

    // Текстуры-заглушки и кнопки (снапшот до UiCtx).
    let placeholder: [TexId; 3] = [
        ctx.assets.get("backyard.png"),
        ctx.assets.get("front.png"),
        ctx.assets.get("tent.png"),
    ];
    let buttonblue = ctx.assets.get("buttonblue.png");
    let paper = ctx.assets.get("Paper.png");

    let state_pick_buy = ctx.building_ui.market_pick.clone();
    let state_pick_sell = ctx.building_ui.player_pick.clone();
    let deal_log = ctx.building_ui.deal_log.clone();
    let inspect_unit = ctx.building_ui.inspect_unit;
    let drag_from = ctx.building_ui.drag_from;

    let mut actions = Actions::default();
    {
        let input = ctx.input;
        let mut ui = UiCtx::new(ctx.gfx, ctx.text, input, &ctx.skins.main, ctx.widgets);
        ui.window(WINDOW_X, WINDOW_Y, WINDOW_W, WINDOW_H, |mut ui| {
            ui.label(Some([WINDOW_W / 2. - 160., 10.]), &building.name);
            if ui.button(Some([WINDOW_W - 60., 4.]), "X") {
                actions.close = true;
            }
            // Левая колонка: вкладки по возможностям строения.
            let mut y = 80.;
            if tab != BuildingTab::Main
                && ui.button_sized([20., y], [TAB_W - 40., 50.], "Главный зал")
            {
                actions.switch = Some((building_id, BuildingTab::Main));
            }
            y += 60.;
            if building.market.is_some() {
                if tab != BuildingTab::Market
                    && ui.button_sized([20., y], [TAB_W - 40., 50.], "Рынок")
                {
                    actions.switch = Some((building_id, BuildingTab::Market));
                }
                y += 60.;
            }
            if building.recruitment.is_some() {
                if tab != BuildingTab::Recruits
                    && ui.button_sized([20., y], [TAB_W - 40., 50.], "Казарма")
                {
                    actions.switch = Some((building_id, BuildingTab::Recruits));
                }
                y += 60.;
            }
            if !building.spells_to_learn.is_empty() {
                if tab != BuildingTab::Spells
                    && ui.button_sized([20., y], [TAB_W - 40., 50.], "Заклинания")
                {
                    actions.switch = Some((building_id, BuildingTab::Spells));
                }
            }
            if ui.button_sized([20., WINDOW_H - 90.], [TAB_W - 40., 50.], "Выход") {
                actions.close = true;
            }
            // Полоса ресурсов внизу.
            ui.label_colored(
                [WINDOW_W - 380., WINDOW_H - 56.],
                &format!("Золото: {}   Мана: {}", gold, mana),
                colors::DARKGRAY,
            );
            match tab {
                BuildingTab::Main => draw_main_tab(&mut ui, paper, &rumors),
                BuildingTab::Market => draw_market_tab(
                    &mut ui,
                    &market_cards,
                    &inv_cards,
                    &state_pick_buy,
                    &state_pick_sell,
                    &deal_log,
                    &mut actions,
                ),
                BuildingTab::Recruits => draw_recruits_tab(
                    &mut ui,
                    &hires,
                    &troops,
                    &army.hitmap,
                    max_troops,
                    columns,
                    can_resurrect,
                    gold,
                    inspect_unit,
                    drag_from,
                    &placeholder,
                    buttonblue,
                    &mut actions,
                ),
                BuildingTab::Spells => {
                    draw_spells_tab(&mut ui, &spells, &known_spell_names, gold, &mut actions)
                }
            }
        });
    }
    apply(ctx, building_id, actions);
    // Мир продолжает жить, пока игрок в окне (доход/пути/события) — как на карте.
    if *ctx.delta > 0.2 {
        ctx.game.executor.tick(ctx.registry);
        *ctx.delta = 0.;
    }
    ctx.input.key_pressed(KeyCode::Escape)
}

fn player_army(executor: &Executor) -> usize {
    executor.players[0].army
}

/// Детальные строки найм-карточки из базовых статов реестра.
fn hire_detail_lines(info: &UnitInfo) -> Vec<String> {
    let s = &info.stats;
    let mut lines = vec![
        format!("Хиты: {}", s.max_hp),
        format!(
            "Атака: {} / {} / {}",
            s.damage.hand, s.damage.ranged, s.damage.magic
        ),
        format!(
            "Защита: {}/{}; от магии: {}/{}/{}",
            s.defence.hand_units,
            s.defence.ranged_units,
            s.defence.death_magic.get(),
            s.defence.life_magic.get(),
            s.defence.elemental_magic.get(),
        ),
        format!("Инициатива: {}", s.speed),
        format!("Манёвры: {}/{}", s.moves, s.max_moves),
    ];
    if !info.descript.is_empty() {
        lines.push(String::new());
        lines.push(info.descript.clone());
    }
    lines
}

/// Замороженная карта: запечённые слоёй камерой из сохранённых настроек
/// Menu::Map. Ввод карты не читается совсем.
fn draw_map_frozen(ctx: &mut Ctx) {
    let settings = ctx.map_settings.borrow().clone();
    let Some(settings) = settings else {
        return;
    };
    ctx.gfx
        .begin_pass(Target::Screen, Some(colors::WHITE), &settings.camera);
    let size = ctx.game.executor.gamemap.tilemap.size as u32;
    let world_w = SIZE.0 * size as f32;
    let world_h = SIZE.1 * size as f32;
    ctx.gfx
        .draw_texture(ctx.textures.map, 0., 0., world_w, world_h, colors::WHITE);
    ctx.gfx
        .draw_texture(ctx.textures.decos, 0., 0., world_w, world_h, colors::WHITE);
}

// ---------------- Виджеты поверх UiCtx ----------------

/// Кнопка на произвольной текстуре (синяя «Лечить» и т.п.).
fn tex_button(ui: &mut UiCtx, tex: TexId, pos: [f32; 2], size: [f32; 2], label: &str) -> bool {
    ui.gfx.draw_texture(tex, pos[0], pos[1], size[0], size[1], colors::WHITE);
    let tw = ui
        .text
        .measure(ui.gfx, label, ui.skin.font, SMALL_FONT, 1.)
        .width;
    ui.text.draw_text(
        ui.gfx,
        label,
        pos[0] + (size[0] - tw) / 2.,
        pos[1] + size[1] * 0.5 + SMALL_FONT as f32 * 0.35,
        ui.skin.font,
        SMALL_FONT,
        1.,
        colors::WHITE,
    );
    ui.is_clicked(Rect::new(pos[0], pos[1], size[0], size[1]))
}

/// Подсказка у курсора: тёмная плашка + строки. Рисовать ПОСЛЕДНЕЙ в кадре.
fn tooltip(ui: &mut UiCtx, lines: &[String]) {
    if lines.is_empty() {
        return;
    }
    let m = ui.mouse();
    let w = 360.;
    let h = lines.len() as f32 * (SMALL_FONT as f32 + 6.) + 12.;
    let x = (m[0] + 16.).min(WINDOW_X + WINDOW_W - w - 8.);
    let y = (m[1] + 16.).min(WINDOW_Y + WINDOW_H - h - 8.);
    ui.gfx.draw_rect(x, y, w, h, [0., 0., 0., 0.85]);
    ui.gfx.draw_rect_lines(x, y, w, h, 2., colors::DARKGRAY);
    for (i, line) in lines.iter().enumerate() {
        ui.text.draw_multiline(
            ui.gfx,
            line,
            (x + 8., y + 8. + i as f32 * (SMALL_FONT as f32 + 6.)),
            w - 16.,
            false,
            ui.skin.font,
            SMALL_FONT,
            colors::WHITE,
        );
    }
}

/// Крупная карта статистик юнита (портрет + строки), как пример-найм-2.
fn draw_stats_card(
    ui: &mut UiCtx,
    x: f32,
    y: f32,
    portrait: TexId,
    name: &str,
    detail: &[String],
    highlight: bool,
) {
    if highlight {
        ui.gfx.draw_rect_lines(x - 6., y - 6., 420., 330., 6., colors::GREEN);
    }
    ui.gfx.draw_texture(portrait, x, y, 200., 260., colors::WHITE);
    ui.label(Some([x + 220., y]), name);
    let mut dy = y + 40.;
    for line in detail {
        ui.label_colored([x + 220., dy], line, colors::WHITE);
        dy += 26.;
    }
}

// ---------------- Главное меню строения ----------------

fn draw_main_tab(ui: &mut UiCtx, paper: TexId, rumors: &[String]) {
    let (x, y, w) = (TAB_W + 20., 70., WINDOW_W - TAB_W - 60.);
    // Фон-картинка строения (дебаг: Menu.png из assets/Window).
    ui.gfx.draw_texture(ui.skin.window_tex, x, y, w, 340., colors::WHITE);
    // Окно слухов и событий: «бумага» + текст.
    let py = y + 360.;
    let ph = WINDOW_H - py - 60.;
    ui.gfx.draw_texture(paper, x, py, w, ph, colors::WHITE);
    ui.label(Some([x + 20., py + 16.]), "Слухи и события строения:");
    let text = if rumors.is_empty() {
        "(события строения пока не подключены)".to_string()
    } else {
        rumors.join("\n\n")
    };
    ui.text.draw_multiline(
        ui.gfx,
        &text,
        (x + 24., py + 60.),
        w - 48.,
        false,
        ui.skin.font,
        26,
        colors::BLACK,
    );
}

// ---------------- Рынок ----------------

fn draw_market_tab(
    ui: &mut UiCtx,
    market: &[MarketCard],
    inventory: &[InvCard],
    buy_pick: &[usize],
    sell_pick: &[usize],
    deal_log: &[String],
    actions: &mut Actions,
) {
    let area_x = TAB_W + 30.;
    ui.label(Some([area_x, 70.]), "Товары рынка (клик — выбрать к покупке):");
    let mut tip: Vec<String> = Vec::new();
    // Сетка товаров рынка.
    let mut x = area_x;
    let mut y = 120.;
    for (i, card) in market.iter().enumerate() {
        let r = Rect::new(x, y, CARD, CARD);
        ui.gfx.draw_texture(card.icon, x, y, CARD, CARD, colors::WHITE);
        if buy_pick.contains(&i) {
            ui.gfx.draw_rect_lines(x, y, CARD, CARD, 8., colors::GREEN);
        } else if ui.is_hovered(r) {
            ui.gfx.draw_rect_lines(x, y, CARD, CARD, 6., colors::GREEN);
        }
        if ui.is_hovered(r) {
            tip = vec![
                card.name.clone(),
                format!("Цена: {} зол.", card.cost),
                card.desc.clone(),
            ];
        }
        ui.label_colored(
            [x + 6., y + CARD + 4.],
            &format!("{} зол.", card.cost),
            colors::DARKGRAY,
        );
        if ui.is_clicked(r) {
            actions.buy_pick = Some(i);
        }
        x += CARD + 16.;
        if x + CARD > WINDOW_W - 40. {
            x = area_x;
            y += CARD + 50.;
        }
    }
    // Инвентарь игрока.
    let iy = y + CARD + 60.;
    ui.label(Some([area_x, iy - 36.]), "Ваш инвентарь (продажа = 50% стоимости):");
    x = area_x;
    y = iy + 10.;
    for (i, card) in inventory.iter().enumerate() {
        let r = Rect::new(x, y, CARD, CARD);
        if let Some(icon) = card.icon {
            ui.gfx.draw_texture(icon, x, y, CARD, CARD, colors::WHITE);
            if sell_pick.contains(&i) {
                ui.gfx.draw_rect_lines(x, y, CARD, CARD, 8., colors::ORANGE);
            } else if ui.is_hovered(r) {
                ui.gfx.draw_rect_lines(x, y, CARD, CARD, 6., colors::ORANGE);
            }
            if ui.is_hovered(r) {
                tip = vec![
                    card.name.clone(),
                    format!("Продажа: {} зол.", card.cost / 2),
                    card.desc.clone(),
                ];
            }
            if ui.is_clicked(r) {
                actions.sell_pick = Some(i);
            }
        }
        x += CARD + 16.;
        if x + CARD > WINDOW_W - 40. {
            x = area_x;
            y += CARD + 40.;
        }
    }
    // Лог последней сделки.
    let ly = iy + CARD + 70.;
    for (i, line) in deal_log.iter().enumerate() {
        ui.label_colored([area_x, ly + i as f32 * 30.], line, colors::DARKGRAY);
    }
    if ui.button_sized([WINDOW_W - 350., WINDOW_H - 120.], [310., 44.], "Совершить сделку") {
        actions.deal_done = true;
    }
    tooltip(ui, &tip);
}

// ---------------- Казарма (найм) ----------------

const HIRE_CARD_W: f32 = 150.;
const SLOT: f32 = 130.;

#[allow(clippy::too_many_arguments)]
fn draw_recruits_tab(
    ui: &mut UiCtx,
    hires: &[HireSnap],
    troops: &[TroopSnap],
    hitmap: &[Option<usize>],
    max_troops: usize,
    columns: usize,
    can_resurrect: bool,
    gold: u64,
    inspect_unit: Option<usize>,
    drag_from: Option<usize>,
    placeholder: &[TexId; 3],
    buttonblue: TexId,
    actions: &mut Actions,
) {
    let area_x = TAB_W + 30.;
    // --- Верх: карточки найма ---
    let mut hover_hire: Option<usize> = None;
    for (i, hire) in hires.iter().enumerate() {
        let x = area_x + i as f32 * (HIRE_CARD_W + 24.);
        let y = 80.;
        let card = Rect::new(x, y, HIRE_CARD_W, HIRE_CARD_W + 130.);
        let hovered = ui.is_hovered(card);
        if hovered {
            hover_hire = Some(hire.unit);
            ui.gfx
                .draw_rect_lines(x, y, HIRE_CARD_W, HIRE_CARD_W + 40., 6., colors::GREEN);
        }
        ui.gfx
            .draw_texture(hire.portrait, x, y, HIRE_CARD_W, HIRE_CARD_W + 40., colors::WHITE);
        ui.label_colored(
            [x + 6., y + HIRE_CARD_W + 46.],
            &format!("Цена = {}", hire.price),
            colors::DARKGRAY,
        );
        ui.label_colored(
            [x + 6., y + HIRE_CARD_W + 74.],
            &format!("В наличии: {}", hire.count),
            colors::DARKGRAY,
        );
        let label = if hire.count > 0 { "Нанять" } else { "нет" };
        if ui.button_sized([x + 8., y + HIRE_CARD_W + 100.], [HIRE_CARD_W - 16., 36.], label)
            && hire.count > 0
        {
            actions.hire = Some(i);
        }
    }
    // --- Крупная карта статистик: hover или закреплённый кликом юнит ---
    let shown_unit = hover_hire.or(inspect_unit).or_else(|| hires.first().map(|h| h.unit));
    let zone_x = area_x + 4. * (HIRE_CARD_W + 24.) + 24.;
    if let Some(unit) = shown_unit {
        if let Some(hire) = hires.iter().find(|h| h.unit == unit) {
            draw_stats_card(
                ui,
                zone_x,
                80.,
                hire.portrait,
                &hire.name,
                &hire.detail,
                hover_hire.is_some(),
            );
            ui.label_colored(
                [zone_x + 230., 44.],
                &format!("Цена найма = {} (в наличии: {})", hire.price, hire.count),
                colors::DARKGRAY,
            );
        }
    }
    if let Some(unit) = hover_hire {
        if ui.input.mouse_button_released(0) {
            actions.inspect = Some(unit);
        }
    }

    // --- Деньги ---
    ui.label_colored([area_x, 400.], &format!("Деньги: {}", gold), colors::DARKGRAY);

    // --- Низ: моя армия 6×2 ---
    ui.label(
        Some([area_x, 450.]),
        "Моя армия (клик — выбрать, клик по другому слоту — переместить; повторный клик — снять):",
    );
    let grid_y = 500.;
    for slot in 0..max_troops {
        let col = slot % columns;
        let row = slot / columns;
        let x = area_x + col as f32 * (SLOT + 12.);
        let y = grid_y + row as f32 * (SLOT + 70.);
        draw_army_slot(
            ui,
            troops,
            hitmap,
            max_troops,
            can_resurrect,
            slot,
            x,
            y,
            placeholder,
            buttonblue,
            drag_from,
            actions,
        );
    }
    // Курсор-перетаскивание: юнит следует за мышью, пока кнопка держится.
    if let Some(from) = drag_from {
        if ui.input.mouse_button_down(0) {
            if let Some(snap) = troops.iter().find(|t| hitmap.get(t.slot) == Some(&Some(t.troops_index)) && t.slot == from) {
                let m = ui.mouse();
                let w = snap.size.0 as f32 * SLOT * 0.7;
                let h = snap.size.1 as f32 * SLOT * 0.9;
                ui.gfx.draw_texture(
                    snap.portrait,
                    m[0] - w / 2.,
                    m[1] - h / 2.,
                    w,
                    h,
                    colors::WHITE,
                );
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_army_slot(
    ui: &mut UiCtx,
    troops: &[TroopSnap],
    hitmap: &[Option<usize>],
    max_troops: usize,
    can_resurrect: bool,
    slot: usize,
    x: f32,
    y: f32,
    placeholder: &[TexId; 3],
    buttonblue: TexId,
    drag_from: Option<usize>,
    actions: &mut Actions,
) {
    let r = Rect::new(x, y, SLOT, SLOT);
    // Мёртвый юнит вне hitmap рисуется на своём pos-слоте (затемнён).
    let dead_here = troops.iter().find(|t| t.is_dead && t.slot == slot);
    if let Some(dead) = dead_here {
        let w = dead.size.0 as f32 * SLOT;
        let h = dead.size.1 as f32 * SLOT;
        ui.gfx.draw_texture(dead.portrait, x, y, w, h, [0.45, 0.45, 0.45, 1.]);
        ui.label_colored([x + 4., y + 2.], "мёртв", colors::BLACK);
        if can_resurrect {
            if tex_button(
                ui,
                buttonblue,
                [x + 4., y + SLOT - 30.],
                [w - 8., 26.],
                &format!("Воскресить Цена = {}", dead.resurrect_price),
            ) {
                actions.resurrect_troop = Some(dead.troops_index);
            }
        }
    }
    if let Some(Some(troops_index)) = hitmap.get(slot).copied() {
        let snap = &troops[troops_index];
        let w = snap.size.0 as f32 * SLOT;
        let h = snap.size.1 as f32 * SLOT;
        ui.gfx.draw_texture(snap.portrait, x, y, w, h, colors::WHITE);
        // Мини-статы под карточкой.
        ui.text.draw_text(
            ui.gfx,
            &snap.lines[0],
            x,
            y + SLOT + 20.,
            ui.skin.font,
            SMALL_FONT,
            1.,
            colors::BLACK,
        );
        ui.text.draw_text(
            ui.gfx,
            &snap.lines[1],
            x,
            y + SLOT + 20. + SMALL_FONT as f32 + 2.,
            ui.skin.font,
            SMALL_FONT,
            1.,
            colors::BLACK,
        );
        // Кнопка лечения раненого живого юнита (за всё золото — «сколько хватает»).
        let mut slot_button = false;
        if !snap.is_dead && snap.heal_price > 0 {
            if tex_button(
                ui,
                buttonblue,
                [x + 4., y + SLOT - 30.],
                [w - 8., 26.],
                &format!("Лечить Цена = {}", snap.heal_price),
            ) {
                actions.heal_budget = Some(snap.troops_index);
                slot_button = true;
            }
        }
        // Выделение: выбранный — зелёный, кандидат вставки — синий.
        if drag_from == Some(slot) {
            ui.gfx.draw_rect_lines(x, y, w, h, 8., colors::GREEN);
        } else if drag_from.is_some() && ui.is_hovered(r) {
            ui.gfx.draw_rect_lines(x, y, w, h, 6., colors::BLUE);
        } else if ui.is_hovered(r) {
            ui.gfx.draw_rect_lines(x, y, w, h, 4., colors::DARKGRAY);
        }
        // Клик-выбор / клик-вставка.
        if ui.is_clicked(r) && !slot_button {
            if drag_from == Some(slot) {
                actions.drag_clear = true;
            } else if let Some(from) = drag_from {
                actions.move_unit = Some((from, slot));
                actions.drag_clear = true;
            } else {
                actions.select_slot = Some(slot);
            }
        }
    } else {
        // Пустой слот: картинка-заглушка поля (front/backyard/tent).
        let tex = match field_type(slot, max_troops) {
            dt_lib::battle::battlefield::Field::Back => placeholder[0],
            dt_lib::battle::battlefield::Field::Front => placeholder[1],
            dt_lib::battle::battlefield::Field::Reserve => placeholder[2],
        };
        ui.gfx.draw_texture(tex, x, y, SLOT, SLOT, colors::WHITE);
        // Вставка выбранного юнита в пустой слот.
        if ui.is_clicked(r) {
            if let Some(from) = drag_from {
                actions.move_unit = Some((from, slot));
                actions.drag_clear = true;
            }
        }
    }
}

// ---------------- Заклинания ----------------

fn draw_spells_tab(
    ui: &mut UiCtx,
    spells: &[SpellSnap],
    known: &[String],
    gold: u64,
    actions: &mut Actions,
) {
    let area_x = TAB_W + 30.;
    ui.label(
        Some([area_x, 70.]),
        &format!("Заклинания строения (книга армии; цена {} зол.):", SPELL_PRICE),
    );
    ui.label_colored(
        [area_x, 110.],
        &format!("Деньги: {}", gold),
        colors::DARKGRAY,
    );
    let mut y = 150.;
    for (i, spell) in spells.iter().enumerate() {
        ui.label(Some([area_x, y]), &spell.name);
        ui.label_colored([area_x + 300., y], &format!("Цена = {}", SPELL_PRICE), colors::DARKGRAY);
        if spell.learned {
            ui.label_colored([area_x + 500., y + 4.], "Изучено", colors::GREEN);
        } else if ui.button_sized([area_x + 500., y - 8.], [160., 40.], "Купить") {
            actions.buy_spell = Some(i);
        }
        ui.text.draw_multiline(
            ui.gfx,
            &spell.desc,
            (area_x + 20., y + 34.),
            700.,
            false,
            ui.skin.font,
            24,
            colors::WHITE,
        );
        y += 120.;
    }
    // Книга армии: перечень изученных заклинаний.
    y += 20.;
    ui.label(Some([area_x, y]), "Книга заклинаний армии:");
    for (i, name) in known.iter().enumerate() {
        ui.label_colored(
            [area_x + 20., y + 40. + i as f32 * 30.],
            &format!("— {name}"),
            colors::DARKGRAY,
        );
    }
}

// ---------------- Применение действий ----------------

fn apply(ctx: &mut Ctx, building_id: usize, actions: Actions) {
    // Навигация: переключение вкладки сбрасывает выборы (индексы устаревают).
    if let Some((b, tab)) = actions.switch {
        ctx.ui.main = Menu::Building(b, tab);
        ctx.building_ui.market_pick.clear();
        ctx.building_ui.player_pick.clear();
        ctx.building_ui.drag_from = None;
        return;
    }
    if actions.close {
        return;
    }
    if let Some(i) = actions.buy_pick {
        toggle(&mut ctx.building_ui.market_pick, i);
    }
    if let Some(i) = actions.sell_pick {
        toggle(&mut ctx.building_ui.player_pick, i);
    }
    if let Some(u) = actions.inspect {
        ctx.building_ui.inspect_unit = Some(u);
    }
    if let Some(slot) = actions.select_slot {
        ctx.building_ui.drag_from = Some(slot);
    }
    if actions.drag_clear {
        ctx.building_ui.drag_from = None;
    }
    if let Some((from, to)) = actions.move_unit {
        move_army_unit(ctx, from, to);
        ctx.building_ui.drag_from = None;
    }
    if let Some(i) = actions.hire {
        hire_unit(ctx, building_id, i);
    }
    if let Some(troops_index) = actions.heal_budget {
        heal_troop(ctx, troops_index, None);
    }
    if let Some(troops_index) = actions.heal_full {
        heal_troop(ctx, troops_index, Some(building_id));
    }
    if let Some(troops_index) = actions.resurrect_troop {
        resurrect_troop(ctx, troops_index);
    }
    if let Some(i) = actions.buy_spell {
        learn_spell(ctx, building_id, i);
    }
    if actions.deal_done {
        market_deal(ctx, building_id);
    }
}

fn toggle(vec: &mut Vec<usize>, value: usize) {
    if let Some(pos) = vec.iter().position(|v| *v == value) {
        vec.remove(pos);
    } else {
        vec.push(value);
    }
}

/// Пачка сделки: продажи player_pick (50%), покупки market_pick (полная цена),
/// лог результата, сброс выбора.
fn market_deal(ctx: &mut Ctx, building_id: usize) {
    let items = &ctx.registry.items;
    let army_idx = player_army(&ctx.game.executor);
    let gamemap = &mut ctx.game.executor.gamemap;
    let mut log = Vec::new();

    // Продажи: слоты инвентаря стабильны (sell ставит None, не сдвигает).
    let mut sell_picks = ctx.building_ui.player_pick.clone();
    sell_picks.sort_unstable();
    for pos in sell_picks {
        let name = gamemap.armys[army_idx].inventory.get(pos).copied().flatten().map(|it| {
            it.get_info(items).name.clone()
        });
        let Some(market) = gamemap.buildings[building_id].market.as_mut() else {
            continue;
        };
        match market.sell(&mut gamemap.armys[army_idx], pos, items) {
            Ok(revenue) => log.push(format!("Продано: {} (+{} зол.)", name.unwrap_or_default(), revenue)),
            Err(()) => log.push(format!("Продажа не удалась: {}", name.unwrap_or_default())),
        }
    }

    // Покупки: по убыванию индексов — удаление не сдвигает оставшиеся выбранные.
    let mut buy_picks = ctx.building_ui.market_pick.clone();
    buy_picks.sort_unstable();
    buy_picks.reverse();
    for i in buy_picks {
        let Some(market) = gamemap.buildings[building_id].market.as_mut() else {
            continue;
        };
        let info = market.items.get(i).map(|it| it.get_info(items).clone());
        let Some(info) = info else {
            log.push(format!("Товара больше нет (#{})", i));
            continue;
        };
        if !info.sells {
            log.push(format!("Рынок не продаёт: {}", info.name));
            continue;
        }
        if market.can_buy(&gamemap.armys[army_idx], i, items) {
            let cost = market.get_item_cost(i, items);
            market.buy(&mut gamemap.armys[army_idx], i, items);
            log.push(format!("Куплено: {} (−{} зол.)", info.name, cost));
        } else {
            log.push(format!("Не хватает золота: {}", info.name));
        }
    }

    let before = log.len();
    let gold_after = gamemap.armys[army_idx].stats.gold;
    let _ = gold_after;
    if log.len() == before {
        log.push("Сделка: ничего не выбрано".to_string());
    }
    ctx.building_ui.deal_log = log;
    ctx.building_ui.market_pick.clear();
    ctx.building_ui.player_pick.clear();
}

fn hire_unit(ctx: &mut Ctx, building_id: usize, unit_num: usize) {
    let army_idx = player_army(&ctx.game.executor);
    let gamemap = &mut ctx.game.executor.gamemap;
    let Some(recruitment) = gamemap.buildings[building_id].recruitment.as_mut() else {
        return;
    };
    let _ = recruitment.buy(&mut gamemap.armys[army_idx], unit_num, ctx.registry);
}

/// Перестановка юнита армии: слот `from` → `to` (свободный — перенос,
/// занятый — обмен). Валидация мульти-гексов через Army::fit_to.
fn move_army_unit(ctx: &mut Ctx, from: usize, to: usize) {
    let army_idx = player_army(&ctx.game.executor);
    let gamemap = &mut ctx.game.executor.gamemap;
    let army = &mut gamemap.armys[army_idx];
    let columns = army.max_troops / 2;
    let Some(Some(a)) = army.hitmap.get(from).copied() else {
        return;
    };
    let b = army.hitmap.get(to).copied().flatten();
    let size_a = army.troops[a].get().unit.get_info(&ctx.registry.units).size;
    let size_b = b
        .map(|bi| army.troops[bi].get().unit.get_info(&ctx.registry.units).size);
    // Пробный hitmap: убрать обе переставляемые фигуры, проверить посадку.
    let mut hit = army.hitmap.clone();
    for cell in hit.iter_mut() {
        if *cell == Some(a) || b.is_some_and(|bi| *cell == Some(bi)) {
            *cell = None;
        }
    }
    let fit_a = Army::fit_to(&hit, size_a, columns, 2, to / columns, to % columns);
    let fit_b = match (b, size_b) {
        (Some(_), Some(sz)) => {
            Army::fit_to(&hit, sz, columns, 2, from / columns, from % columns)
        }
        (None, None) => true,
        _ => false,
    };
    if !(fit_a && fit_b) {
        return;
    }
    // Коммит: короткие guard'ы последовательно (никаких вложенных .get()).
    army.troops[a].get().pos = UnitPos::from_index(to, columns);
    if let Some(bi) = b {
        army.troops[bi].get().pos = UnitPos::from_index(from, columns);
    }
    army.recalc_army_hitmap(&ctx.registry.units);
}

/// Лечение: budget None — за всё золото армии («сколько хватает»), Some(цена
/// лечения) — до конца за полную цену. Списывает фактически потраченное.
fn heal_troop(ctx: &mut Ctx, troops_index: usize, full_price: Option<usize>) {
    let army_idx = player_army(&ctx.game.executor);
    let gamemap = &mut ctx.game.executor.gamemap;
    let budget = match full_price {
        None => gamemap.armys[army_idx].stats.gold,
        Some(_) => {
            let cell = &gamemap.armys[army_idx].troops[troops_index];
            heal_cost(&*cell.get())
        }
    };
    let cell = &mut gamemap.armys[army_idx].troops[troops_index];
    let mut guard = cell.get();
    if let Ok(spent) = heal_for_gold(&mut guard, budget, ctx.registry) {
        drop(guard);
        gamemap.armys[army_idx].stats.gold -= spent;
    }
}

fn resurrect_troop(ctx: &mut Ctx, troops_index: usize) {
    let army_idx = player_army(&ctx.game.executor);
    let gamemap = &mut ctx.game.executor.gamemap;
    let gold = gamemap.armys[army_idx].stats.gold;
    let cell = &mut gamemap.armys[army_idx].troops[troops_index];
    let mut guard = cell.get();
    if let Ok(spent) = resurrect(&mut guard, gold, ctx.registry) {
        drop(guard);
        gamemap.armys[army_idx].stats.gold -= spent;
    }
}

/// Изучение заклинания: запись в книгу армии (без наложения эффектов),
/// списание номинальной цены. Повторное изучение отклонено ядром.
fn learn_spell(ctx: &mut Ctx, building_id: usize, spell_i: usize) {
    let Some(spell) = ctx.game.executor.gamemap.buildings[building_id]
        .spells_to_learn
        .get(spell_i)
        .copied()
    else {
        return;
    };
    let army_idx = player_army(&ctx.game.executor);
    let army = &mut ctx.game.executor.gamemap.armys[army_idx];
    if army.stats.gold < SPELL_PRICE {
        return;
    }
    if matches!(army.learn_spell(spell), Ok(true)) {
        army.stats.gold -= SPELL_PRICE;
    }
}