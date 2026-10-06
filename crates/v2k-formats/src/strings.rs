//! Section 2 (and Section 10) string table decoder.
//!
//! Section 2 contains null-terminated ASCII strings used for level text,
//! UI strings, and weapon/rank names. Strings are indexed SEQUENTIALLY by
//! NUL terminator, INCLUDING empty entries — the runtime's cumulative
//! global string pool (DAT_004FE628) counts every slot, so dropping
//! empties would shift all later ids (e.g. system level 2 has an empty
//! slot at index 37; "Exit" is id 66, not 65).

/// Extract null-terminated strings from section data, one entry per NUL
/// terminator (empty entries preserved). Authored horizontal tabs are retained
/// because gameplay text uses them before its `<...>` layout prefix;
/// unsupported non-ASCII/control bytes are replaced.
pub fn extract_strings(data: &[u8]) -> Vec<String> {
    let mut strings = Vec::new();
    let mut pos = 0usize;

    while pos < data.len() {
        let end = data[pos..]
            .iter()
            .position(|&b| b == 0)
            .map(|i| pos + i)
            .unwrap_or(data.len());
        let s: String = data[pos..end]
            .iter()
            .map(|&b| match b {
                b'\t' => '\t',
                32..=126 => b as char,
                _ => '\u{FFFD}',
            })
            .collect();
        strings.push(s);
        pos = end + 1;
    }

    strings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_basic() {
        let data = b"Hello\0World\0";
        let strings = extract_strings(data);
        assert_eq!(strings, vec!["Hello", "World"]);
    }

    #[test]
    fn empty_entries_keep_their_slots() {
        let data = b"A\0\0B\0";
        let strings = extract_strings(data);
        assert_eq!(strings, vec!["A", "", "B"]);
    }

    #[test]
    fn all_empty() {
        assert_eq!(extract_strings(b"\0\0\0"), vec!["", "", ""]);
    }

    #[test]
    fn unterminated_tail_is_an_entry() {
        assert_eq!(extract_strings(b"A\0tail"), vec!["A", "tail"]);
    }

    #[test]
    fn authored_layout_tabs_are_preserved() {
        assert_eq!(
            extract_strings(b"\t\t<*,3000, 4,30,10>%s\0"),
            vec!["\t\t<*,3000, 4,30,10>%s"]
        );
    }
}
