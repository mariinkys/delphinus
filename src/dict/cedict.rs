//! CC-CEDICT, read from the unmodified `cedict_ts.u8` distribution file.
//!
//! Line format (https://cc-cedict.org/wiki/format:syntax):
//! ```text
//! # comment
//! 學 学 [xue2] /to learn/to study/science/-ology/
//! ```
//! Both the traditional and the simplified headword are indexed.

use super::mapped::{KeyIndex, Mapped};
use crate::model::{Candidate, MAX_CANDIDATES, Sense};
use memchr::{memchr, memchr_iter, memchr2, memrchr};
use std::{io, path::Path};

pub struct Cedict {
    file: Mapped,
    index: KeyIndex,
}

/// A key starts at an offset and runs until the next space (or end of line)
fn key_at(data: &[u8], off: u32) -> &[u8] {
    let rest = &data[off as usize..];
    let end = memchr2(b' ', b'\n', rest).unwrap_or(rest.len());
    &rest[..end]
}

impl Cedict {
    pub fn open(path: &Path) -> io::Result<Self> {
        let file = Mapped::open(path)?;
        let index = Self::build_index(file.bytes());
        if index.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "no CC-CEDICT entries found",
            ));
        }
        file.settle();
        Ok(Self { file, index })
    }

    fn build_index(data: &[u8]) -> KeyIndex {
        // ~124k lines → reserve for two keys per line
        let mut offsets = Vec::with_capacity(data.len() / 30);
        let mut line_start = 0usize;
        let push_line = |start: usize, end: usize, offsets: &mut Vec<u32>| {
            let line = &data[start..end];
            if line.is_empty() || line[0] == b'#' || memchr(b'[', line).is_none() {
                return;
            }
            let Some(sp) = memchr(b' ', line) else { return };
            offsets.push(start as u32);
            let simp = start + sp + 1;
            if key_at(data, simp as u32) != &line[..sp] {
                offsets.push(simp as u32);
            }
        };
        for nl in memchr_iter(b'\n', data) {
            push_line(line_start, nl, &mut offsets);
            line_start = nl + 1;
        }
        if line_start < data.len() {
            push_line(line_start, data.len(), &mut offsets);
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
        let data = self.file.bytes();
        let hits = self.index.find(data, term.as_bytes(), key_at);
        let mut seen_lines: Vec<usize> = Vec::with_capacity(hits.len());
        let mut found: Vec<(bool, Candidate)> = Vec::with_capacity(hits.len());
        for &off in hits {
            let start = memrchr(b'\n', &data[..off as usize]).map_or(0, |p| p + 1);
            if seen_lines.contains(&start) {
                continue;
            }
            seen_lines.push(start);
            let end = memchr(b'\n', &data[start..]).map_or(data.len(), |p| start + p);
            if let Some(line) = std::str::from_utf8(&data[start..end])
                .ok()
                .and_then(parse_line)
            {
                let demoted = line.is_secondary();
                found.push((demoted, line.into_candidate(term)));
            }
        }
        // keep dictionary order inside each group
        found.sort_by_key(|(demoted, _)| *demoted);
        found
            .into_iter()
            .map(|(_, c)| c)
            .take(MAX_CANDIDATES)
            .collect()
    }
}

#[derive(Debug, PartialEq)]
pub struct Line<'a> {
    pub trad: &'a str,
    pub simp: &'a str,
    pub pinyin: &'a str,
    pub defs: Vec<&'a str>,
}

pub fn parse_line(line: &str) -> Option<Line<'_>> {
    let line = line.trim_end_matches('\r');
    let (trad, rest) = line.split_once(' ')?;
    let (simp, rest) = rest.split_once(" [")?;
    let (pinyin, rest) = rest.split_once(']')?;
    let defs = rest
        .trim()
        .trim_matches('/')
        .split('/')
        .map(str::trim)
        .filter(|d| !d.is_empty())
        .collect();
    Some(Line {
        trad,
        simp,
        pinyin: pinyin.trim(),
        defs,
    })
}

impl<'a> Line<'a> {
    /// Surname-only entries, proper nouns and pure variant references are rarely what a learner wants first
    fn is_secondary(&self) -> bool {
        let proper = self
            .pinyin
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_uppercase());
        let only_refs = !self.defs.is_empty()
            && self.defs.iter().all(|d| {
                d.starts_with("surname ")
                    || d.starts_with("variant of ")
                    || d.starts_with("old variant of ")
                    || d.starts_with("archaic variant of ")
                    || d.starts_with("Japanese variant of ")
                    || d.starts_with("see ")
                    || d.starts_with("used in ")
            });
        proper || only_refs
    }

    fn into_candidate(self, term: &str) -> Candidate {
        let other = if term == self.trad {
            self.simp
        } else {
            self.trad
        };
        Candidate {
            headword: term.to_string(),
            reading: self.pinyin.to_string(),
            variants: if other != term {
                vec![other.to_string()]
            } else {
                Vec::new()
            },
            senses: self
                .defs
                .into_iter()
                .map(|d| Sense {
                    tags: Vec::new(),
                    glosses: vec![d.to_string()],
                })
                .collect(),
            common: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn parses_lines() {
        let l = parse_line("學 学 [xue2] /to learn/to study/science/-ology/\r").unwrap();
        assert_eq!(l.trad, "學");
        assert_eq!(l.simp, "学");
        assert_eq!(l.pinyin, "xue2");
        assert_eq!(l.defs, vec!["to learn", "to study", "science", "-ology"]);
        assert!(parse_line("# CC-CEDICT").is_none());
    }

    #[test]
    fn looks_up_both_scripts_and_ranks() {
        let mut f = tempfile();
        writeln!(f.1, "# CC-CEDICT\n#! version=1").unwrap();
        writeln!(f.1, "學 学 [Xue2] /surname Xue/").unwrap();
        writeln!(f.1, "學 学 [xue2] /to learn/to study/").unwrap();
        writeln!(f.1, "學生 学生 [xue2 sheng5] /student/").unwrap();
        write!(f.1, "中 中 [zhong1] /middle/").unwrap(); // no trailing newline
        drop(f.1);
        let d = Cedict::open(&f.0).unwrap();
        assert_eq!(d.headwords(), 7);
        let r = d.lookup("学");
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].reading, "xue2", "common reading before the surname");
        assert_eq!(r[0].variants, vec!["學"]);
        assert_eq!(d.lookup("學生")[0].senses[0].glosses, vec!["student"]);
        assert_eq!(d.lookup("中")[0].variants, Vec::<String>::new());
        assert!(d.lookup("学s").is_empty());
        assert!(d.lookup("#").is_empty());
        assert!(d.contains("學生") && !d.contains("學生們"));
        std::fs::remove_file(&f.0).unwrap();
    }

    fn tempfile() -> (std::path::PathBuf, std::fs::File) {
        let p = std::env::temp_dir().join(format!("cedict-test-{}.u8", std::process::id()));
        let f = std::fs::File::create(&p).unwrap();
        (p, f)
    }
}
