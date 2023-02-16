use derive_builder::Builder;
use notan::{
    app::{App, Graphics, Plugins},
    draw::{Draw, DrawTextSection, Font},
    prelude::Assets,
};
use notan_ui::{defs::*, form::Form, rect::*, text::*, wrappers::*};
use std::fmt::{Debug, Formatter};
