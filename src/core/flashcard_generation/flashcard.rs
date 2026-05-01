use leptos::prelude::*;
use leptos::server;
use serde::{Deserialize, Serialize};

#[cfg(feature = "ssr")]
use crate::core::flashcard_generation::dictionaries;

pub fn parse_ch_input(input: &str) -> Vec<&str> {
    match input {
        "" => Vec::new(),
        s if s.contains(',') => s.split(',').collect(),
        s if s.contains('，') => s.split('，').collect(),
        s => vec![s],
    }
}

pub fn parse_jap_input(input: &str) -> Vec<&str> {
    match input {
        "" => Vec::new(),
        s if s.contains(',') => s.split(',').collect(),
        s if s.contains('、') => s.split('、').collect(),
        s => vec![s],
    }
}

pub fn remove_whitespace(input: &str) -> String {
    input.chars().filter(|&c| !c.is_whitespace()).collect()
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DictionaryEntry {
    pub id: usize,
    pub hanzi: String,
    pub lecture: String,
    pub definition: String,
}

#[derive(Clone, Serialize, Deserialize, Debug, Eq, PartialEq, Hash)]
pub struct Flashcard {
    pub id: i32,
    pub front: RwSignal<String>,
    pub back: RwSignal<String>,
}

#[server(SearchDictionary, "/searchdictionary")]
pub async fn search_dictionary(
    chars_string: String,
    is_ch: bool,
) -> Result<Vec<Flashcard>, ServerFnError> {
    use actix_web::web::Data;
    use leptos_actix::extract;
    use std::collections::HashSet;

    let chars_array = if is_ch {
        parse_ch_input(&chars_string)
    } else {
        parse_jap_input(&chars_string)
    };

    if chars_array.is_empty() {
        return Ok(Vec::new());
    }

    let mut flashcards = Vec::with_capacity(chars_array.len() * 2);
    let mut seen_chars = HashSet::with_capacity(chars_array.len());
    let mut id_counter = 1;

    let search_chars: HashSet<&str> = chars_array.iter().map(|s| s.trim()).collect();

    if is_ch {
        let ch_dictionary: Data<dictionaries::ChineseDictionary> = extract().await?;
        let char_to_entries = ch_dictionary.find_entries_for_chars(&search_chars);

        for &ch in &chars_array {
            let trimmed_ch = ch.trim();
            if seen_chars.insert(trimmed_ch) {
                if let Some(entries) = char_to_entries.get(trimmed_ch) {
                    for entry in entries {
                        flashcards.push(Flashcard {
                            id: id_counter,
                            front: RwSignal::new(entry.hanzi.clone()),
                            back: RwSignal::new(format!("{} {}", entry.lecture, entry.definition)),
                        });
                        id_counter += 1;
                    }
                } else {
                    flashcards.push(Flashcard {
                        id: id_counter,
                        front: RwSignal::new(trimmed_ch.to_string()),
                        back: RwSignal::new("NOT FOUND".to_string()),
                    });
                    id_counter += 1;
                }
            }
        }
    } else {
        let jap_dictionary: Data<dictionaries::JapaneseDictionary> = extract().await?;
        let char_to_entries = jap_dictionary.find_entries_for_chars(&search_chars);

        for &jp in &chars_array {
            let trimmed_jp = jp.trim();
            if seen_chars.insert(trimmed_jp) {
                if let Some(entries) = char_to_entries.get(trimmed_jp) {
                    for entry in entries {
                        flashcards.push(Flashcard {
                            id: id_counter,
                            front: RwSignal::new(entry.hanzi.clone()),
                            back: RwSignal::new(format!("{} {}", entry.lecture, entry.definition)),
                        });
                        id_counter += 1;
                    }
                } else {
                    flashcards.push(Flashcard {
                        id: id_counter,
                        front: RwSignal::new(trimmed_jp.to_string()),
                        back: RwSignal::new("NOT FOUND".to_string()),
                    });
                    id_counter += 1;
                }
            }
        }
    }

    Ok(flashcards)
}

pub fn create_vaia_import_string(flashcards: &[Flashcard]) -> String {
    let mut result = String::new();

    for (i, entry) in flashcards.iter().enumerate() {
        result = result + &entry.front.get_untracked() + "/#*#/" + &entry.back.get_untracked();

        if i + 1 != flashcards.len() {
            result.push_str("\n\\#\n");
        }
    }

    result
}

pub fn create_quizlet_import_string(flashcards: &[Flashcard]) -> String {
    let mut result = String::new();

    for (i, entry) in flashcards.iter().enumerate() {
        result = result + &entry.front.get_untracked() + "\t" + &entry.back.get_untracked();

        if i + 1 != flashcards.len() {
            result.push('\n');
        }
    }

    result
}

pub fn create_anki_hex_file(flashcards: &[Flashcard]) -> String {
    let mut result = String::new();
    result += "#separator:tab\n";
    result += "#html:false\n";

    for (i, entry) in flashcards.iter().enumerate() {
        result = result + &entry.front.get_untracked() + "\t" + &entry.back.get_untracked();

        if i + 1 != flashcards.len() {
            result.push('\n');
        }
    }

    encode_hex(&result)
}

fn encode_hex(input: &str) -> String {
    let mut result = String::new();
    for byte in input.bytes() {
        // Encode special characters
        if byte == b' ' {
            result.push_str("%20");
        } else if byte == b'\n' {
            result.push_str("%0A");
        } else {
            result.push_str(&format!("%{:02X}", byte));
        }
    }
    result
}

#[cfg(test)]
mod parse_tests {
    use super::*;

    #[test]
    fn test_parse_ch_input() {
        assert_eq!(parse_ch_input(""), Vec::<&str>::new());
        assert_eq!(parse_ch_input("你,好"), vec!["你", "好"]);
        assert_eq!(parse_ch_input("你，好"), vec!["你", "好"]);
        assert_eq!(parse_ch_input("你好"), vec!["你好"]);
    }

    #[test]
    fn test_parse_jap_input() {
        assert_eq!(parse_jap_input(""), Vec::<&str>::new());
        assert_eq!(parse_jap_input("学,校"), vec!["学", "校"]);
        assert_eq!(parse_jap_input("学、校"), vec!["学", "校"]);
        assert_eq!(parse_jap_input("学校"), vec!["学校"]);
    }

    #[test]
    fn test_remove_whitespace() {
        assert_eq!(remove_whitespace(" a b c "), "abc");
        assert_eq!(remove_whitespace("漢 字"), "漢字");
        assert_eq!(remove_whitespace(""), "");
    }
}
