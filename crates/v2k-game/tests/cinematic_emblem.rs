//! `FUN_004537F0`'s cinematic emblem: `FUN_0042D030` geometry for the
//! fallback frames Intro2 draws, and their pixels against a TTD recording of
//! retail Intro2.

use v2k_game::menu_billboard::{MenuBillboardLayout, MenuBillboardRect};
use v2k_game::model_color::sprite_flat_shade_row;
use v2k_game::session::GameSession;
use v2k_render::software::store::OwnedMaterial;
use v2k_render::software::{
    material_flags, ClipRect, FillSlot, NoWordImages, PixelFormat, SoftwareRaster, Surface565,
};

fn tier_session(tier: u32) -> GameSession {
    let mut session = GameSession::init(&v2k_test_support::retail_dir()).expect("PRELOAD");
    for level in [2, 3, 5] {
        session
            .load_auxiliary_ovl(level, tier)
            .expect("selected menu tier");
    }
    session
}

fn frame_size(session: &GameSession, gid: u16) -> [u16; 2] {
    let (_, entry) = session.cache.global_sprite(gid).expect("emblem frame");
    [entry.flags as u16, (entry.flags >> 16) as u16]
}

#[v2k_test_support::retail_test]
fn fallback_frames_size_the_cinematic_emblem() {
    let expected = [
        [0, 176, 51, 64],
        [0, 351, 101, 128],
        [0, 411, 126, 160],
        [0, 563, 162, 205],
    ];
    for (tier, [x, y, width, height]) in expected.into_iter().enumerate() {
        let session = tier_session(tier as u32);
        let layout = MenuBillboardLayout::from_cache(&session.cache).expect("billboard layout");
        for gid in 420..=425 {
            assert_eq!(
                layout.cinematic_rect(frame_size(&session, gid)),
                Some(MenuBillboardRect {
                    x,
                    y,
                    width: width as u32,
                    height: height as u32,
                }),
                "tier {tier} sprite {gid}"
            );
        }
    }
}

/// 64-bit FNV-1a over the little-endian surface words of a rectangle.
fn region_digest(
    surface: &Surface565,
    x: std::ops::Range<usize>,
    y: std::ops::Range<usize>,
) -> u64 {
    let mut digest = 0xCBF2_9CE4_8422_2325u64;
    for row in y {
        for &word in &surface.pixels[row * surface.width as usize..][x.clone()] {
            for byte in word.to_le_bytes() {
                digest = (digest ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01B3);
            }
        }
    }
    digest
}

/// The emblem as the game submits it: the selected frame decoded through
/// its flat palette row, rebuilt as an indexed additive material, stretched
/// over the cinematic rectangle onto `background`.
fn draw_emblem(session: &GameSession, gid: u16, background: u16) -> Surface565 {
    let layout = MenuBillboardLayout::from_cache(&session.cache).expect("billboard layout");
    let rect = layout
        .cinematic_rect(frame_size(session, gid))
        .expect("cinematic rectangle");
    let (atlas, entry) = session.cache.global_sprite(gid).expect("emblem frame");
    let row = sprite_flat_shade_row(entry.pal_size as u8);
    let frame = atlas
        .decode_sprite(entry, usize::from(row))
        .expect("decoded frame");
    let material = OwnedMaterial::indexed_from_rgba(
        &frame.rgba,
        u32::from(frame.width),
        u32::from(frame.height),
        material_flags::ADDITIVE,
    )
    .expect("indexed overlay material");
    let mut surface = Surface565::new(640, 480);
    surface.pixels.fill(background);
    let mut raster = SoftwareRaster::new(
        ClipRect {
            x0: 0,
            y0: 0,
            x1: 640,
            y1: 480,
        },
        PixelFormat::RGB565,
        0x001A_FDEC,
    );
    // Additive fillers read FUN_0047CA20's masks, which the world's fogged
    // primitives have set before any Intro2 emblem draws.
    raster.fog_mut().select(PixelFormat::RGB565, 1);
    let (left, top) = (rect.x as i16, rect.y as i16);
    let right = left + rect.width as i16;
    let bottom = top + rect.height as i16;
    let mut packet = [0u8; 0x14];
    for (index, (x, y)) in [(left, top), (right, top), (right, bottom), (left, bottom)]
        .into_iter()
        .enumerate()
    {
        packet[4 * index..4 * index + 2].copy_from_slice(&x.to_le_bytes());
        packet[4 * index + 2..4 * index + 4].copy_from_slice(&y.to_le_bytes());
    }
    raster.draw(
        FillSlot::TexturedQuad,
        &packet,
        &mut surface,
        &vec![material.view()],
        &NoWordImages,
    );
    surface
}

/// Digests of the recorded 640x480 frames' emblem pixels: tick 13 over
/// Klaus's black cover (the 60 columns his wing leaves clear) and tick
/// 4003 over the final card's uniform `0xB596`.
#[v2k_test_support::retail_test]
fn fallback_frames_reproduce_the_recorded_cinematic_emblem() {
    let session = tier_session(1);
    let cover = draw_emblem(&session, 425, 0);
    assert_eq!(
        region_digest(&cover, 0..60, 351..479),
        0xF387_2001_182D_A8CE
    );
    let card = draw_emblem(&session, 423, 0xB596);
    assert_eq!(
        region_digest(&card, 0..101, 351..479),
        0x1DFF_D68F_D247_926B
    );
}
