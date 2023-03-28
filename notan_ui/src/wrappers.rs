#![allow(unused_parens)]

use std::fmt::{Debug, Formatter};
use {
    std::{
        marker::PhantomData,
        ops::Not
    },
    derive_builder::Builder,
    notan::{
        prelude::{Graphics, Plugins, App, Assets},
        draw::*
    },
    super::{
        form::Form,
        rect::*,
        defs::*
}   };

type ButtonFunc<State, T> = fn(&mut Button<State, T>, &mut App, &mut Assets, &mut Plugins, &mut State);
#[derive(Clone, Builder)]
#[builder(build_fn(error = "StructBuildError"), pattern="owned")]
pub struct Button<State: UIStateCl, T: PosForm<State>> {
    #[builder(setter(into, strip_option), default)]
    pub inside: Option<T>,
    #[builder(default)]
    pub rect: Rect,
    #[builder(setter(strip_option), default="None")]
    pub if_hovered: Option<ButtonFunc<State, T>>,
    #[builder(setter(strip_option), default="None")]
    pub if_clicked: Option<ButtonFunc<State, T>>,
    #[builder(setter(skip), default = "false")]
    pub focused: bool,
    #[builder(setter(skip), default = "false")]
    pub selected: bool
}
pub fn button<State: UIStateCl, T: PosForm<State>>(inside: T, rect: Rect) -> ButtonBuilder<State, T> {
    ButtonBuilder::default()
        .inside(inside)
        .rect(rect)
}
impl<State: UIStateCl + Clone, T: PosForm<State>> Debug for Button<State, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Button")
            .field("inside", &self.inside)
            .field("rect", &self.rect)
            .field("focused", &self.focused)
            .field("selected", &self.selected)
            .finish()
}   }
impl<State: UIStateCl + Clone, T: PosForm<State>> Form<State> for Button<State, T> {
    fn draw(&mut self, app: &mut App, assets: &mut Assets, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State) {
        if let Some(form) = &mut self.inside {
            form.with_pos(self.rect.pos).draw( app, assets, gfx, plugins, state);
    }   }
    fn after(&mut self, app: &mut App, assets: &mut Assets, plugins: &mut Plugins, state: &mut State) {
        let mouse_pos = Position(app.mouse.x, app.mouse.y);
        self.focused = self.rect.collides(mouse_pos);
        self.selected = app.mouse.left_was_released() && self.focused;
        if self.focused {
            if let Some(func) = self.if_hovered { func(self, app, assets, plugins, state); }
            if self.selected {
                if let Some(func) = self.if_clicked { func(self, app, assets, plugins, state); }
        }   }
        if let Some(form) = &mut self.inside {
            form.with_pos(self.rect.pos).after(app, assets, plugins, state);
    }    }   }
impl<State: UIStateCl + Clone, T: PosForm<State>> Positionable for Button<State, T> {
    fn with_pos(&self, to_add: Position) -> Self {
        Self { rect: Rect { pos: self.rect.pos + to_add, size: self.rect.size }, ..self.clone() }
    }
    fn add_pos(&mut self, to_add: Position) {
        self.rect.pos += to_add;
    }
    fn get_size(&self) -> Size { self.rect.size }
    fn get_pos(&self) -> Position { self.rect.pos }
}
impl<State: UIStateCl + Clone, T: PosForm<State>> Button<State, T> {
    pub fn new(inside: Option<T>, rect: Rect,
               if_hovered: Option<fn(&mut Self, &mut App, &mut Assets, &mut Plugins, &mut State)>,
               if_clicked: Option<fn(&mut Self, &mut App, &mut Assets, &mut Plugins, &mut State)>
    ) -> Self {
        Self {
            inside,
            rect,
            if_hovered,
            if_clicked,
            focused: false,
            selected: false
}   }   }
impl<State: UIStateCl + Clone, T: PosForm<State>> Default for Button<State, T> {
    fn default() -> Self {
        Self {
            inside: None,
            rect: Default::default(),
            if_hovered: None,
            if_clicked: None,
            focused: false,
            selected: false
}   }   }

#[derive(Clone, Debug, Builder)]
#[builder(build_fn(error = "StructBuildError"), pattern="owned")]
pub struct Slider<State: UIStateCl, T: PosForm<State>> {
    #[builder(default)]
    pub rect: Rect,
    pub slider_inside: T,
    #[builder(default="0.")]
    pub scroll: f32,
    pub max_scroll: f32,
    #[builder(default="0.")]
    pub scroll_percent: f32,
    #[builder(default)]
    pub boo: PhantomData<State>
}
impl<State: UIStateCl, T: PosForm<State>> Form<State> for Slider<State, T> {
    fn draw(&mut self, app: &mut App, assets: &mut Assets, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State) {
        self.scroll_percent = -self.scroll / (self.rect.size.1 as f32) * self.max_scroll;
        self.slider_inside.with_pos(Position(self.rect.size.0, 0.) + Position(0., self.scroll_percent)).draw(app, assets, gfx, plugins, state);
    }
    fn after(&mut self, app: &mut App, assets: &mut Assets, plugins: &mut Plugins, state: &mut State) {
        self.slider_inside.with_pos(Position(0., self.scroll_percent) + self.rect.size.into()).after(app, assets, plugins, state);
    }   }
impl<State: UIStateCl, T: PosForm<State>> Positionable for Slider<State, T> {
    fn with_pos(&self, to_add: Position) -> Self {
        let mut cloned = self.clone();
        cloned.add_pos(to_add);
        cloned
    }
    fn add_pos(&mut self, to_add: Position) {
        self.rect.pos += to_add;
    }
    fn get_size(&self) -> Size {
        self.rect.size
    }
    fn get_pos(&self) -> Position { self.rect.pos }
}
impl<State: UIStateCl, T: PosForm<State> + Default> Default for Slider<State, T> {
    fn default() -> Self {
        Self {
            rect: Rect::default(),
            slider_inside: T::default(),
            scroll: 0.,
            max_scroll: 0.,
            scroll_percent: 0.,
            boo: PhantomData
}   }   }


#[derive(Clone, Builder)]
#[builder(build_fn(error = "StructBuildError"), pattern="owned")]
pub struct Checkbox<State: UIStateCl, T: PosForm<State>> {
    #[builder(setter(strip_option), default)]
    pub inside: Option<T>,
    #[builder(default)]
    pub rect: Rect,
    #[builder(default="false")]
    pub focused: bool,
    #[builder(default="false")]
    pub checked: bool,
    #[builder(setter(strip_option), default="None")]
    pub on_draw: Option<fn(&mut Checkbox<State, T>, &mut App, &mut Assets, &mut Graphics, &mut Plugins, &mut State)>,
    #[builder(setter(strip_option), default="None")]
    pub if_hovered: Option<fn(&mut Checkbox<State, T>, &mut App, &mut Assets, &mut Plugins, &mut State)>,
    #[builder(setter(strip_option), default="None")]
    pub if_selected: Option<fn(&mut Checkbox<State, T>, &mut App, &mut Assets, &mut Plugins, &mut State)>,
    #[builder(default)]
    pub pos: Position
}
pub fn checkbox<State: UIStateCl, T: PosForm<State>>(inside: T, rect: Rect) -> CheckboxBuilder<State, T> {
    CheckboxBuilder::default()
        .inside(inside)
        .rect(rect)
}
impl<State: UIStateCl + Clone, T: PosForm<State>> Debug for Checkbox<State, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Checkbox")
            .field("inside", &self.inside)
            .field("rect", &self.rect)
            .field("focused", &self.focused)
            .field("checked", &self.checked)
            .field("pos", &self.pos)
            .finish()
}   }
impl<State: UIStateCl + Clone, T: PosForm<State>> Form<State> for Checkbox<State, T> {
    fn draw(&mut self, app: &mut App, assets: &mut Assets, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State) {
        if let Some(fun) = self.on_draw {
            fun(self, app, assets, gfx, plugins, state);
        }
        if let Some(form) = &mut self.inside {
            if self.checked {
                form.with_pos(self.rect.pos).draw( app, assets, gfx, plugins, state);
    }   }   }
    fn after(&mut self, app: &mut App, assets: &mut Assets, plugins: &mut Plugins, state: &mut State) {
        let mouse_pos = Position(app.mouse.x, app.mouse.y);
        self.focused = self.rect.collides(mouse_pos);
        let clicked = app.mouse.left_was_released() && self.focused;
        if clicked {
            self.checked = self.checked.not();
        }
        if self.focused {
            if let Some(func) = self.if_hovered { func(self, app, assets, plugins, state); }
            if self.checked {
                if let Some(func) = self.if_selected { func(self, app, assets, plugins, state); }
        }   }
        if let Some(form) = &mut self.inside {
            if self.checked {
                form.after( app, assets, plugins, state);
    }    }  }   }
impl<State: UIStateCl, T: PosForm<State>> Positionable for Checkbox<State, T> {
    fn with_pos(&self, to_add: Position) -> Self {
        let mut cloned = self.clone();
        cloned.add_pos(to_add);
        cloned
    }
    fn add_pos(&mut self, to_add: Position) {
        self.pos += to_add;
    }
    fn get_size(&self) -> Size { self.rect.size }
    fn get_pos(&self) -> Position { self.pos }
}
impl<State: UIStateCl, T: PosForm<State>> Default for Checkbox<State, T> {
    fn default() -> Self {
        Self {
            inside: None,
            rect: Default::default(),
            focused: false,
            checked: false,
            on_draw: None,
            if_hovered: None,
            pos: Position(0., 0.),
            if_selected: None
}   }   }

#[derive(Clone, Debug)]
pub struct Mask<State: UIStateCl, T: PosForm<State>> {
    inside: Option<T>,
    mask_rect: Rect,
    pos: Position,
    boo: PhantomData<State>
}
impl<State: UIStateCl, T: PosForm<State>> Form<State> for Mask<State, T> {
    fn draw(&mut self, app: &mut App, assets: &mut Assets, gfx: &mut Graphics, plugins: &mut Plugins, state: &mut State) {
        let mut mask = gfx.create_draw();
        mask.rect(self.mask_rect.pos.into(), self.mask_rect.size.into());
        let draw = get_mut::<State, Draw>(state);
        draw.mask(Some(&mask));
        if let Some(form) = &mut self.inside {
            form.draw(app, assets, gfx, plugins, state);
        }
        let draw = get_mut::<State, Draw>(state);
        draw.mask(None);
    }

    fn after(&mut self, app: &mut App, assets: &mut Assets, plugins: &mut Plugins, state: &mut State) {
        if let Some(form) = &mut self.inside {
            form.with_pos(self.mask_rect.pos).after(app, assets, plugins, state);
}   }   }
impl<State: UIStateCl, T: PosForm<State>> Positionable for Mask<State, T> {
    fn with_pos(&self, to_add: Position) -> Self {
        let mut cloned = self.clone();
        cloned.add_pos(to_add);
        cloned
    }
    fn add_pos(&mut self, to_add: Position) {
        self.pos+=to_add;
    }
    fn get_size(&self) -> Size {
        self.mask_rect.size
    }
    fn get_pos(&self) -> Position {
        self.pos
    }   }
impl<State: UIStateCl, T: PosForm<State>> Default for Mask<State, T> {
    fn default() -> Self {
        Self {
            inside: None,
            mask_rect: Default::default(),
            pos: Default::default(),
            boo: Default::default()
}   }   }
