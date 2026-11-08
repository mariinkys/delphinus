//! Server-side dictionaries, read straight from the *original* distribution
//! files (JMdict XML and CC-CEDICT text) through a read-only memory map.
//!
//! Memory model
//! ------------
//! * The files are never copied into the heap. They are `mmap`ed, so their
//!   pages live in the kernel page cache: shared, lazily loaded and evictable.
//! * At start-up each file is scanned once to collect the byte offset of every
//!   headword. Those offsets (`u32`) are sorted by the bytes they point to, so a
//!   lookup is a binary search over the map. The whole index is ~4 bytes per
//!   headword: about 1.8 MB for JMdict and 0.9 MB for CC-CEDICT.
//! * Only the handful of entries that match a query are parsed, on demand.
//! * After indexing we tell the kernel we no longer need the scanned pages
//!   (`MADV_DONTNEED`) and switch to `MADV_RANDOM`, so the resident set falls
//!   back to a few MB and lookups do not trigger large read-ahead. The same
//!   release runs every 30 s after lookups (see `release_pages`)

pub mod cedict;
pub mod jmdict;
mod mapped;

pub use cedict::Cedict;
pub use jmdict::Jmdict;

use crate::model::{Candidate, Lang, Lookup, to_full_width, to_half_width};
use std::path::Path;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

/// Set by lookups; tells the periodic task there are pages to release
static TOUCHED: AtomicBool = AtomicBool::new(false);

pub struct Dictionaries {
    pub ja: Option<Jmdict>,
    pub zh: Option<Cedict>,
}

static DICTS: OnceLock<Dictionaries> = OnceLock::new();

impl Dictionaries {
    /// Opens both dictionaries. A missing file disables that language instead
    /// of aborting, so the app can run with only one of them
    pub fn load(jmdict: &Path, cedict: &Path) -> &'static Dictionaries {
        let ja = match Jmdict::open(jmdict) {
            Ok(d) => {
                eprintln!(
                    "JMdict: {} headwords indexed from {}",
                    d.headwords(),
                    jmdict.display()
                );
                Some(d)
            }
            Err(e) => {
                eprintln!("JMdict disabled ({}): {e}", jmdict.display());
                None
            }
        };
        let zh = match Cedict::open(cedict) {
            Ok(d) => {
                eprintln!(
                    "CC-CEDICT: {} headwords indexed from {}",
                    d.headwords(),
                    cedict.display()
                );
                Some(d)
            }
            Err(e) => {
                eprintln!("CC-CEDICT disabled ({}): {e}", cedict.display());
                None
            }
        };
        DICTS.get_or_init(|| Dictionaries { ja, zh })
    }

    pub fn get() -> Option<&'static Dictionaries> {
        DICTS.get()
    }

    pub fn available(&self, lang: Lang) -> bool {
        match lang {
            Lang::Ja => self.ja.is_some(),
            Lang::Zh => self.zh.is_some(),
        }
    }

    /// Drops the dictionary pages touched by recent lookups from this
    /// process's resident set. They stay in the kernel page cache, so the next
    /// access is a cheap minor fault rather than a disk read. Without this,
    /// binary searches gradually map most of both files into RSS
    pub fn release_pages(&self) {
        if TOUCHED.swap(false, Ordering::Relaxed) {
            if let Some(d) = &self.ja {
                d.release_pages();
            }
            if let Some(d) = &self.zh {
                d.release_pages();
            }
        }
    }

    fn contains(&self, lang: Lang, key: &str) -> bool {
        let has = |k: &str| match lang {
            Lang::Ja => self.ja.as_ref().is_some_and(|d| d.contains(k)),
            Lang::Zh => self.zh.as_ref().is_some_and(|d| d.contains(k)),
        };
        has(key) || has(&to_half_width(key))
    }

    fn find(&self, lang: Lang, term: &str) -> Vec<Candidate> {
        match lang {
            Lang::Ja => self.ja.as_ref().map(|d| d.lookup(term)),
            Lang::Zh => self.zh.as_ref().map(|d| d.lookup(term)),
        }
        .unwrap_or_default()
    }

    /// Splits a phrase or sentence into dictionary words, longest match
    /// first (我们去图书馆 → 我们 / 去 / 图书馆). Runs of characters that match
    /// nothing are kept together so they still show up as "not found"
    pub fn segment(&self, lang: Lang, text: &str) -> Vec<String> {
        const MAX_WORD: usize = 8;
        let chars: Vec<char> = text.chars().collect();
        let mut out = Vec::new();
        let mut unknown = String::new();
        let mut i = 0;
        while i < chars.len() {
            let longest = (1..=MAX_WORD.min(chars.len() - i)).rev().find_map(|len| {
                let word: String = chars[i..i + len].iter().collect();
                self.contains(lang, &word).then_some((word, len))
            });
            match longest {
                Some((word, len)) => {
                    if !unknown.is_empty() {
                        out.push(std::mem::take(&mut unknown));
                    }
                    out.push(word);
                    i += len;
                }
                None => {
                    if chars[i].is_alphanumeric() {
                        unknown.push(chars[i]);
                    }
                    i += 1;
                }
            }
        }
        if !unknown.is_empty() {
            out.push(unknown);
        }
        out
    }

    pub fn lookup(&self, lang: Lang, term: &str) -> Lookup {
        TOUCHED.store(true, Ordering::Relaxed);
        let mut candidates = self.find(lang, term);
        // The word box writes everything full-width (so it fits the grid),
        // but CC-CEDICT spells Latin letters half-width (`T恤`) and some
        // JMdict entries use full-width ones (`ＣＤ`): try the other form too
        if candidates.is_empty() {
            for alt in [to_half_width(term), to_full_width(term)] {
                if alt != term {
                    candidates = self.find(lang, &alt);
                    if !candidates.is_empty() {
                        break;
                    }
                }
            }
        }
        Lookup {
            term: term.to_string(),
            candidates,
        }
    }
}
