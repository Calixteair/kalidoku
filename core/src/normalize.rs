use unicode_normalization::UnicodeNormalization;

/// Canonical-form normalisation used for autocomplete and answer matching.
///
/// Strips diacritics, lowercases, unifies apostrophes & punctuation, expands common
/// French abbreviations. Idempotent. Same function used at index time and query time.
#[must_use]
pub fn normalize(input: &str) -> String {
    let lowered: String = input
        .nfd()
        .filter(|c| !is_combining_mark(*c))
        .collect::<String>()
        .to_lowercase();

    let mut s = String::with_capacity(lowered.len());
    let mut prev_space = true;
    for c in lowered.chars() {
        let mapped = match c {
            '\u{2019}' | '\u{2018}' | '`' | '´' => '\'',
            '-' | '_' | '/' | '.' | ',' | ';' | ':' | '(' | ')' | '[' | ']' => ' ',
            other => other,
        };
        if mapped == ' ' {
            if !prev_space {
                s.push(' ');
                prev_space = true;
            }
        } else {
            s.push(mapped);
            prev_space = false;
        }
    }
    let trimmed = s.trim().to_string();

    // expand a small allowlist of abbreviations
    trimmed
        .split(' ')
        .map(|w| match w {
            "st" => "saint",
            "ste" => "sainte",
            "pl" => "place",
            "av" => "avenue",
            other => other,
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_combining_mark(c: char) -> bool {
    matches!(c, '\u{0300}'..='\u{036F}')
}

#[cfg(test)]
mod tests {
    use super::normalize;

    #[test]
    fn strips_accents_and_lowercases() {
        assert_eq!(normalize("Châtelet"), "chatelet");
        assert_eq!(normalize("RÉPUBLIQUE"), "republique");
    }

    #[test]
    fn unifies_apostrophes_and_punctuation() {
        assert_eq!(normalize("Saint-Michel"), "saint michel");
        assert_eq!(normalize("Père Lachaise"), "pere lachaise");
        assert_eq!(normalize("Charles-de-Gaulle--Étoile"), "charles de gaulle etoile");
    }

    #[test]
    fn expands_common_abbrevs() {
        assert_eq!(normalize("St-Michel"), "saint michel");
        assert_eq!(normalize("Pl. d'Italie"), "place d'italie");
    }

    #[test]
    fn idempotent() {
        let once = normalize("Saint-Lazare");
        assert_eq!(once, normalize(&once));
    }
}
