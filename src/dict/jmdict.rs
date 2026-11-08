//! JMdict, read from the unmodified XML distribution (`JMdict_e` or the full
//! multilingual `JMdict`; only English glosses are used).
//!
//! The file is not run through a general XML parser: its DTD declares custom
//! entities (`&n;`, `&v5k;`…) that strict parsers reject, which is why the old
//! Go tool had to strip lines from the file first. Instead we
//! * index the byte offset of every `<keb>` (kanji form) and `<reb>` (reading)
//!   once at start-up, and
//! * parse only the `<entry>` elements that match a query, with a tiny tag
//!   scanner that reads entity references as plain codes (`&n;` → `n`).
//!
//! Element reference: https://www.edrdg.org/jmdict/jmdict_dtd_h.html

use super::mapped::{KeyIndex, Mapped};
use crate::model::{Candidate, MAX_CANDIDATES, Sense};
use memchr::{memchr, memchr_iter, memmem};
use std::{io, path::Path};

pub struct Jmdict {
    file: Mapped,
    index: KeyIndex,
}

/// A key (keb/reb text) runs until the closing tag
fn key_at(data: &[u8], off: u32) -> &[u8] {
    let rest = &data[off as usize..];
    &rest[..memchr(b'<', rest).unwrap_or(rest.len())]
}

impl Jmdict {
    pub fn open(path: &Path) -> io::Result<Self> {
        let file = Mapped::open(path)?;
        let index = Self::build_index(file.bytes());
        if index.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "no JMdict entries found (is the file gzipped?)",
            ));
        }
        file.settle();
        Ok(Self { file, index })
    }

    fn build_index(data: &[u8]) -> KeyIndex {
        // Skip the DTD so element declarations are never mistaken for data
        let body = memmem::find(data, b"<JMdict>").unwrap_or(0);
        let mut offsets = Vec::with_capacity(data.len() / 128);
        for p in memchr_iter(b'<', &data[body..]) {
            let p = body + p;
            let tag = &data[p + 1..(p + 5).min(data.len())];
            if tag == b"keb>" || tag == b"reb>" {
                offsets.push((p + 5) as u32);
            }
        }
        KeyIndex::build(data, offsets, key_at)
    }

    pub fn headwords(&self) -> usize {
        self.index.len()
    }

    /// Whether `key` is a headword (no entry parsing)
    pub fn contains(&self, key: &str) -> bool {
        !self
            .index
            .find(self.file.bytes(), key.as_bytes(), key_at)
            .is_empty()
    }

    pub fn release_pages(&self) {
        self.file.release();
    }

    pub fn lookup(&self, term: &str) -> Vec<Candidate> {
        let found = self.lookup_exact(term);
        if !found.is_empty() {
            return found;
        }
        // ひらがな ⇄ カタカナ fallback (e.g. "てれび" finds テレビ)
        match swap_kana(term) {
            Some(other) => self.lookup_exact(&other),
            None => found,
        }
    }

    fn lookup_exact(&self, term: &str) -> Vec<Candidate> {
        let data = self.file.bytes();
        let hits = self.index.find(data, term.as_bytes(), key_at);
        let open = memmem::FinderRev::new(b"<entry>");
        let close = memmem::Finder::new(b"</entry>");
        let mut seen: Vec<usize> = Vec::with_capacity(hits.len());
        let mut ranked: Vec<(Rank, Candidate)> = Vec::with_capacity(hits.len());
        for &off in hits {
            let off = off as usize;
            let Some(start) = open.rfind(&data[..off]) else {
                continue;
            };
            if seen.contains(&start) {
                continue;
            }
            seen.push(start);
            let end = close.find(&data[off..]).map_or(data.len(), |p| off + p);
            let Ok(xml) = std::str::from_utf8(&data[start..end]) else {
                continue;
            };
            if let Some(hit) = parse_entry(xml).candidate(term) {
                ranked.push(hit);
            }
        }
        ranked.sort_by_key(|(rank, _)| *rank);
        ranked
            .into_iter()
            .map(|(_, c)| c)
            .take(MAX_CANDIDATES)
            .collect()
    }
}

// Entry parsing

#[derive(Default, Debug)]
pub struct KEle<'a> {
    pub text: &'a str,
    pub info: Vec<&'a str>,
    pub common: bool,
}

#[derive(Default, Debug)]
pub struct REle<'a> {
    pub text: &'a str,
    pub nokanji: bool,
    pub restr: Vec<&'a str>,
    pub info: Vec<&'a str>,
    pub common: bool,
}

#[derive(Default, Debug)]
pub struct RawSense<'a> {
    pub stagk: Vec<&'a str>,
    pub stagr: Vec<&'a str>,
    pub pos: Vec<&'a str>,
    pub misc: Vec<&'a str>,
    pub glosses: Vec<String>,
}

#[derive(Default, Debug)]
pub struct Entry<'a> {
    pub seq: u32,
    pub kanji: Vec<KEle<'a>>,
    pub readings: Vec<REle<'a>>,
    pub senses: Vec<RawSense<'a>>,
}

/// The priority codes JMdict itself treats as "common" (the `(P)` marker)
fn is_common(pri: &str) -> bool {
    matches!(pri, "news1" | "ichi1" | "spec1" | "spec2" | "gai1")
}

/// `&v5k;` → `v5k`
fn entity(text: &str) -> &str {
    text.trim().trim_start_matches('&').trim_end_matches(';')
}

fn unescape(text: &str) -> String {
    if !text.contains('&') {
        return text.to_string();
    }
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

/// Glosses without `xml:lang` are English; the full JMdict tags the others
fn is_english(attrs: &str) -> bool {
    match attrs.find("xml:lang=") {
        None => true,
        Some(i) => attrs[i + 9..]
            .trim_start_matches(['"', '\''])
            .starts_with("eng"),
    }
}

pub fn parse_entry(xml: &str) -> Entry<'_> {
    let b = xml.as_bytes();
    let mut e = Entry::default();
    let mut i = 0;
    while let Some(lt) = memchr(b'<', &b[i..]).map(|p| p + i) {
        let Some(gt) = memchr(b'>', &b[lt..]).map(|p| p + lt) else {
            break;
        };
        let raw = &xml[lt + 1..gt];
        i = gt + 1;
        if raw.starts_with('/') || raw.starts_with('!') || raw.starts_with('?') {
            continue;
        }
        let self_closing = raw.ends_with('/');
        let raw = raw.trim_end_matches('/');
        let (name, attrs) = raw.split_once(char::is_whitespace).unwrap_or((raw, ""));
        let text = if self_closing {
            ""
        } else {
            let end = memchr(b'<', &b[i..]).map_or(b.len(), |p| p + i);
            xml[i..end].trim()
        };
        match name {
            "ent_seq" => e.seq = text.parse().unwrap_or(0),
            "k_ele" => e.kanji.push(KEle::default()),
            "r_ele" => e.readings.push(REle::default()),
            "sense" => e.senses.push(RawSense::default()),
            _ => {}
        }
        if let Some(k) = e.kanji.last_mut() {
            match name {
                "keb" => k.text = text,
                "ke_inf" => k.info.push(entity(text)),
                "ke_pri" => k.common |= is_common(text),
                _ => {}
            }
        }
        if let Some(r) = e.readings.last_mut() {
            match name {
                "reb" => r.text = text,
                "re_nokanji" => r.nokanji = true,
                "re_restr" => r.restr.push(text),
                "re_inf" => r.info.push(entity(text)),
                "re_pri" => r.common |= is_common(text),
                _ => {}
            }
        }
        if let Some(s) = e.senses.last_mut() {
            match name {
                "stagk" => s.stagk.push(text),
                "stagr" => s.stagr.push(text),
                "pos" => s.pos.push(entity(text)),
                "misc" => s.misc.push(entity(text)),
                "gloss" if is_english(attrs) && !text.is_empty() => s.glosses.push(unescape(text)),
                _ => {}
            }
        }
    }
    // Per the DTD, a sense without its own part-of-speech inherits the
    // previous one. Senses with no English gloss are dropped afterwards
    let mut last_pos: Vec<&str> = Vec::new();
    for s in &mut e.senses {
        if s.pos.is_empty() {
            s.pos = last_pos.clone();
        } else {
            last_pos = s.pos.clone();
        }
    }
    e.senses.retain(|s| !s.glosses.is_empty());
    e
}

/// Lower sorts first: common words, then kana-first words for kana queries,
/// then the position of the matched form inside its entry, then file order
type Rank = (bool, bool, usize, u32);

/// Irregular, outdated, rare or search-only spellings/readings
fn is_unusual(info: &[&str]) -> bool {
    info.iter().any(|i| {
        matches!(
            *i,
            "iK" | "oK" | "rK" | "sK" | "ik" | "ok" | "rk" | "sk" | "io"
        )
    })
}

impl<'a> Entry<'a> {
    pub fn candidate(&self, query: &str) -> Option<(Rank, Candidate)> {
        if let Some(ki) = self.kanji.iter().position(|k| k.text == query) {
            return Some(self.kanji_candidate(ki));
        }
        let ri = self.readings.iter().position(|r| r.text == query)?;
        Some(self.kana_candidate(ri))
    }

    fn kanji_candidate(&self, ki: usize) -> (Rank, Candidate) {
        let k = &self.kanji[ki];
        let mut readings: Vec<&REle> = self
            .readings
            .iter()
            .filter(|r| !r.nokanji && (r.restr.is_empty() || r.restr.contains(&k.text)))
            .collect();
        if readings.iter().any(|r| !is_unusual(&r.info)) {
            readings.retain(|r| !is_unusual(&r.info));
        }
        readings.truncate(3);
        let senses = self.senses_for(|s| {
            (s.stagk.is_empty() || s.stagk.contains(&k.text))
                && (s.stagr.is_empty() || readings.iter().any(|r| s.stagr.contains(&r.text)))
        });
        let variants = self
            .kanji
            .iter()
            .filter(|o| o.text != k.text && !is_unusual(&o.info))
            .map(|o| o.text.to_string())
            .take(3)
            .collect();
        let common = k.common || readings.first().is_some_and(|r| r.common);
        let cand = Candidate {
            headword: k.text.to_string(),
            reading: readings
                .iter()
                .map(|r| r.text)
                .collect::<Vec<_>>()
                .join("、"),
            variants,
            senses,
            common,
        };
        ((!common, false, ki, self.seq), cand)
    }

    fn kana_candidate(&self, ri: usize) -> (Rank, Candidate) {
        let r = &self.readings[ri];
        let spellings: Vec<&KEle> = if r.nokanji {
            Vec::new()
        } else {
            self.kanji
                .iter()
                .filter(|k| r.restr.is_empty() || r.restr.contains(&k.text))
                .collect()
        };
        let senses = self.senses_for(|s| {
            (s.stagr.is_empty() || s.stagr.contains(&r.text))
                && (s.stagk.is_empty()
                    || spellings.is_empty()
                    || spellings.iter().any(|k| s.stagk.contains(&k.text)))
        });
        let kana_first =
            spellings.is_empty() || self.senses.first().is_some_and(|s| s.misc.contains(&"uk"));
        let cand = Candidate {
            headword: r.text.to_string(),
            reading: String::new(),
            variants: spellings
                .iter()
                .filter(|k| !is_unusual(&k.info))
                .map(|k| k.text.to_string())
                .take(3)
                .collect(),
            senses,
            common: r.common,
        };
        ((!r.common, !kana_first, ri, self.seq), cand)
    }

    fn senses_for(&self, keep: impl Fn(&RawSense) -> bool) -> Vec<Sense> {
        self.senses
            .iter()
            .filter(|s| keep(s))
            .map(|s| Sense {
                tags: s.pos.iter().map(|p| p.to_string()).collect(),
                glosses: s.glosses.clone(),
            })
            .collect()
    }
}

/// Hiragana → katakana or katakana → hiragana, if the text is entirely one of them
fn swap_kana(s: &str) -> Option<String> {
    let hira = |c: char| ('\u{3041}'..='\u{3096}').contains(&c);
    let kata = |c: char| ('\u{30A1}'..='\u{30F6}').contains(&c);
    let neutral = |c: char| c == 'ー';
    if s.chars().all(|c| hira(c) || neutral(c)) && s.chars().any(hira) {
        Some(
            s.chars()
                .map(|c| {
                    if hira(c) {
                        char::from_u32(c as u32 + 0x60).unwrap_or(c)
                    } else {
                        c
                    }
                })
                .collect(),
        )
    } else if s.chars().all(|c| kata(c) || neutral(c)) && s.chars().any(kata) {
        Some(
            s.chars()
                .map(|c| {
                    if kata(c) {
                        char::from_u32(c as u32 - 0x60).unwrap_or(c)
                    } else {
                        c
                    }
                })
                .collect(),
        )
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ENTRY: &str = r#"<entry>
<ent_seq>1234</ent_seq>
<k_ele><keb>明白</keb><ke_pri>news1</ke_pri></k_ele>
<k_ele><keb>明々白々</keb><ke_inf>&iK;</ke_inf></k_ele>
<r_ele><reb>めいはく</reb><re_pri>news1</re_pri></r_ele>
<r_ele><reb>あからさま</reb><re_restr>明白</re_restr></r_ele>
<r_ele><reb>メイハク</reb><re_nokanji/></r_ele>
<sense><pos>&adj-na;</pos><gloss>obvious</gloss><gloss>clear</gloss><gloss xml:lang="ger">klar</gloss></sense>
<sense><stagr>あからさま</stagr><gloss>plain &amp; frank</gloss></sense>
<sense><gloss xml:lang="fre">évident</gloss></sense>
</entry>"#;

    #[test]
    fn parses_entry() {
        let e = parse_entry(ENTRY);
        assert_eq!(e.seq, 1234);
        assert_eq!(e.kanji.len(), 2);
        assert_eq!(e.kanji[1].info, vec!["iK"]);
        assert!(e.readings[2].nokanji);
        assert_eq!(e.readings[1].restr, vec!["明白"]);
        assert_eq!(e.senses.len(), 2, "French-only sense dropped");
        assert_eq!(
            e.senses[0].glosses,
            vec!["obvious", "clear"],
            "German gloss dropped"
        );
        assert_eq!(e.senses[1].pos, vec!["adj-na"], "pos inherited");
        assert_eq!(e.senses[1].glosses, vec!["plain & frank"]);
    }

    #[test]
    fn builds_candidates() {
        let e = parse_entry(ENTRY);
        let (_, c) = e.candidate("明白").unwrap();
        assert_eq!(
            c.reading, "めいはく、あからさま",
            "nokanji reading excluded"
        );
        assert!(c.common);
        assert!(c.variants.is_empty(), "irregular spelling hidden");
        assert_eq!(c.senses.len(), 2);

        let (_, c) = e.candidate("明々白々").unwrap();
        assert_eq!(c.reading, "めいはく", "re_restr respected");
        assert_eq!(c.senses.len(), 1, "stagr respected");
        assert_eq!(c.variants, vec!["明白"]);

        let (_, c) = e.candidate("メイハク").unwrap();
        assert!(c.variants.is_empty(), "re_nokanji has no spellings");
        assert!(e.candidate("なし").is_none());
    }

    #[test]
    fn swaps_kana() {
        assert_eq!(swap_kana("てれび").as_deref(), Some("テレビ"));
        assert_eq!(swap_kana("テレビ").as_deref(), Some("てれび"));
        assert_eq!(swap_kana("コーヒー").as_deref(), Some("こーひー"));
        assert_eq!(swap_kana("漢字"), None);
    }

    /// Runs against the real JMdict excerpt (with its full DTD) when present.
    #[test]
    fn real_jmdict_excerpt() {
        let Ok(path) = std::env::var("JMDICT_SAMPLE") else {
            return;
        };
        let d = Jmdict::open(Path::new(&path)).unwrap();
        assert!(d.headwords() > 500);
        for (q, reading, first_gloss) in [
            ("明白", "めいはく", "obvious"),
            ("ＣＤプレーヤー", "シーディープレーヤー", "CD player"),
            ("あっという間に", "あっというま", "just like that"),
        ] {
            let r = d.lookup(q);
            assert!(!r.is_empty(), "{q} not found");
            assert!(r[0].reading.starts_with(reading), "{q}: {:?}", r[0]);
            assert!(
                r.iter()
                    .flat_map(|c| &c.senses)
                    .flat_map(|s| &s.glosses)
                    .any(|g| g.contains(first_gloss)),
                "{q}: {r:?}"
            );
        }
        assert!(!d.lookup("くりかえし").is_empty());
        assert!(d.lookup("ELEMENT").is_empty(), "DTD is not indexed");
    }
}
