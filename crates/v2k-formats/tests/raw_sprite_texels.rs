use v2k_formats::{
    ovl::OvlFile,
    palette, sections,
    sprites::{parse_section3, SpriteAtlas, SpriteTexelFormat, ATLAS_STRIDE},
};

fn raw_atlas(render_flags: u16, x: u16, y: u16, rows: &[&[u16]]) -> SpriteAtlas {
    let width = rows[0].len() as u16;
    assert!(rows.iter().all(|row| row.len() == usize::from(width)));
    let height = rows.len() as u16;
    let mut metadata = vec![0u8; 28];
    metadata[4..6].copy_from_slice(&render_flags.to_le_bytes());
    // Deliberately unusable palette metadata: raw words must not consult it.
    metadata[6..8].copy_from_slice(&247u16.to_le_bytes());
    metadata[8..12]
        .copy_from_slice(&((u32::from(y) * ATLAS_STRIDE as u32) + u32::from(x)).to_le_bytes());
    metadata[12..16].copy_from_slice(&u32::MAX.to_le_bytes());
    metadata[16..18].copy_from_slice(&width.to_le_bytes());
    metadata[18..20].copy_from_slice(&height.to_le_bytes());
    let mut section = Vec::new();
    section.extend_from_slice(&(metadata.len() as u32).to_le_bytes());
    section.extend_from_slice(&metadata);
    for coordinate in [x, y, x + width * 2, y + height] {
        section.extend_from_slice(&coordinate.to_le_bytes());
    }
    for row in rows {
        for word in *row {
            section.extend_from_slice(&word.to_le_bytes());
        }
    }
    section.extend_from_slice(&0xffffu16.to_le_bytes());
    parse_section3((usize::from(y + height) * ATLAS_STRIDE) as u32, &section).unwrap()
}

#[test]
fn raw_rgb555_uses_little_endian_words_and_ignores_palette_and_shade() {
    let atlas = raw_atlas(0x02, 0, 0, &[&[0x7c00, 0x03e0, 0x001f, 0xffff]]);
    let entry = &atlas.entries[0];
    assert_eq!(entry.texel_format(), SpriteTexelFormat::Rgb555);
    assert_eq!(atlas.dimensions(entry).unwrap(), (4, 1));
    let expected = [
        248, 0, 0, 255, 0, 248, 0, 255, 0, 0, 248, 255, 248, 248, 248, 255,
    ];
    for shade in [0, 28, palette::BRIGHTEST_SHADE, usize::MAX] {
        let decoded = atlas.decode_sprite(entry, shade).unwrap();
        assert_eq!((decoded.width, decoded.height), (4, 1));
        assert_eq!(decoded.rgba, expected);
    }
    assert_eq!(atlas.decode_all_strict(28).unwrap().len(), 1);
    assert!(atlas
        .decode_indices(entry)
        .unwrap_err()
        .to_string()
        .contains("no palette indices"));
    assert!(atlas
        .palette_row(entry, 0)
        .unwrap_err()
        .to_string()
        .contains("no palette row"));
}

#[test]
fn raw_zero_key_is_applied_after_ignoring_bit_fifteen_and_opaque_override_keeps_black() {
    let atlas = raw_atlas(0x03, 0, 0, &[&[0, 0x8000, 0x8001, 0x7c00]]);
    let entry = &atlas.entries[0];
    assert!(entry.is_zero_keyed());
    assert_eq!(
        atlas.decode_sprite(entry, 28).unwrap().rgba,
        [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 255, 248, 0, 0, 255]
    );
    let opaque = atlas.decode_sprite_opaque(entry, 28).unwrap();
    assert_eq!(&opaque.rgba[..8], &[0, 0, 0, 255, 0, 0, 0, 255]);
    let unkeyed = raw_atlas(0x02, 0, 0, &[&[0, 0x8000, 0x8001, 0x7c00]]);
    assert_eq!(
        unkeyed.decode_sprite(&unkeyed.entries[0], 28).unwrap().rgba,
        opaque.rgba
    );
}

#[test]
fn raw_rectangles_keep_byte_row_stride_allow_odd_addresses_and_halve_only_width() {
    let atlas = raw_atlas(0x06, 17, 2, &[&[0x7c00, 0x03e0], &[0x001f, 0x7fff]]);
    let entry = &atlas.entries[0];
    assert_eq!(entry.tex_offset as usize, 2 * ATLAS_STRIDE + 17);
    assert_eq!((atlas.rects[0].width, atlas.rects[0].height), (4, 2));
    assert_eq!(atlas.dimensions(entry).unwrap(), (2, 2));
    assert_eq!(
        atlas.decode_sprite(entry, 0).unwrap().rgba,
        [248, 0, 0, 255, 0, 248, 0, 255, 0, 0, 248, 255, 248, 248, 248, 255]
    );
}

#[test]
fn invalid_raw_word_widths_and_atlas_ranges_fail_for_dimensions_and_decode() {
    for fault in [
        "odd_width",
        "empty_width",
        "empty_height",
        "row_overflow",
        "buffer_end",
    ] {
        let mut atlas = raw_atlas(0x02, 0, 0, &[&[0x7c00, 0x03e0], &[0x001f, 0x7fff]]);
        match fault {
            "odd_width" => atlas.rects[0].width = 3,
            "empty_width" => atlas.rects[0].width = 0,
            "empty_height" => atlas.rects[0].height = 0,
            "row_overflow" => {
                atlas.rects[0].x_start = (ATLAS_STRIDE - 2) as u16;
                atlas.entries[0].tex_offset = (ATLAS_STRIDE - 2) as u32;
            }
            "buffer_end" => atlas.atlas.truncate(ATLAS_STRIDE + 3),
            _ => unreachable!(),
        }
        let entry = &atlas.entries[0];
        assert!(atlas.dimensions(entry).is_err(), "{fault}");
        assert!(atlas.decode_sprite(entry, 28).is_err(), "{fault}");
        assert!(atlas.decode_sprite_opaque(entry, 28).is_err(), "{fault}");
        assert!(atlas.decode_all_strict(28).is_err(), "{fault}");
    }
}

#[test]
fn indexed_accessors_keep_their_original_byte_dimensions_and_palette_semantics() {
    let mut atlas = raw_atlas(0x00, 0, 0, &[&[0x0100]]);
    let entry = &mut atlas.entries[0];
    entry.shade_count = 2;
    entry.pal_offset = 0;
    assert_eq!(entry.texel_format(), SpriteTexelFormat::Indexed8);
    let entry = &atlas.entries[0];
    assert_eq!(atlas.dimensions(entry).unwrap(), (2, 1));
    let indexed = atlas.decode_indices(entry).unwrap();
    assert_eq!((indexed.width, indexed.height), (2, 1));
    assert_eq!(indexed.indices, [0, 1]);
    assert_eq!(atlas.palette_row(entry, 28).unwrap()[1], [17, 17, 17, 255]);
    assert_eq!(
        atlas.decode_sprite(entry, 28).unwrap().rgba,
        [0, 0, 0, 255, 17, 17, 17, 255]
    );
}

fn retail_atlas(name: &str) -> SpriteAtlas {
    let path = v2k_test_support::retail_dir().join("Overlay").join(name);
    let ovl = OvlFile::parse(&std::fs::read(path).unwrap()).unwrap();
    sections::parse_sprites(&ovl).unwrap()
}

#[v2k_test_support::retail_test]
fn normal_tier_raw_pool_entries_decode_at_authored_dimensions_without_palettes() {
    for (file, sprite_id, dimensions) in [("1X3XX.OVL", 677, (1, 1)), ("1X5XX.OVL", 1319, (4, 8))] {
        let atlas = retail_atlas(file);
        let entry = atlas
            .entries
            .iter()
            .find(|entry| entry.index == sprite_id)
            .unwrap();
        assert_eq!(entry.texel_format(), SpriteTexelFormat::Rgb555);
        assert_eq!(atlas.dimensions(entry).unwrap(), dimensions);
        let decoded = atlas.decode_sprite(entry, 28).unwrap();
        assert_eq!((decoded.width, decoded.height), dimensions);
        if sprite_id == 677 {
            assert_eq!(decoded.rgba, [0, 0, 0, 255]);
        } else {
            assert!(decoded
                .rgba
                .chunks_exact(4)
                .any(|pixel| pixel[..3] != [0, 0, 0]));
        }
        assert_eq!(atlas.decode_sprite(entry, 0).unwrap().rgba, decoded.rgba);
    }
}

#[v2k_test_support::retail_test]
fn normal_tier_raw_thumbnails_preserve_authored_opaque_black() {
    let atlas = retail_atlas("1X51XX.OVL");
    let raw_entries = atlas
        .entries
        .iter()
        .filter(|entry| entry.texel_format() == SpriteTexelFormat::Rgb555)
        .collect::<Vec<_>>();
    assert_eq!(raw_entries.len(), 30);
    assert_eq!(
        raw_entries
            .iter()
            .map(|entry| entry.index)
            .collect::<Vec<_>>(),
        (3733..=3762).collect::<Vec<_>>()
    );
    let mut black_pixels = 0;
    for entry in raw_entries {
        assert!(!entry.is_zero_keyed());
        assert_eq!(atlas.dimensions(entry).unwrap(), (64, 48));
        let decoded = atlas.decode_sprite(entry, 28).unwrap();
        assert_eq!(decoded.rgba.len(), 64 * 48 * 4);
        assert!(decoded.rgba.chunks_exact(4).all(|pixel| pixel[3] == 255));
        black_pixels += decoded
            .rgba
            .chunks_exact(4)
            .filter(|pixel| pixel[..3] == [0, 0, 0])
            .count();
    }
    assert!(black_pixels > 0);
}

#[v2k_test_support::retail_test]
fn explicit_low_tier_copyright_raw_sprite_keeps_its_key_and_pixel_width() {
    // The authored raw copyright variant exists only in the low-resolution tier.
    let atlas = retail_atlas("0X5XX.OVL");
    let entry = atlas
        .entries
        .iter()
        .find(|entry| entry.index == 1292)
        .unwrap();
    assert_eq!(entry.texel_format(), SpriteTexelFormat::Rgb555);
    assert!(entry.is_zero_keyed());
    assert_eq!(atlas.dimensions(entry).unwrap(), (155, 14));
    let decoded = atlas.decode_sprite(entry, 28).unwrap();
    assert!(decoded.rgba.chunks_exact(4).any(|pixel| pixel[3] == 0));
    assert!(decoded.rgba.chunks_exact(4).any(|pixel| pixel[3] == 255));
}
