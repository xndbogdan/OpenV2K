//! Real-data checks for the display-tier boundary between world descriptors
//! and their selected world-style resource packs.

use std::path::{Path, PathBuf};

use v2k_formats::ovl::{OvlFile, NUM_SECTIONS};
use v2k_formats::sections;
use v2k_game::loader;
use v2k_game::session::GameSession;

const LOW_TEXTURE_BUFFER_SIZE: u32 = 0x001A_E000;
const HIGH_TEXTURE_BUFFER_SIZE: u32 = 0x0041_E800;

fn world_path(root: &Path, variant: u32, level: u32) -> PathBuf {
    root.join("Overlay")
        .join(format!("{variant}X{level}XX.OVL"))
}

fn parse_world(path: &Path) -> OvlFile {
    let bytes = std::fs::read(path).unwrap_or_else(|error| {
        panic!("failed to read retail fixture {}: {error}", path.display())
    });
    OvlFile::parse(&bytes)
        .unwrap_or_else(|error| panic!("failed to parse {}: {error}", path.display()))
}

#[v2k_test_support::retail_test]
fn ordinary_world_payloads_are_gameplay_identical_across_low_and_high_tiers() {
    let root = v2k_test_support::retail_dir();

    for level in 13..=50 {
        let low_path = world_path(&root, 0, level);
        let high_path = world_path(&root, 1, level);
        let low = parse_world(&low_path);
        let high = parse_world(&high_path);

        assert_eq!(low.sections.len(), NUM_SECTIONS, "level {level} low tier");
        assert_eq!(high.sections.len(), NUM_SECTIONS, "level {level} high tier");

        for section_index in 0..NUM_SECTIONS {
            let low_section = low.section(section_index).unwrap();
            let high_section = high.section(section_index).unwrap();

            if section_index == sections::idx::SPRITE_ATLAS {
                // Empty world-local sprite sections still carry the display
                // tier's texture-buffer allocation hint. The actual visual
                // sprites come from the selected world-style system pack.
                assert_eq!(low_section.header_value, LOW_TEXTURE_BUFFER_SIZE);
                assert_eq!(high_section.header_value, HIGH_TEXTURE_BUFFER_SIZE);
            } else {
                assert_eq!(
                    low_section.header_value, high_section.header_value,
                    "level {level} Section {section_index} header"
                );
            }

            if section_index == sections::idx::LEVEL_DESCRIPTORS {
                // The first 64 bytes are a null-terminated editor name buffer.
                // Bytes after its first NUL contain non-runtime build residue
                // in a subset of tiers; every gameplay-bearing byte follows it.
                assert!(low_section.data.len() >= 64);
                assert!(high_section.data.len() >= 64);
                assert_eq!(
                    &low_section.data[64..],
                    &high_section.data[64..],
                    "level {level} Section 13 gameplay payload"
                );
            } else {
                assert_eq!(
                    low_section.data, high_section.data,
                    "level {level} Section {section_index} payload"
                );
            }
        }

        let low_state = loader::load_ovl(&low, low_path.to_string_lossy().as_ref()).unwrap();
        let high_state = loader::load_ovl(&high, high_path.to_string_lossy().as_ref()).unwrap();
        assert_eq!(
            low_state.level.as_ref().unwrap().name,
            high_state.level.as_ref().unwrap().name,
            "level {level} parsed name"
        );
    }
}

#[v2k_test_support::retail_test]
fn selected_world_tier_loads_the_matching_visual_resource_pack() {
    let root = v2k_test_support::retail_dir();

    let mut low = GameSession::init(&root).expect("low-tier session");
    low.load_level_by_id(13, 0).expect("0X13XX world");
    assert!(low
        .cache
        .level()
        .unwrap()
        .source_path
        .ends_with("0X13XX.OVL"));
    let low_atlas = low.cache.sprites().expect("0X6XX world-style sprites");
    assert_eq!((low_atlas.width, low_atlas.height), (2048, 860));

    let mut high = GameSession::init(&root).expect("high-tier session");
    high.load_level_by_id(13, 1).expect("1X13XX world");
    assert!(high
        .cache
        .level()
        .unwrap()
        .source_path
        .ends_with("1X13XX.OVL"));
    let high_atlas = high.cache.sprites().expect("1X6XX world-style sprites");
    assert_eq!((high_atlas.width, high_atlas.height), (2048, 2109));
}
