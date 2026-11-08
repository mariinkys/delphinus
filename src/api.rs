//! Server functions called from the browser

use crate::model::{Lang, Lookup};
use leptos::prelude::*;
use leptos::server_fn::codec::Json;
use serde::{Deserialize, Serialize};

/// What this instance can do; rendered into the page at SSR time
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Status {
    pub zh: bool,
    pub ja: bool,
    pub ocr: bool,
}

#[server(endpoint = "status")]
pub async fn get_status() -> Result<Status, ServerFnError> {
    let dicts = crate::dict::Dictionaries::get();
    Ok(Status {
        zh: dicts.is_some_and(|d| d.available(Lang::Zh)),
        ja: dicts.is_some_and(|d| d.available(Lang::Ja)),
        ocr: crate::ocr::enabled(),
    })
}

/// Looks up every term. Terms are de-duplicated and capped by `split_terms`
/// on the client, and re-checked here
#[server(endpoint = "lookup", input = Json)]
pub async fn lookup_words(lang: Lang, terms: Vec<String>) -> Result<Vec<Lookup>, ServerFnError> {
    use crate::model::{MAX_TERM_CHARS, MAX_TERMS};
    let Some(dicts) = crate::dict::Dictionaries::get() else {
        return Err(ServerFnError::new("dictionaries are not loaded"));
    };
    if !dicts.available(lang) {
        return Err(ServerFnError::new(format!(
            "the {} dictionary is not installed on this server",
            lang.english_name()
        )));
    }
    let mut out: Vec<Lookup> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for term in terms
        .iter()
        .map(|t| t.trim())
        .filter(|t| !t.is_empty() && t.chars().count() <= MAX_TERM_CHARS)
    {
        if out.len() >= MAX_TERMS {
            break;
        }
        let found = dicts.lookup(lang, term);
        // A phrase or sentence (often from a scan) is split into its words
        if found.candidates.is_empty() && term.chars().count() > 2 {
            let parts = dicts.segment(lang, term);
            if parts.len() > 1 {
                for part in parts {
                    if out.len() < MAX_TERMS && seen.insert(part.clone()) {
                        out.push(dicts.lookup(lang, &part));
                    }
                }
                continue;
            }
        }
        if seen.insert(term.to_string()) {
            out.push(found);
        }
    }
    Ok(out)
}
