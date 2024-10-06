#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::{
    collections::HashMap,
    fs::{self, DirEntry, FileType, ReadDir},
    io::{self, Error},
    os,
    path::Path,
};
mod locale;
use advini::{self, parse_for_props, parse_for_sections};
use locale::*;
// hide console window on Windows in release
use eframe;
use egui;
// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0])
            .with_icon(
                // NOTE: Adding an icon is optional
                eframe::icon_data::from_png_bytes(&include_bytes!("../barbuta.png")[..])
                    .expect("Failed to load icon"),
            ),
        ..Default::default()
    };
    eframe::run_native(
        "eframe template",
        native_options,
        Box::new(|cc| Box::new(TheApp::new(cc))),
    )
}
fn parse_mod_info(entry: Result<DirEntry, Error>) -> io::Result<ModInfo> {
    let entry = entry?;
    let meta = entry.metadata()?;

    if !meta.is_dir() {
        return io::Result::Err(io::Error::new(io::ErrorKind::Other, "Expected a directory"));
    }
    let name = entry.file_name().into_string().unwrap();
    let translations: Vec<String> = fs::read_dir(entry.path())?
        .filter(|entry| entry.as_ref().is_ok_and(|v| v.path().ends_with("lang")))
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    Ok(ModInfo {
        name,
        translations,
        patches: Vec::new(),
    })
}
fn parse_mods() -> io::Result<Vec<ModInfo>> {
    let mut mods = Vec::new();
    for entry in fs::read_dir("./mods/")? {
        dbg!(&entry);
        if let Ok(mod_info) = parse_mod_info(entry) {
            mods.push(mod_info);
        }
    }
    Ok(mods)
}
fn parse_maps() -> io::Result<Vec<MapInfo>> {
    let mut maps = Vec::new();
    for entry in fs::read_dir("./maps/")? {
        dbg!(&entry);
        let entry = entry?;
        if entry.file_type()?.is_file() && entry.path().ends_with(".DTm") {
            maps.push(MapInfo {
                name: entry.file_name().into_string().unwrap(),
            });
        }
    }
    Ok(maps)
}
fn parse_translations() -> io::Result<Vec<String>> {
    let mut translations = Vec::new();
    for entry in fs::read_dir("./translations/")? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            translations.push(entry.file_name().into_string().unwrap());
        }
    }
    Ok(translations)
}
fn parse_mod_lang(path: &str, name: String) -> Locale {
    let mut locale = Locale::new(name.clone(), "".to_string());
    for (sec, props) in advini::parse_for_sections(path.into()) {
        match &*sec {
            "Global" => {
                for (key, value) in props {
                    if &*key == "base_language" {
                        locale.additional_lang = value;
                    }
                }
            }
            "Locale" => {
                for (key, value) in props {
                    locale.insert(key, value, &name);
                }
            }
            _ => {}
        }
    }
    locale
}
fn parse_base_lang(translations: Vec<String>, locale: &mut Locale) {
    let lang_name = locale.additional_lang.clone();
    if translations.contains(&lang_name) {
        parse_locale(
            &*format!("./translations/{}", &lang_name),
            &lang_name,
            locale,
        );
    } else {
        locale.additional_lang = "Russian".into();
        let lang_name = locale.additional_lang.clone();
        parse_locale("./tranlations/Russian.lang", &lang_name, locale);
    }
}
fn parse_deps() -> io::Result<HashMap<String, (Option<String>, Option<String>)>> {
    let mut deps = HashMap::new();
    for entry in fs::read_dir("./deps/")? {
        let entry = entry?;
        if entry.file_type()?.is_file() && entry.path().ends_with(".ini") {
            for (k, v) in parse_for_props(entry.path().to_str().unwrap()) {
                deps.insert(k, (Some(v), None));
            }
        }
    }
    Ok(deps)
}
fn parse_all_this_shit_up() -> TheApp {
    let mods = parse_mods().unwrap();
    let maps = parse_maps().unwrap();
    let translations = parse_translations().unwrap();
    let deps = parse_deps().unwrap();
    TheApp {
        mods,
        maps,
        translations,
        deps,
        cur_lang: None,
        cur_map: None,
        cur_mod: None,
    }
}

fn prepare_files(state: TheApp) {
    todo!()
}

#[derive(Clone, PartialEq)]
struct ModPatch {
    pub name: String,
}
#[derive(Clone, PartialEq)]
struct MapInfo {
    pub name: String,
}
#[derive(Clone, PartialEq)]
struct ModInfo {
    pub name: String,
    pub translations: Vec<String>,
    pub patches: Vec<String>,
}
#[derive(Default)]
struct TheApp {
    pub mods: Vec<ModInfo>,
    pub maps: Vec<MapInfo>,
    // key: Map name value: (maybe mod name, maybe patch name)
    pub deps: HashMap<String, (Option<String>, Option<String>)>,
    pub translations: Vec<String>,

    pub cur_mod: Option<ModInfo>,
    pub cur_map: Option<String>,
    pub cur_lang: Option<String>,
}
impl TheApp {
    fn new(cc: &eframe::CreationContext) -> Self {
        parse_all_this_shit_up()
    }
}
impl eframe::App for TheApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.label("ВРЕМЕНА РАЗДОРА ЗАПУСК");
                egui::ComboBox::from_label("Выбрать карту")
                    .selected_text(self.cur_map.clone().unwrap_or("Не выбрана".to_string()))
                    .show_ui(ui, |ui| {
                        let maps = if let Some(cur_mod) = &self.cur_mod {
                            &self
                                .maps
                                .iter()
                                .filter(|map| {
                                    self.deps
                                        .get(&map.name)
                                        .is_some_and(|v| v.0 == Some(cur_mod.name.clone()))
                                })
                                .cloned()
                                .collect()
                        } else {
                            &self.maps
                        };
                        ui.selectable_value(&mut self.cur_map, None, "Не выбрана");
                        for map in maps {
                            ui.selectable_value(
                                &mut self.cur_map,
                                Some(map.name.clone()),
                                map.name.clone(),
                            );
                        }
                    });
                egui::ComboBox::from_label("Выбрать мод")
                    .selected_text(
                        self.cur_mod
                            .clone()
                            .and_then(|v| Some(v.name))
                            .unwrap_or("Автоподбор".to_string()),
                    )
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.cur_mod, None, "Автоподбор");
                        if self.cur_map.is_none() {
                            for mod_info in &self.mods {
                                ui.selectable_value(
                                    &mut self.cur_mod,
                                    Some(mod_info.clone()),
                                    mod_info.name.clone(),
                                );
                            }
                        }
                    });
                egui::ComboBox::from_label("Выбрать язык")
                    .selected_text(self.cur_lang.clone().unwrap_or("Стандарт".to_string()))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.cur_lang, None, "Стандарт");
                        for lang in &self.translations {
                            ui.selectable_value(
                                &mut self.cur_lang,
                                Some(lang.clone()),
                                lang.clone(),
                            );
                        }
                        if let Some(cur_mod) = self.cur_mod.as_ref() {
                            for lang in &cur_mod.translations {
                                ui.selectable_value(
                                    &mut self.cur_lang,
                                    Some(lang.clone()),
                                    lang.clone(),
                                );
                            }
                        }
                    });
            })
        });
    }
}
