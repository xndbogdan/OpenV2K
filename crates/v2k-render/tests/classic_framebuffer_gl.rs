//! Explicit desktop-GL checks: cargo test -p v2k-render --test
//! classic_framebuffer_gl -- --ignored --test-threads=1

use v2k_render::{FrameCaptureSource, GlRenderer, RenderScene, Renderer, ScalingMode};

#[test]
#[ignore = "requires a desktop OpenGL context"]
fn quantization_precedes_enlargement_and_classic_readback_survives_resize() {
    let sdl = sdl2::init().unwrap();
    let video = sdl.video().unwrap();
    let attributes = video.gl_attr();
    attributes.set_context_profile(sdl2::video::GLProfile::Compatibility);
    attributes.set_context_version(2, 1);
    attributes.set_double_buffer(true);
    let window = video
        .window("Classic framebuffer verification", 8, 6)
        .hidden()
        .opengl()
        .build()
        .unwrap();
    let mut renderer = GlRenderer::new(window, 8, 6).unwrap();
    renderer.set_scaling_mode(ScalingMode::Stretched, 4, 3);
    assert!(renderer.set_classic_framebuffer(true));
    renderer.begin_scene(RenderScene::Menu);
    renderer.clear(0.0, 0.0, 0.0);
    let row = [7, 7, 7, 255, 9, 9, 9, 255, 7, 7, 7, 255, 9, 9, 9, 255];
    renderer.draw_fullscreen(&row.repeat(3), 4, 3);
    renderer.present();
    let logical = renderer
        .capture_frame(FrameCaptureSource::Presented)
        .unwrap();
    assert_eq!((logical.width, logical.height), (4, 3));
    assert_eq!(&logical.rgba[..8], &[0, 4, 0, 255, 8, 8, 8, 255]);

    // Read the actual presented output through the ordinary FRONT path, so a
    // CPU-only quantized readback cannot conceal a broken presentation shader.
    assert!(!renderer.set_classic_framebuffer(false));
    let enlarged = renderer
        .capture_frame(FrameCaptureSource::Presented)
        .unwrap();
    assert_eq!((enlarged.width, enlarged.height), (8, 6));
    let boundary = &enlarged.rgba[4..8];
    // Pixel x=1 samples 25% of source pixel 1: mix([0,4,0],[8,8,8],.25).
    for (actual, expected) in boundary[..3].iter().zip([2_i32, 5, 2]) {
        assert!(
            (i32::from(*actual) - expected).abs() <= 1,
            "boundary {boundary:?}"
        );
    }

    renderer.set_window_size(1024, 768);
    renderer.set_scaling_mode(ScalingMode::FourThree, 1024, 768);
    assert!(renderer.set_classic_framebuffer(true));
    renderer.begin_scene(RenderScene::World);
    renderer.clear(1.0, 1.0, 1.0);
    renderer.present();
    let high = renderer
        .capture_frame(FrameCaptureSource::Presented)
        .unwrap();
    assert_eq!((high.width, high.height), (1024, 768));
    assert!(high
        .rgba
        .chunks_exact(4)
        .all(|pixel| pixel == [248, 252, 248, 255]));
    renderer.begin_scene(RenderScene::Menu);
    renderer.clear(0.0, 0.0, 0.0);
    renderer.draw_fullscreen(&high.rgba, high.width, high.height);
    renderer.present();
    assert_eq!(
        renderer
            .capture_frame(FrameCaptureSource::Presented)
            .unwrap(),
        high
    );
    renderer.resize(1920, 1080);
    assert_eq!(renderer.viewport_size(), (1024, 768));
    renderer.set_scaling_mode(ScalingMode::Native, 1024, 768);
    assert!(!renderer.classic_framebuffer_active());
}

#[test]
#[ignore = "requires a desktop OpenGL context"]
fn scene_capture_reads_back_before_swap_and_retains_precision_across_classic_toggles() {
    let sdl = sdl2::init().unwrap();
    let video = sdl.video().unwrap();
    let attributes = video.gl_attr();
    attributes.set_context_profile(sdl2::video::GLProfile::Compatibility);
    attributes.set_context_version(2, 1);
    attributes.set_double_buffer(true);
    let window = video
        .window("Frame capture source verification", 8, 6)
        .hidden()
        .opengl()
        .build()
        .unwrap();
    let mut renderer = GlRenderer::new(window, 8, 6).unwrap();
    renderer.set_scaling_mode(ScalingMode::Stretched, 8, 6);
    let front_pixels = [201, 151, 101, 255].repeat(8 * 6);
    let scene_pixels = [17, 35, 69, 255].repeat(8 * 6);
    let quantized_pixels = [16, 32, 64, 255].repeat(8 * 6);

    renderer.begin_scene(RenderScene::World);
    renderer.clear(0.0, 0.0, 0.0);
    renderer.draw_fullscreen(&front_pixels, 8, 6);
    renderer.present();
    let presented = renderer
        .capture_frame(FrameCaptureSource::Presented)
        .unwrap();
    assert_eq!(presented.rgba, front_pixels);

    renderer.begin_scene(RenderScene::World);
    renderer.clear(0.0, 0.0, 0.0);
    renderer.draw_fullscreen(&scene_pixels, 8, 6);
    let back = renderer
        .capture_frame(FrameCaptureSource::CurrentScene)
        .unwrap();
    assert_eq!((back.width, back.height), (8, 6));
    assert_eq!(back.rgba, scene_pixels);
    assert_eq!(
        renderer
            .capture_frame(FrameCaptureSource::Presented)
            .unwrap(),
        presented,
        "capturing BACK must neither swap nor read FRONT"
    );

    assert!(renderer.set_classic_framebuffer(true));
    renderer.begin_scene(RenderScene::World);
    renderer.clear(0.0, 0.0, 0.0);
    renderer.draw_fullscreen(&scene_pixels, 8, 6);
    let raw_scene = renderer
        .capture_frame(FrameCaptureSource::CurrentScene)
        .unwrap();
    assert_eq!(raw_scene.rgba, scene_pixels);
    renderer.present();
    assert_eq!(
        renderer
            .capture_frame(FrameCaptureSource::Presented)
            .unwrap()
            .rgba,
        quantized_pixels
    );

    assert!(!renderer.set_classic_framebuffer(false));
    renderer.begin_scene(RenderScene::World);
    renderer.clear(0.0, 0.0, 0.0);
    renderer.draw_fullscreen(&raw_scene.rgba, raw_scene.width, raw_scene.height);
    assert_eq!(
        renderer
            .capture_frame(FrameCaptureSource::CurrentScene)
            .unwrap(),
        raw_scene
    );
    renderer.present();
    assert_eq!(
        renderer
            .capture_frame(FrameCaptureSource::Presented)
            .unwrap(),
        raw_scene,
        "turning Classic off restores the retained low color bits"
    );

    assert!(renderer.set_classic_framebuffer(true));
    renderer.begin_scene(RenderScene::World);
    renderer.clear(0.0, 0.0, 0.0);
    renderer.draw_fullscreen(&raw_scene.rgba, raw_scene.width, raw_scene.height);
    assert_eq!(
        renderer
            .capture_frame(FrameCaptureSource::CurrentScene)
            .unwrap(),
        raw_scene
    );
    renderer.present();
    assert_eq!(
        renderer
            .capture_frame(FrameCaptureSource::Presented)
            .unwrap()
            .rgba,
        quantized_pixels,
        "presentation reapplies quantization without changing the scene capture"
    );
}
