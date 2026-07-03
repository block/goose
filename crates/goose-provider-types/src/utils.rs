use unicode_normalization::UnicodeNormalization;

fn is_in_unicode_tag_range(c: char) -> bool {
    matches!(c, '\u{E0000}'..='\u{E007F}')
}

pub fn sanitize_unicode_tags(text: &str) -> String {
    let normalized: String = text.nfc().collect();

    normalized
        .chars()
        .filter(|&c| !is_in_unicode_tag_range(c))
        .collect()
}
