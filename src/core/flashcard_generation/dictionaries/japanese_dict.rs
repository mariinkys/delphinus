#[cfg(feature = "ssr")]
use leptos::prelude::*;
#[cfg(feature = "ssr")]
use memmap2::Mmap;
#[cfg(feature = "ssr")]
use std::collections::HashMap;
#[cfg(feature = "ssr")]
use std::sync::Arc;

#[cfg(feature = "ssr")]
use crate::core::flashcard_generation::flashcard::DictionaryEntry;

#[cfg(feature = "ssr")]
#[derive(Clone)]
pub struct JapaneseDictionary {
    mmap: Arc<Mmap>,
    line_offsets: Vec<(usize, usize)>, // (start, end) positions for each line
}

#[cfg(feature = "ssr")]
impl JapaneseDictionary {
    /// Init the chinese dictionary
    pub async fn init(dictionary_path: String) -> Result<JapaneseDictionary, ServerFnError> {
        use leptos::logging::log;

        let file = std::fs::File::open(dictionary_path).expect("Failed to open file");
        let mmap = unsafe { memmap2::Mmap::map(&file).expect("Failed to create memory map") };
        let mmap = std::sync::Arc::new(mmap);

        // Build index of line positions instead of parsing all entries
        let mut line_offsets = Vec::with_capacity(140000);
        let mut start = 0;

        for (pos, &byte) in mmap.iter().enumerate() {
            if byte == b'\n' {
                line_offsets.push((start, pos));
                start = pos + 1;
            }
        }

        // Handle last line if file doesn't end with newline
        if start < mmap.len() {
            line_offsets.push((start, mmap.len()));
        }

        line_offsets.shrink_to_fit();
        log!(
            "Indexed {} lines in japanese dictionary",
            line_offsets.len()
        );

        Ok(JapaneseDictionary { mmap, line_offsets })
    }

    /// Get all entries of the dictionary
    pub fn entries(&self) -> impl Iterator<Item = DictionaryEntry> + '_ {
        self.line_offsets
            .iter()
            .enumerate()
            .filter_map(|(id, &(start, end))| {
                let line_bytes = &self.mmap[start..end];
                if let Ok(line) = std::str::from_utf8(line_bytes) {
                    self.parse_line(line, id)
                } else {
                    None
                }
            })
    }

    /// Method to search without loading all entries
    pub fn search(&self, query: &str) -> Vec<DictionaryEntry> {
        self.entries()
            .filter(|entry| entry.hanzi.contains(query) || entry.definition.contains(query))
            .collect()
    }

    /// Find entries for specific characters
    pub fn find_entries_for_chars(
        &self,
        search_chars: &std::collections::HashSet<&str>,
    ) -> HashMap<String, Vec<DictionaryEntry>> {
        let mut char_to_entries: HashMap<String, Vec<DictionaryEntry>> =
            HashMap::with_capacity(search_chars.len());

        for (id, &(start, end)) in self.line_offsets.iter().enumerate() {
            let line_bytes = &self.mmap[start..end];
            if let Ok(line) = std::str::from_utf8(line_bytes) {
                let contains_search_char = search_chars.iter().any(|&ch| line.contains(ch));
                if !contains_search_char {
                    continue;
                }

                if let Some(entry) = self.parse_line(line, id) {
                    let hanzi = entry.hanzi.trim();
                    if search_chars.contains(hanzi) {
                        char_to_entries
                            .entry(hanzi.to_string())
                            .or_default()
                            .push(entry);
                    }
                }
            }
        }

        char_to_entries
    }

    fn parse_line(&self, line: &str, id: usize) -> Option<DictionaryEntry> {
        let trimmed_line = line.trim();
        if trimmed_line.is_empty() {
            return None;
        }

        #[allow(clippy::collapsible_if)]
        if let Some(start_bracket) = trimmed_line.find('[') {
            if let Some(end_bracket) = trimmed_line.find(']') {
                let hanzi_part = &trimmed_line[..start_bracket].trim();
                let lecture = &trimmed_line[start_bracket + 1..end_bracket];
                let definitions = &trimmed_line[end_bracket + 1..];
                let hanzi = hanzi_part.split_whitespace().last().unwrap_or("").trim();

                if !hanzi.is_empty() {
                    return Some(DictionaryEntry {
                        id: id + 1,
                        hanzi: hanzi.to_string(),
                        lecture: lecture.to_string(),
                        definition: definitions.trim().to_string(),
                    });
                }
            }
        }
        None
    }
}
