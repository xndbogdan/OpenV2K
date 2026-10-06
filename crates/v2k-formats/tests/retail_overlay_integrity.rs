//! Guard the on-disk EOF boundary consumed by retail's final section loader.
//! NoCD05 reached EOF ten bytes before Alpine's declared dependency-list end.

use v2k_formats::ovl::OvlFile;

#[v2k_test_support::retail_test]
fn retail_overlays_have_the_complete_declared_final_section() {
    let root = v2k_test_support::retail_dir().join("Overlay");
    for variant in 0..4 {
        for level in 0..53 {
            let path = root.join(format!("{variant}X{level}XX.OVL"));
            let bytes = std::fs::read(&path).unwrap_or_else(|error| {
                panic!("{}: {error}", path.display());
            });
            let overlay = OvlFile::parse(&bytes).unwrap();
            let last = overlay.section(14).unwrap();
            assert_eq!(
                last.data.len(),
                last.header_value as usize,
                "{}: retail 004AB3B0 reads the complete declared Section-14 byte count",
                path.display(),
            );
        }
    }
}
