use leptos::prelude::*;

#[server(OCRImage, "/ocr")]
pub async fn ocr_image(bytes: Vec<u8>, separation_char: String) -> Result<String, ServerFnError> {
    use actix_web::web::Data;
    use leptos_actix::extract;
    use oar_ocr::prelude::*;
    use std::sync::Arc;

    let oarocr: Data<Arc<OAROCR>> = extract().await?;
    let image = oar_ocr::utils::image::load_image_from_memory(&bytes)?;
    let results = oarocr.predict(&[image])?;
    let result = &results[0];

    let mut clean_result = Vec::new();
    for region in &result.text_regions {
        if let (Some(text), Some(confidence)) = (&region.text, region.confidence) {
            let filtered = filter_cjk(text);

            if !filtered.is_empty() {
                // only those with more than 50% confidence
                //println!("Text: {} (confidence: {:.2})", filtered, confidence);
                if confidence > 0.5 {
                    clean_result.push(filtered);
                }
            }
        }
    }

    if clean_result.is_empty() {
        Ok(String::new())
    } else {
        Ok(clean_result.join(&format!("{} ", separation_char)))
    }
}

#[cfg(feature = "ssr")]
fn filter_cjk(text: &str) -> String {
    let mut inside_parens = false;

    text.chars()
        .filter(|&c| {
            match c {
                // Opening parens
                '(' | '（' => {
                    inside_parens = true;
                    false
                }
                // Closing parens
                ')' | '）' => {
                    inside_parens = false;
                    false
                }
                // Hanzi/Kanji: always keep
                '\u{4E00}'..='\u{9FFF}'
                | '\u{3400}'..='\u{4DBF}'
                | '\u{20000}'..='\u{2A6DF}'
                | '\u{2A700}'..='\u{2B73F}'
                | '\u{2B740}'..='\u{2B81F}'
                | '\u{2B820}'..='\u{2CEAF}'
                | '\u{F900}'..='\u{FAFF}'
                | '\u{2F800}'..='\u{2FA1F}' => true,

                // Hiragana / Katakana: only if not inside parentheses
                '\u{3040}'..='\u{309F}'
                | '\u{30A0}'..='\u{30FF}'
                | '\u{31F0}'..='\u{31FF}'
                | '\u{FF66}'..='\u{FF9F}' => !inside_parens,

                _ => false,
            }
        })
        .collect()
}
