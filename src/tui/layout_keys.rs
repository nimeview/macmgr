//! Map Russian (ЙЦУКЕН) key positions → Latin (QWERTY) so TUI hotkeys
//! work the same regardless of the active input source.

use crossterm::event::KeyCode;

/// Translate a character as if typed on the same physical key under QWERTY.
/// Unmapped characters (digits, Latin, Cyrillic not on the letter row, etc.) pass through.
pub fn ru_to_en_char(c: char) -> char {
    match c {
        // lower
        'й' => 'q',
        'ц' => 'w',
        'у' => 'e',
        'к' => 'r',
        'е' => 't',
        'н' => 'y',
        'г' => 'u',
        'ш' => 'i',
        'щ' => 'o',
        'з' => 'p',
        'х' => '[',
        'ъ' => ']',
        'ф' => 'a',
        'ы' => 's',
        'в' => 'd',
        'а' => 'f',
        'п' => 'g',
        'р' => 'h',
        'о' => 'j',
        'л' => 'k',
        'д' => 'l',
        'ж' => ';',
        'э' => '\'',
        'я' => 'z',
        'ч' => 'x',
        'с' => 'c',
        'м' => 'v',
        'и' => 'b',
        'т' => 'n',
        'ь' => 'm',
        'б' => ',',
        'ю' => '.',
        'ё' => '`',
        // upper
        'Й' => 'Q',
        'Ц' => 'W',
        'У' => 'E',
        'К' => 'R',
        'Е' => 'T',
        'Н' => 'Y',
        'Г' => 'U',
        'Ш' => 'I',
        'Щ' => 'O',
        'З' => 'P',
        'Х' => '{',
        'Ъ' => '}',
        'Ф' => 'A',
        'Ы' => 'S',
        'В' => 'D',
        'А' => 'F',
        'П' => 'G',
        'Р' => 'H',
        'О' => 'J',
        'Л' => 'K',
        'Д' => 'L',
        'Ж' => ':',
        'Э' => '"',
        'Я' => 'Z',
        'Ч' => 'X',
        'С' => 'C',
        'М' => 'V',
        'И' => 'B',
        'Т' => 'N',
        'Ь' => 'M',
        'Б' => '<',
        'Ю' => '>',
        'Ё' => '~',
        _ => c,
    }
}

/// Normalize a keycode so RU-layout letters match EN hotkey bindings.
pub fn normalize_keycode(code: KeyCode) -> KeyCode {
    match code {
        KeyCode::Char(c) => KeyCode::Char(ru_to_en_char(c)),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyCode;

    #[test]
    fn maps_navigation_and_quit() {
        assert_eq!(ru_to_en_char('о'), 'j');
        assert_eq!(ru_to_en_char('л'), 'k');
        assert_eq!(ru_to_en_char('р'), 'h');
        assert_eq!(ru_to_en_char('д'), 'l');
        assert_eq!(ru_to_en_char('й'), 'q');
        assert_eq!(ru_to_en_char('Ж'), ':');
    }

    #[test]
    fn leaves_latin_and_digits() {
        assert_eq!(ru_to_en_char('q'), 'q');
        assert_eq!(ru_to_en_char('5'), '5');
        assert_eq!(ru_to_en_char('/'), '/');
    }

    #[test]
    fn normalize_keycode_char() {
        assert_eq!(normalize_keycode(KeyCode::Char('с')), KeyCode::Char('c'));
        assert_eq!(normalize_keycode(KeyCode::Esc), KeyCode::Esc);
    }
}
