//! Types and pure functions shared by the server and the browser
//!
//! The server only returns *structured* dictionary data; turning it into the
//! text of a flashcard happens on the client, so changing a formatting option
//! is instant and never costs a round-trip

use serde::{Deserialize, Serialize};

/// Hard limits that keep a single request cheap
pub const MAX_TERMS: usize = 500;
pub const MAX_TERM_CHARS: usize = 48;
pub const MAX_CANDIDATES: usize = 6;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Lang {
    #[default]
    Zh,
    Ja,
}

impl Lang {
    pub fn code(self) -> &'static str {
        match self {
            Lang::Zh => "zh",
            Lang::Ja => "ja",
        }
    }
    pub fn from_code(s: &str) -> Option<Self> {
        match s {
            "zh" => Some(Lang::Zh),
            "ja" => Some(Lang::Ja),
            _ => None,
        }
    }
    pub fn native_name(self) -> &'static str {
        match self {
            Lang::Zh => "中文",
            Lang::Ja => "日本語",
        }
    }
    pub fn english_name(self) -> &'static str {
        match self {
            Lang::Zh => "Chinese",
            Lang::Ja => "Japanese",
        }
    }
}

/// One meaning of a word.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sense {
    /// Short grammatical tags (JMdict part-of-speech codes such as `n`, `v5k`)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    pub glosses: Vec<String>,
}

/// A dictionary entry that matched a searched term
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Candidate {
    /// The written form that matched (normally identical to the query)
    pub headword: String,
    /// Kana (Japanese) or numbered pinyin (Chinese). Empty when the headword
    /// already *is* the reading (kana-only words)
    pub reading: String,
    /// Other ways of writing it (traditional/simplified, other kanji forms…)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variants: Vec<String>,
    pub senses: Vec<Sense>,
    /// Marked as a common word by the dictionary
    #[serde(default)]
    pub common: bool,
}

/// Every candidate found for one searched term (empty = not found)
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lookup {
    pub term: String,
    pub candidates: Vec<Candidate>,
}

// Input parsing

/// Splits free-form input into unique terms. Any mix of separators works:
/// commas (`,` `，` `、`), semicolons, whitespace and new lines
pub fn split_terms(input: &str) -> Vec<String> {
    let is_sep = |c: char| {
        c.is_whitespace() || matches!(c, ',' | '，' | '、' | ';' | '；' | '|' | '｜' | '/' | '／')
    };
    let mut out: Vec<String> = Vec::new();
    for raw in input.split(is_sep) {
        let t = raw.trim_matches(|c: char| {
            matches!(
                c,
                '。' | '.'
                    | '!'
                    | '！'
                    | '?'
                    | '？'
                    | '"'
                    | '“'
                    | '”'
                    | '「'
                    | '」'
                    | '『'
                    | '』'
                    | '('
                    | ')'
                    | '（'
                    | '）'
            )
        });
        if t.is_empty() || t.chars().count() > MAX_TERM_CHARS {
            continue;
        }
        if !out.iter().any(|e| e == t) {
            out.push(t.to_string());
        }
        if out.len() == MAX_TERMS {
            break;
        }
    }
    out
}

/// Makes every character exactly one grid cell wide: ASCII becomes its
/// full-width form (`,` → `，`, `A` → `Ａ`), spaces become ideographic spaces
/// and half-width katakana becomes full-width (`ｶﾞ` → `ガ`). New lines are kept
pub fn to_full_width(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            ' ' => out.push('\u{3000}'),
            '!'..='~' => out.push(char::from_u32(c as u32 + 0xFEE0).unwrap_or(c)),
            '\u{FF61}'..='\u{FF9F}' => {
                let base = half_kana(c);
                // A following (han)dakuten combines into one character
                let combined = match chars.peek() {
                    Some('ﾞ') => char::from_u32(base as u32 + 1).filter(|_| takes_mark(base, 1)),
                    Some('ﾟ') => char::from_u32(base as u32 + 2).filter(|_| takes_mark(base, 2)),
                    _ => None,
                };
                match combined {
                    Some(k) => {
                        chars.next();
                        out.push(k);
                    }
                    None if base == 'ウ' && chars.peek() == Some(&'ﾞ') => {
                        chars.next();
                        out.push('ヴ');
                    }
                    None => out.push(base),
                }
            }
            _ => out.push(c),
        }
    }
    out
}

/// The reverse of the ASCII part of [`to_full_width`], used as a lookup
/// fallback because CC-CEDICT writes Latin letters half-width (`T恤`)
pub fn to_half_width(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '\u{3000}' => ' ',
            '\u{FF01}'..='\u{FF5E}' => char::from_u32(c as u32 - 0xFEE0).unwrap_or(c),
            _ => c,
        })
        .collect()
}

fn half_kana(c: char) -> char {
    const TABLE: &str = "。「」、・ヲァィゥェォャュョッーアイウエオカキクケコサシスセソタチツテトナニヌネノハヒフヘホマミムメモヤユヨラリルレロワン゛゜";
    TABLE.chars().nth((c as u32 - 0xFF61) as usize).unwrap_or(c)
}

/// Whether a katakana can take a dakuten (`mark` = 1) or handakuten (2)
fn takes_mark(k: char, mark: u32) -> bool {
    match mark {
        1 => matches!(
            k,
            'カ' | 'キ'
                | 'ク'
                | 'ケ'
                | 'コ'
                | 'サ'
                | 'シ'
                | 'ス'
                | 'セ'
                | 'ソ'
                | 'タ'
                | 'チ'
                | 'ツ'
                | 'テ'
                | 'ト'
                | 'ハ'
                | 'ヒ'
                | 'フ'
                | 'ヘ'
                | 'ホ'
        ),
        _ => matches!(k, 'ハ' | 'ヒ' | 'フ' | 'ヘ' | 'ホ'),
    }
}

// Pinyin

/// Converts CC-CEDICT numbered pinyin (`ni3 hao3`, `lu:4`) to tone marks
/// (`nǐ hǎo`, `lǜ`). Tokens that are not syllables are left untouched
pub fn pinyin_marks(numbered: &str) -> String {
    let mut out = String::with_capacity(numbered.len() + 8);
    for (i, tok) in numbered.split(' ').enumerate() {
        if i > 0 {
            out.push(' ');
        }
        out.push_str(&syllable_marks(tok));
    }
    out
}

fn syllable_marks(tok: &str) -> String {
    let Some(last) = tok.chars().last() else {
        return String::new();
    };
    let Some(tone) = last.to_digit(10).filter(|d| (1..=5).contains(d)) else {
        return tok.replace("u:", "ü").replace("U:", "Ü");
    };
    let body = tok[..tok.len() - 1]
        .replace("u:", "ü")
        .replace("U:", "Ü")
        .replace('v', "ü");
    if tone == 5
        || body.is_empty()
        || !body
            .chars()
            .any(|c| c.is_ascii_alphabetic() || c == 'ü' || c == 'Ü')
    {
        return body;
    }
    let chars: Vec<char> = body.chars().collect();
    let lower: Vec<char> = chars
        .iter()
        .map(|c| c.to_lowercase().next().unwrap_or(*c))
        .collect();
    let target = if let Some(i) = lower.iter().position(|&c| c == 'a' || c == 'e') {
        Some(i)
    } else if let Some(i) = lower.windows(2).position(|w| w == ['o', 'u']) {
        Some(i)
    } else {
        lower
            .iter()
            .rposition(|&c| matches!(c, 'a' | 'e' | 'i' | 'o' | 'u' | 'ü'))
    };
    let Some(t) = target else {
        return body;
    };
    chars
        .iter()
        .enumerate()
        .map(|(i, &c)| if i == t { mark(c, tone) } else { c })
        .collect()
}

fn mark(c: char, tone: u32) -> char {
    const TABLE: [(char, [char; 4]); 12] = [
        ('a', ['ā', 'á', 'ǎ', 'à']),
        ('e', ['ē', 'é', 'ě', 'è']),
        ('i', ['ī', 'í', 'ǐ', 'ì']),
        ('o', ['ō', 'ó', 'ǒ', 'ò']),
        ('u', ['ū', 'ú', 'ǔ', 'ù']),
        ('ü', ['ǖ', 'ǘ', 'ǚ', 'ǜ']),
        ('A', ['Ā', 'Á', 'Ǎ', 'À']),
        ('E', ['Ē', 'É', 'Ě', 'È']),
        ('I', ['Ī', 'Í', 'Ǐ', 'Ì']),
        ('O', ['Ō', 'Ó', 'Ǒ', 'Ò']),
        ('U', ['Ū', 'Ú', 'Ǔ', 'Ù']),
        ('Ü', ['Ǖ', 'Ǘ', 'Ǚ', 'Ǜ']),
    ];
    TABLE
        .iter()
        .find(|(base, _)| *base == c)
        .map(|(_, marks)| marks[(tone - 1) as usize])
        .unwrap_or(c)
}

// ---------------------------------------------------------------------------
// Card formatting
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReadingSide {
    Back,
    Front,
    Hidden,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardFormat {
    pub reading: ReadingSide,
    pub tone_marks: bool,
    /// 0 = all senses
    pub max_senses: u8,
    pub show_tags: bool,
    pub show_variants: bool,
}

impl Default for CardFormat {
    fn default() -> Self {
        Self {
            reading: ReadingSide::Back,
            tone_marks: true,
            max_senses: 3,
            show_tags: false,
            show_variants: true,
        }
    }
}

pub fn display_reading(c: &Candidate, lang: Lang, fmt: &CardFormat) -> String {
    match lang {
        Lang::Zh if fmt.tone_marks => pinyin_marks(&c.reading),
        _ => c.reading.clone(),
    }
}

pub fn card_front(c: &Candidate, lang: Lang, fmt: &CardFormat) -> String {
    let reading = display_reading(c, lang, fmt);
    if fmt.reading == ReadingSide::Front && !reading.is_empty() {
        format!("{} [{}]", c.headword, reading)
    } else {
        c.headword.clone()
    }
}

/// Renders the meanings as a single line (every export target is line based)
pub fn senses_line(c: &Candidate, fmt: &CardFormat) -> String {
    let take = if fmt.max_senses == 0 {
        usize::MAX
    } else {
        fmt.max_senses as usize
    };
    let senses: Vec<&Sense> = c.senses.iter().take(take).collect();
    let numbered = senses.len() > 1;
    let mut out = String::new();
    for (i, s) in senses.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        if numbered {
            out.push_str(&format!("{}. ", i + 1));
        }
        if fmt.show_tags && !s.tags.is_empty() {
            out.push_str(&format!("({}) ", s.tags.join(", ")));
        }
        out.push_str(&s.glosses.join("; "));
    }
    out
}

pub fn card_back(c: &Candidate, lang: Lang, fmt: &CardFormat) -> String {
    let mut head: Vec<String> = Vec::new();
    if fmt.reading == ReadingSide::Back {
        let r = display_reading(c, lang, fmt);
        if !r.is_empty() {
            head.push(r);
        }
    }
    if fmt.show_variants && !c.variants.is_empty() {
        head.push(format!("({})", c.variants.join("、")));
    }
    let meanings = senses_line(c, fmt);
    match (head.is_empty(), meanings.is_empty()) {
        (true, _) => meanings,
        (false, true) => head.join(" "),
        (false, false) => format!("{} — {}", head.join(" "), meanings),
    }
}

// Export formats

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportTarget {
    Anki,
    Quizlet,
    Vaia,
    Csv,
}

impl ExportTarget {
    pub const ALL: [ExportTarget; 4] = [Self::Anki, Self::Quizlet, Self::Vaia, Self::Csv];
    pub fn name(self) -> &'static str {
        match self {
            Self::Anki => "Anki",
            Self::Quizlet => "Quizlet",
            Self::Vaia => "Vaia",
            Self::Csv => "CSV",
        }
    }
    /// Downloaded as a file (true) or copied to the clipboard (false)
    pub fn is_file(self) -> bool {
        matches!(self, Self::Anki | Self::Csv)
    }
    pub fn file_name(self) -> &'static str {
        match self {
            Self::Csv => "delphinus.csv",
            _ => "delphinus_anki.txt",
        }
    }
    pub fn mime(self) -> &'static str {
        match self {
            Self::Csv => "text/csv;charset=utf-8",
            _ => "text/plain;charset=utf-8",
        }
    }
}

fn one_line(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c == '\t' || c == '\n' || c == '\r' {
                ' '
            } else {
                c
            }
        })
        .collect::<String>()
        .trim()
        .to_string()
}

pub fn export(target: ExportTarget, cards: &[(String, String)]) -> String {
    match target {
        ExportTarget::Anki => {
            // https://docs.ankiweb.net/importing/text-files.html#file-headers
            let mut s = String::from("#separator:tab\n#html:false\n#columns:Front\tBack\n");
            for (f, b) in cards {
                s.push_str(&one_line(f));
                s.push('\t');
                s.push_str(&one_line(b));
                s.push('\n');
            }
            s
        }
        ExportTarget::Quizlet => cards
            .iter()
            .map(|(f, b)| format!("{}\t{}", one_line(f), one_line(b)))
            .collect::<Vec<_>>()
            .join("\n"),
        ExportTarget::Vaia => cards
            .iter()
            .map(|(f, b)| format!("{}/#*#/{}", one_line(f), one_line(b)))
            .collect::<Vec<_>>()
            .join("\n\\#\n"),
        ExportTarget::Csv => {
            let q = |s: &str| format!("\"{}\"", s.replace('"', "\"\""));
            let mut s = String::from("\u{feff}front,back\r\n");
            for (f, b) in cards {
                s.push_str(&q(f));
                s.push(',');
                s.push_str(&q(b));
                s.push_str("\r\n");
            }
            s
        }
    }
}

/// Separators chosen by the person for the custom export, stored as typed:
/// `\t` and `\n` are written as two characters and expanded on export
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomFormat {
    /// Between the front and the back of a card
    pub between_sides: String,
    /// Between one card and the next
    pub between_cards: String,
}

impl Default for CustomFormat {
    fn default() -> Self {
        Self {
            between_sides: "\\t".into(),
            between_cards: "\\n".into(),
        }
    }
}

/// Expands `\t`, `\n`, `\r` and `\\` typed in a text box
pub fn unescape_separator(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('t') => out.push('\t'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('\\') => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

/// Joins the cards with the person's own separators. A separator that also
/// appears inside a card is replaced by a space there, so the output always
/// splits back into the same cards
pub fn export_custom(cards: &[(String, String)], fmt: &CustomFormat) -> String {
    let sides = unescape_separator(&fmt.between_sides);
    let between = unescape_separator(&fmt.between_cards);
    let clean = |s: &str| {
        let mut t = one_line(s);
        for sep in [&sides, &between] {
            if !sep.is_empty() && t.contains(sep.as_str()) {
                t = t.replace(sep.as_str(), " ");
            }
        }
        t
    };
    cards
        .iter()
        .map(|(f, b)| format!("{}{sides}{}", clean(f), clean(b)))
        .collect::<Vec<_>>()
        .join(&between)
}

/// Friendly names for the most frequent JMdict part-of-speech codes
pub fn tag_label(tag: &str) -> &str {
    match tag {
        "n" => "noun",
        "adj-i" => "i-adjective",
        "adj-na" => "na-adjective",
        "adj-no" => "no-adjective",
        "adj-pn" => "pre-noun adjectival",
        "adv" => "adverb",
        "adv-to" => "to-adverb",
        "aux-v" => "auxiliary verb",
        "conj" => "conjunction",
        "ctr" => "counter",
        "exp" => "expression",
        "int" => "interjection",
        "pn" => "pronoun",
        "pref" => "prefix",
        "suf" => "suffix",
        "prt" => "particle",
        "v1" => "ichidan verb",
        "vs" => "suru verb",
        "vs-i" => "suru verb",
        "vt" => "transitive",
        "vi" => "intransitive",
        t if t.starts_with("v5") => "godan verb",
        t => t,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_any_separator() {
        assert_eq!(
            split_terms("你好，中文,学习、 図書館\n気温;  空"),
            vec!["你好", "中文", "学习", "図書館", "気温", "空"]
        );
        assert_eq!(split_terms("  "), Vec::<String>::new());
        assert_eq!(split_terms("a, a ,a"), vec!["a"]);
        assert_eq!(split_terms("「本棚」。"), vec!["本棚"]);
    }

    #[test]
    fn widths() {
        assert_eq!(
            to_full_width("図書館,本棚 CD-ROM;空\nT恤"),
            "図書館，本棚　ＣＤ－ＲＯＭ；空\nＴ恤"
        );
        assert_eq!(
            to_full_width("ｶﾀｶﾅ ｶﾞｲﾄﾞ ﾊﾟﾝ ｳﾞｧ ｱﾞ"),
            "カタカナ　ガイド　パン　ヴァ　ア゛"
        );
        assert_eq!(to_half_width("ＣＤ－ＲＯＭ　Ｔ恤"), "CD-ROM T恤");
        assert_eq!(to_full_width("学，中文"), "学，中文");
        assert_eq!(
            split_terms(&to_full_width("本棚,写真 空;ＣＤ")),
            vec!["本棚", "写真", "空", "ＣＤ"]
        );
    }

    #[test]
    fn tone_marks() {
        assert_eq!(pinyin_marks("ni3 hao3"), "nǐ hǎo");
        assert_eq!(pinyin_marks("lu:4 se4"), "lǜ sè");
        assert_eq!(pinyin_marks("xiu1 xi5"), "xiū xi");
        assert_eq!(pinyin_marks("dou1"), "dōu");
        assert_eq!(pinyin_marks("gui4"), "guì");
        assert_eq!(pinyin_marks("liu2"), "liú");
        assert_eq!(pinyin_marks("Bei3 jing1"), "Běi jīng");
        assert_eq!(pinyin_marks("er4 shi2 yi1 san1 C"), "èr shí yī sān C");
        assert_eq!(pinyin_marks("r5"), "r");
        assert_eq!(pinyin_marks("A1 Q"), "Ā Q");
        assert_eq!(pinyin_marks("nu:3 er2"), "nǚ ér");
        assert_eq!(pinyin_marks("xx5"), "xx");
    }

    fn cand() -> Candidate {
        Candidate {
            headword: "学".into(),
            reading: "xue2".into(),
            variants: vec!["學".into()],
            senses: vec![
                Sense {
                    tags: vec![],
                    glosses: vec!["to learn".into(), "to study".into()],
                },
                Sense {
                    tags: vec![],
                    glosses: vec!["science".into()],
                },
            ],
            common: true,
        }
    }

    #[test]
    fn formats_cards() {
        let f = CardFormat::default();
        assert_eq!(card_front(&cand(), Lang::Zh, &f), "学");
        assert_eq!(
            card_back(&cand(), Lang::Zh, &f),
            "xué (學) — 1. to learn; to study 2. science"
        );
        let f2 = CardFormat {
            reading: ReadingSide::Front,
            tone_marks: false,
            max_senses: 1,
            show_variants: false,
            ..f
        };
        assert_eq!(card_front(&cand(), Lang::Zh, &f2), "学 [xue2]");
        assert_eq!(card_back(&cand(), Lang::Zh, &f2), "to learn; to study");
    }

    #[test]
    fn custom_export() {
        let cards = vec![
            ("学".to_string(), "to learn; to study".to_string()),
            ("中文".to_string(), "Chinese".to_string()),
        ];
        let tsv = CustomFormat::default();
        assert_eq!(
            export_custom(&cards, &tsv),
            "学\tto learn; to study\n中文\tChinese"
        );
        let own = CustomFormat {
            between_sides: " = ".into(),
            between_cards: ";".into(),
        };
        assert_eq!(
            export_custom(&cards, &own),
            "学 = to learn  to study;中文 = Chinese",
            "separator inside a card is replaced"
        );
        assert_eq!(unescape_separator("a\\tb\\\\n\\x\\"), "a\tb\\n\\x\\");
    }

    #[test]
    fn exports() {
        let cards = vec![
            ("学".to_string(), "a\tb".to_string()),
            ("x\"".to_string(), "y".to_string()),
        ];
        assert_eq!(export(ExportTarget::Quizlet, &cards), "学\ta b\nx\"\ty");
        assert_eq!(
            export(ExportTarget::Vaia, &cards),
            "学/#*#/a b\n\\#\nx\"/#*#/y"
        );
        assert!(export(ExportTarget::Anki, &cards).starts_with("#separator:tab\n#html:false\n"));
        assert!(export(ExportTarget::Csv, &cards).contains("\"x\"\"\",\"y\""));
    }
}
