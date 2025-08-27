use crate::parse::FileAccess;

use advini;
use itertools::Itertools;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Locale {
    pub map: HashMap<String, HashMap<String, String>>,
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
    pub fn keys_lack(&mut self) -> Vec<String> {
        let keys = self
            .map
            .get(&self.main_lang)
            .and_then(|map| Some(map.keys()));
        let keys_other = self
            .map
            .get(&self.additional_lang)
            .and_then(|map| Some(map.keys()));
        if let (Some(keys), Some(other_keys)) = (keys, keys_other) {
            let other_keys: Vec<_> = other_keys.collect();
            keys.filter(|key| !other_keys.contains(key))
                .cloned()
                .collect()
        } else {
            vec![]
        }
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

pub fn find_all_matches_in_string(
    string: &'_ str,
    pat: impl Fn(char) -> bool,
) -> Vec<(usize, usize)> {
    let string = string.chars().collect::<Vec<_>>();
    let indices = string
        .iter()
        .enumerate()
        .filter(|(_, c)| **c == '$')
        .map(|(x, _)| x);
    indices
        .filter_map(|i| {
            let mut counter = 1;
            loop {
                if string.get(i + counter).is_some_and(|c| pat(*c)) {
                    counter += 1;
                } else {
                    if counter == 1 {
                        break None;
                    } else {
                        break Some((i, counter));
                    }
                }
            }
        })
        .collect()
}
pub fn process_locale(locale: impl ToString, map_locale: &Locale) -> String {
    let mut locale = locale.to_string();
    let is_identifier = |ch: char| {
        ch.is_ascii_alphabetic() || ch.is_ascii_digit() || ch.is_rus_alphabet() || ch == '_'
    };
    let identifiers = find_all_matches_in_string(&locale, is_identifier);
    for identifier in identifiers {
        let Some(sub) = locale.get((identifier.0)..(identifier.0 + identifier.1)) else {
            continue;
        };
        let Some(identifier) = sub.strip_prefix("$") else {
            continue;
        };
        locale = locale.replace(sub, &map_locale.get(identifier));
    }
    locale
}
pub fn register_locale(
    locale_name: impl ToString,
    locale: impl ToString,
    lang: String,
    map_locale: &mut Locale,
) {
    let end_string = process_locale(locale, map_locale);
    map_locale.insert(locale_name.to_string(), end_string, &lang);
}

pub async fn parse_locale<Reader: FileAccess>(languages: &[&String], locale: &mut Locale) {
    for language in languages {
        parse_locale_doc(
            Reader::read_as_string(&format!("{}_Locale.ini", language)).await,
            &language,
            locale,
        );
    }
}
pub fn parse_locale_doc(ini_doc: String, language: &String, locale: &mut Locale) {
    let props = advini::parse_for_props(&ini_doc);
    for (k, value) in props {
        register_locale(k, value, language.clone(), locale);
    }
}
pub async fn parse_for_sections_localised<Reader: FileAccess>(
    path: &str,
    locale: &mut Locale,
) -> Vec<(String, HashMap<String, String>)> {
    let ini_doc = Reader::read_as_string(path).await;
    advini::parse_for_sections_with(
        &ini_doc,
        |(prop, v, s)| (prop.to_lowercase(), process_locale(v, s)),
        locale,
    )
}

pub async fn parse_map_locale<Reader: FileAccess>(
    path: &str,
    languages: &[&String],
    locale: &mut Locale,
) {
    for (sec, props) in advini::parse_for_sections(&Reader::read_as_string(path).await) {
        if !languages.contains(&&sec) {
            continue;
        }
        for prop in props {
            register_locale(prop.0, prop.1, sec.clone(), locale);
        }
    }
}
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn manual_locale() {
        let mut locale = Locale::new("Rus".into(), "Eng".into());
        locale.insert("key", "value", &"Rus".into());
        locale.insert("key1", "value1", &"Eng".into());
        assert!(process_locale("$key", &mut locale) == "value".to_string());
        assert!(process_locale("$key1", &mut locale) == "value1".to_string());
        assert!(process_locale("$key2", &mut locale) == "key2".to_string());
        assert!(dbg!(process_locale("$key $key1", &mut locale)) == "value value1".to_string());
    }
    #[test]
    fn locale_parsing() {
        let mut locale = Locale::new("Rus".into(), "Eng".into());
        let doc = r#"
a=test
dggdgdd=adddlk
b = test
c = 
test"#;
        parse_locale_doc(doc.to_string(), &"Rus".into(), &mut locale);
        dbg!(&locale);
        assert!(dbg!(process_locale("$a", &mut locale)) == "test".to_string());
        assert!(dbg!(process_locale("$b", &mut locale)) == "test".to_string());
        assert!(dbg!(process_locale("$c", &mut locale)) == "test".to_string());
        assert_eq!(locale.get("dggdgdd"), "adddlk".to_string())
    }
}
