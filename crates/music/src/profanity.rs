use crate::models::{Lyrics, LyricsLane, LyricsLine, LyricsWord};

const INSIDE: &[&str] = &[
    "fuck", "shit", "bitch", "cunt", "nigg", "whore", "bastard", "asshole", "mierda", "pendej",
    "carajo", "chinga", "culer", "jodid", "joder", "cabron", "cabrón",
];

const WHOLE: &[&str] = &[
    "ass", "hoe", "tit", "dick", "cock", "pussy", "slut", "puta", "puto", "verga", "coño",
];

const ENDINGS: &[&str] = &["", "s", "es", "ed", "er", "ing", "in"];

fn letter(c: char) -> bool {
    c.is_alphabetic() || c == '\'' || c == '\u{2019}'
}

fn whole(word: &str) -> bool {
    WHOLE.iter().any(|base| {
        let base: &str = base;
        ENDINGS.iter().any(|ending| {
            let ending: &str = ending;
            word.len() == base.len() + ending.len()
                && word.starts_with(base)
                && word.ends_with(ending)
        })
    })
}

/// The same text with every rude word cut down to its first letter and asterisks, so it
/// keeps its length and the word-by-word timing still lines up with it.
pub fn mask(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let lower: Vec<char> = chars
        .iter()
        .map(|c| c.to_lowercase().next().unwrap_or(*c))
        .collect();
    let mut hide = vec![false; chars.len()];

    let mut at = 0;
    while at < chars.len() {
        if !letter(chars[at]) {
            at += 1;
            continue;
        }
        let start = at;
        while at < chars.len() && letter(chars[at]) {
            at += 1;
        }
        let word: String = lower[start..at]
            .iter()
            .filter(|c| **c != '\'' && **c != '\u{2019}')
            .collect();
        if whole(&word) {
            for flag in &mut hide[start + 1..at] {
                *flag = true;
            }
        }
    }

    for stem in INSIDE {
        let stem: Vec<char> = stem.chars().collect();
        let mut at = 0;
        while at + stem.len() <= lower.len() {
            if lower[at..at + stem.len()] == stem[..] {
                for flag in &mut hide[at + 1..at + stem.len()] {
                    *flag = true;
                }
                at += stem.len();
            } else {
                at += 1;
            }
        }
    }

    chars
        .iter()
        .zip(hide)
        .map(|(c, hidden)| if hidden { '*' } else { *c })
        .collect()
}

fn words(words: &[LyricsWord]) -> Vec<LyricsWord> {
    words
        .iter()
        .map(|word| LyricsWord {
            text: mask(&word.text),
            ..word.clone()
        })
        .collect()
}

fn lane(lane: &LyricsLane) -> LyricsLane {
    LyricsLane {
        text: mask(&lane.text),
        words: lane.words.as_deref().map(words),
        ..lane.clone()
    }
}

fn line(line: &LyricsLine) -> LyricsLine {
    LyricsLine {
        text: mask(&line.text),
        words: line.words.as_deref().map(words),
        secondary: line.secondary.iter().map(lane).collect(),
        ..line.clone()
    }
}

pub fn masked(lyrics: &Lyrics) -> Lyrics {
    match lyrics {
        Lyrics::Plain { text, romanized } => Lyrics::Plain {
            text: mask(text),
            romanized: romanized.clone(),
        },
        Lyrics::Synced { lines } => Lyrics::Synced {
            lines: lines.iter().map(line).collect::<Vec<_>>().into(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::mask;

    #[test]
    fn rude_words_keep_their_length_and_first_letter() {
        assert_eq!(mask("What the fuck"), "What the f***");
        assert_eq!(mask("With no hoes"), "With no h***");
        assert_eq!(mask("Motherfucker"), "Motherf***er");
    }

    #[test]
    fn plain_words_are_left_alone() {
        assert_eq!(mask("Put on your shoes, class"), "Put on your shoes, class");
        assert_eq!(mask("Dickens and peacocks"), "Dickens and peacocks");
    }
}
