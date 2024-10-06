use advini;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Locale {
    map: HashMap<String, HashMap<String, String>>,
    pub main_lang: String,
    pub additional_lang: String,
}
impl Locale {
    pub fn switch_lang(&mut self) {
        let main = self.main_lang.clone();
        let add = self.additional_lang.clone();
        self.main_lang = add;
        self.additional_lang = main;
    }
    pub fn set_lang(&mut self, lang: (&String, &String)) {
        self.main_lang = lang.0.clone();
        self.additional_lang = lang.1.clone();
    }
    pub fn get<K: AsRef<str> + ToString>(&self, id: K) -> String {
        let id = id.as_ref();
        self.map
            .get(&self.main_lang)
            .and_then(|lang_map| {
                lang_map.get(id).or_else(|| {
                    self.map
                        .get(&self.additional_lang)
                        .and_then(|lang_map| lang_map.get(id))
                })
            })
            .cloned()
            .unwrap_or(id.to_string())
    }
    pub fn insert<V: Into<String>, K: Into<String>>(&mut self, key: K, value: V, lang: &String) {
        let (k, v) = (key.into(), value.into());
        self.map
            .entry(lang.clone())
            .or_insert_with(HashMap::new)
            .insert(k, v);
    }
    pub fn new(main_lang: String, additional_lang: String) -> Self {
        Locale {
            map: HashMap::new(),
            main_lang,
            additional_lang,
        }
    }
}

trait IsRus {
    fn is_rus_alphabet(&self) -> bool;
}
impl IsRus for char {
    fn is_rus_alphabet(&self) -> bool {
        matches!(*self, 'А'..='Я' | 'а'..='я' | 'ё' | 'Ё')
    }
}

pub fn process_locale(locale: impl Into<String>, map_locale: &mut Locale) -> String {
    const LOCALE_START: char = '$';
    let locale = locale.into();
    let mut end_string = locale.clone();
    let mut locale_chars = locale.chars();
    for i in 0..locale.len() {
        if locale_chars.nth(i) == Some(LOCALE_START) {
            let end = locale
                .chars()
                .skip(i + 1)
                .position(|ch| {
                    !(ch.is_ascii_alphabetic()
                        || ch.is_ascii_digit()
                        || ch.is_rus_alphabet()
                        || ch == '_')
                })
                .unwrap_or(locale.len() - 1);
            if !(i + 1 == end) {
                let identificator = locale
                    .chars()
                    .skip(i + 1)
                    .take(end - i + 1)
                    .collect::<String>();
                end_string = end_string.replace(
                    &("$".to_owned() + &identificator),
                    &map_locale.get(&identificator),
                );
            }
        }
    }
    end_string
}

pub fn register_locale(
    locale_name: impl Into<String>,
    locale: impl Into<String>,
    lang: String,
    map_locale: &mut Locale,
) {
    let end_string = process_locale(locale, map_locale);
    map_locale.insert(locale_name.into(), end_string, &lang);
}

pub fn parse_locale(path: &str, language: &String, locale: &mut Locale) {
    let props = advini::parse_for_props(path);

    for (k, value) in props {
        locale.insert(k, value, language);
    }
}

pub fn parse_for_sections_localised(
    path: &str,
    locale: &mut Locale,
) -> Vec<(String, HashMap<String, String>)> {
    let ini_doc = read_file_as_string(path.into());
    advini::parse_for_sections_with(
        &ini_doc,
        |(prop, v, s)| (prop.to_lowercase(), process_locale(v, s)),
        locale,
    )
}

pub fn parse_map_locale(path: &str, languages: &[&String], locale: &mut Locale) {
    for (sec, props) in advini::parse_for_sections(path) {
        if !languages.contains(&&sec) {
            continue;
        }
        for prop in props {
            register_locale(prop.0, prop.1, sec.clone(), locale);
        }
    }
}

pub fn read_file_as_string(path: String) -> String {
    String::from_utf8(std::fs::read(path.clone()).unwrap()).unwrap()
}
