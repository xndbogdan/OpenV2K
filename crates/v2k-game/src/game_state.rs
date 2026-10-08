//! Game state machine: drives the main loop through Intro -> Menu -> Playing,
//! including the authored in-game pause stack.
//!
//! The menu is data-driven: screen structure comes from the decoded EXE
//! screen records ([`crate::menu_data`]), navigation/dispatch from
//! [`crate::menu_engine`], and labels from the global OVL string pool
//! (`ResourceCache::global_string`). This module owns only presentation:
//! the per-screen view model ([`MenuState`]) consumed by the renderer,
//! fly transitions, the carousel animation, and the portable save/load row
//! view. The frontend Load entry preserves the retail memory-card access
//! screen and its two fly legs before substituting `SaveManager` data for the
//! platform-specific slot rows.

use v2k_render::GameConfig;

use crate::menu::generate_placeholder_icon;
use crate::menu_data::{
    DrawKind, ScreenDef, SelectAction, SettingId, MEMCARD_ACCESS, PAUSE_FULL, PAUSE_SIMPLE,
    SAVE_ACCESS_FAILED, SAVE_OVERWRITE_CONFIRM, SAVE_PROGRESS, SAVE_SUCCEEDED, VIS_FRONTEND,
    VIS_INGAME, VIS_INGAME_SP,
};
use crate::menu_engine::{MenuCommand, MenuEngine, MenuInput, SOUND_WHOOSH};
use crate::resource_cache::ResourceCache;
use crate::save::{
    NativeSaveRestore, SaveManager, SaveSlotStatus, SavedPlayerState, NUM_SAVE_SLOTS, NUM_SLOTS,
    SETTINGS_SLOT,
};

// ── Top-level game state ──

/// Why a level is being loaded and where control should go afterwards.
///
/// The post-Intro2 handoff uses the normal level-loading path between two
/// closing/opening Klaus presentation legs. Keeping that intent in the state
/// machine prevents an unrelated level load from accidentally resuming the
/// second handoff leg.
pub enum LoadingPurpose {
    /// Enter the level's ordinary mode (`OpeningCinematic` or `Playing`).
    Normal,
    /// Native mode6: retain the complete snapshot and restore its controller
    /// fields around the shared player-first authored world reconstruction.
    NativeSave { restore: Box<NativeSaveRestore> },
    /// Restore the explicit compatibility subset written by the port. Body
    /// attitude is included; mode, fuel, inventory, cargo, and campaign state
    /// remain outside this snapshot and must not be presented as full fidelity.
    PortableCompatibilityPreview { player: SavedPlayerState },
    /// The frontend fly-out completed and its persistent Klaus presentation
    /// must resume over the already-running Intro2 world.
    BeginIntro { shell: MenuShell },
    /// Resume the second Klaus handoff leg once the first world is ready.
    ///
    /// Retail keeps the same type-0 entity, component root, and
    /// `FUN_0042C090` state across this load. Carrying the live shell prevents
    /// the port from silently restarting its pose/twitch/RNG state halfway
    /// through the two-leg bridge.
    ResumePostIntro { shell: MenuShell },
    /// Replace one gameplay world after contact with an authored campaign
    /// warp marker. Section 13 supplies the route-specific destination pose;
    /// carrying it in the load request keeps loading independent of the
    /// source world after that world's resource layer has been replaced.
    CampaignWarp {
        source_level_id: u32,
        arrival_position_raw: [i16; 3],
        arrival_heading_raw: u16,
    },
}

impl LoadingPurpose {
    /// Ordinary authored reconstruction is shared across campaign worlds.
    /// Compatibility snapshots retain their separate restoration contract.
    pub fn uses_authored_world_publication(&self, level_id: u32) -> bool {
        (13..=49).contains(&level_id)
            && matches!(
                self,
                Self::Normal
                    | Self::NativeSave { .. }
                    | Self::ResumePostIntro { .. }
                    | Self::CampaignWarp { .. }
            )
    }
}

/// Which side of the first-world load the post-Intro2 Klaus handoff is on.
///
/// FUN_00453C20 closes the morph before the load; FUN_00451710 opens it over
/// the new world. The component's progress determines completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostIntroStage {
    /// Intro2 is still resident while Klaus closes from the full morph.
    BeforeWorldLoad,
    /// The first world is resident while Klaus opens and then hides.
    AfterWorldLoad,
}

/// Whether Klaus is still submitted over the already-running Intro2 world.
pub enum Intro2PresentationStage {
    /// Intro2 is resident and drawn underneath the authored Klaus morph.
    Covered { shell: MenuShell },
    /// The authored Intro2 world is visible. A frontend-started run retains
    /// its persistent Klaus presentation state even though the world, rather
    /// than the shell, is drawn. Direct development entry (`--level 50`) has
    /// no frontend shell and therefore stores `None`.
    Visible { shell: Option<MenuShell> },
}

impl Intro2PresentationStage {
    /// Advance the one continuous Intro2 clock and its optional cover.
    /// Returns the previous elapsed time for one-shot timeline crossings.
    /// Switching to [`Self::Visible`] deliberately leaves `elapsed` intact.
    /// Klaus consumes the caller's process RNG at this callback boundary.
    pub fn advance(
        &mut self,
        elapsed: &mut f32,
        frame_delta_micros: u32,
        effects: &mut impl KlausEffects,
    ) -> f32 {
        let dt = frame_delta_micros as f32 / 1_000_000.0;
        let previous_elapsed = *elapsed;
        *elapsed += dt;
        let reveal_complete = match self {
            Self::Covered { shell } => {
                shell.update(frame_delta_micros, effects);
                shell.take_transition_ready()
            }
            // Klaus's C090 component remains live throughout Intro2 in
            // 20260717-032945: the entity pointer, component root, wrapper,
            // state pointer and callback address all remain identical. Keep
            // advancing the retained presentation state while its draw is
            // suppressed.
            Self::Visible { shell: Some(shell) } => {
                shell.update(frame_delta_micros, effects);
                false
            }
            Self::Visible { shell: None } => false,
        };
        if reveal_complete {
            let covered = std::mem::replace(self, Self::Visible { shell: None });
            let Self::Covered { shell } = covered else {
                unreachable!("only the covered Intro2 stage can finish its reveal");
            };
            *self = Self::Visible { shell: Some(shell) };
        }
        previous_elapsed
    }

    /// Whether the Klaus opening has completed.
    pub fn is_visible(&self) -> bool {
        matches!(self, Self::Visible { .. })
    }

    /// Borrow the cover shell only while it is actually drawn.
    pub fn covered_shell(&self) -> Option<&MenuShell> {
        match self {
            Self::Covered { shell } => Some(shell),
            Self::Visible { .. } => None,
        }
    }

    /// Transfer the persistent frontend presentation into the post-Intro
    /// bridge. Direct Intro2 development entry deliberately returns `None`.
    pub fn take_shell(&mut self) -> Option<MenuShell> {
        match std::mem::replace(self, Self::Visible { shell: None }) {
            Self::Covered { shell } => Some(shell),
            Self::Visible { shell } => shell,
        }
    }
}

/// Top-level game state.
pub enum GameState {
    /// Playing the AVI intro video.
    Intro,
    /// Menu system (data-driven screen engine + presentation shell).
    Menu(MenuShell),
    /// Loading a level by ID for a typed continuation.
    Loading {
        level_id: u32,
        purpose: LoadingPurpose,
    },
    /// Non-interactive in-engine story level (`Intro2`).
    OpeningCinematic {
        /// world+0x290, consumed before the next world actor pass.
        exit_pending: bool,
        elapsed: f32,
        stage: Intro2PresentationStage,
    },
    /// Klaus closes over the completed Intro2 before the first-world load.
    /// The opening leg after load is a presentation owner over live gameplay.
    PostIntro { shell: MenuShell },
    /// Authored normal single-player in-game pause menu. Gameplay ownership
    /// remains outside the menu shell and stays resident while this is active.
    Paused { shell: MenuShell },
    /// Confirmed pause-menu quit (or the completed player-death lifecycle).
    /// This is the only state that tears down a live gameplay world.
    ReturningToFrontend,
    /// In-game (existing gameplay loop).
    Playing,
}

// ── Shell events (menu side effects for the main loop) ──

/// Menu side effects the shell cannot apply itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellEvent {
    /// Play a menu sound (global ids 0-3; FUN_0042A860).
    Sound(u32),
    /// Start a new game (main-ring vehicle prop).
    StartGame,
    /// Quit to desktop (Exit prop).
    QuitToDesktop,
    /// Resume gameplay after the pause switch-away fly-out commits.
    ResumeGame,
    /// Confirmed quit from the pause menu.
    QuitToFrontend,
    /// Load from save slot N (shell checked occupancy already).
    LoadSlot(usize),
    /// Save to slot N.
    SaveSlot(usize),
    /// A setting changed; sync GameConfig / audio / video.
    SettingChanged(SettingId, u32),
    /// Cheat granted (0x3E7xx weapon code) — gameplay wiring pending.
    CheatGranted(u32),
}

/// Pose inputs written by Klaus's `FUN_0042C090` callback before that same
/// callback consumes a pending command or advances either packed phase.
///
/// Keeping this as an explicit render snapshot preserves the retail split:
/// the model callback sees the pre-update pose while the entity transform
/// (Y/Z below) is allowed to advance for the current frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KlausBackdropAnimState {
    /// Private dword +0x10, published to selector 1 by FUN_0042C710.
    pub morph_progress: u16,
    /// C710's channel-6 wave amplitude from the pre-command morph. Command
    /// 3 overwrites selector 1 afterwards but leaves this wave unchanged.
    pub sway_amplitude_raw: u16,
    pub primary_phase: u16,
    pub twitch_phase: u16,
    pub twitch_strength: i32,
    pub model_state: u16,
    /// Component flag 0x800, changed after pose publication by C090.
    pub visible: bool,
}

/// Private state word 0 in Klaus's FUN_0042C090 component. Opening and hidden
/// states do not run the idle position/RNG branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KlausSequenceState {
    Idle,
    Selected,
    Opening,
    FlyOff,
    Hidden,
}

/// Frontend fade branches selected by `FUN_0042D210() == 1`.
/// The executable returns 4/5/2 for Selected/FlyOff/Hidden, not boolean true.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrontendDepthFadeMode {
    RingPulse,
    Receded,
}

// ── Known string indices (global pool ids; L2 occupies 0-120) ──

pub mod menu_strings {
    pub const COPYRIGHT: usize = 32;
    pub const LOADING: usize = 34;
    pub const SAVE_GAME: usize = 62;
    pub const LOAD_GAME: usize = 64;
    pub const USED_FOR_GAME_SETTINGS: usize = 80;
}

// ── Menu layout ──

/// How a menu screen's items are arranged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuLayout {
    /// Main menu: 3D elliptical carousel (the original prop ring).
    Carousel,
    /// Submenus: centered vertical list.
    VerticalList,
}

// ── Menu items (view model) ──

/// A single rendered menu item.
#[derive(Clone)]
pub struct MenuItem {
    /// Display label.
    pub label: String,
    /// Global Section 8 model pool id for 3D prop items (main ring).
    pub model_id: Option<u32>,
    /// Whether the cursor can rest on and activate this item. Decorative
    /// headings deliberately remain false without being visually disabled.
    pub selectable: bool,
    /// Whether this item is currently available. This is normally true,
    /// including for decorative rows; unavailable controls are dimmed.
    pub enabled: bool,
    /// Current value display for settings items (e.g. "On", "640x480").
    pub value_label: Option<String>,
    /// Spinner bar `(filled, total glyph width)` — rendered with the
    /// original's glyphs 0x1B-0x1E instead of a number. Used by volume,
    /// sensitivity and Active Camera rows. When set, `value_label` is ignored.
    pub bar: Option<(u32, u32)>,
    /// RGBA icon pixels (carousel mode only).
    pub icon_rgba: Vec<u8>,
    /// Icon width in pixels.
    pub icon_width: u32,
    /// Icon height in pixels.
    pub icon_height: u32,
}

impl MenuItem {
    /// Whether this item should receive cursor/highlight presentation.
    pub fn is_interactive(&self) -> bool {
        self.selectable && self.enabled
    }
}

// ── Menu transition phase ──
//
// The original has NO alpha fades. Screen changes between the ring and a
// submenu run the FLY transition: the 0x7000 clock decays at ~45776/s →
// 0.6264 s per leg (fly-out, stack switch, fly-in), during which the screen
// engine's tick early-outs (input ignored) and text is hidden. After the
// intro AVI there is a 3.0 s scene-only intro (DAT_004DB200 == 1) before
// the ring flies in.

/// One fly-transition leg (0x7000 units at ~45776/s).
pub const FLY_LEG_SECS: f32 = 0.6264;
/// Scene-only intro after the AVI (150 ticks at 50 Hz).
pub const INTRO_SECS: f32 = 3.0;
/// Selection-scroll exponential time constant (dv = −v·dt/τ, min step 1).
pub const SCROLL_TAU: f32 = 0.2097;
/// Ring prop spin period (DAT_004DCEB0 at ~19073 units/s → 0x10000).
pub const PROP_SPIN_SECS: f32 = 3.436;

/// Where the menu is in its presentation cycle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MenuPhase {
    /// 3.0 s live-scene intro; no UI drawn, input ignored.
    Intro { remaining: f32 },
    /// Old screen's props fly out (text hidden); the switched-to view is
    /// held in `MenuShell::view` while `prev_view` is what's rendered.
    FlyOut { remaining: f32 },
    /// New screen's props fly in from far.
    FlyIn { remaining: f32 },
    /// New Game: outgoing ring fly-off and background zoom, then load Intro2.
    GameStart { remaining: f32 },
    /// Intro2 is resident and ticking behind the first Klaus/iris reveal.
    IntroReveal,
    /// Close Klaus before loading world 1, then open over that resident world.
    PostIntro { stage: PostIntroStage },
    /// Interactive.
    Idle,
}

/// Fly-transition prop depth offsets, in raw engine units (÷100 for port
/// world units): `(selected_prop_z_offset, other_props_z_offset)` added to
/// the ring's base depth 0xDAC. Fly-out pulls the selected prop toward the
/// camera (−0x7000·t/16) and pushes the rest away (+0x7000·t/2); fly-in
/// brings everything in from far (+0x7000·(1−t)/2).
pub fn fly_offsets(phase: MenuPhase) -> (f32, f32) {
    match phase {
        MenuPhase::FlyOut { remaining } | MenuPhase::GameStart { remaining } => {
            let consumed = 0x7000 as f32 * (1.0 - remaining / FLY_LEG_SECS).clamp(0.0, 1.0);
            (-consumed / 16.0, consumed / 2.0)
        }
        MenuPhase::FlyIn { remaining } => {
            let clk = 0x7000 as f32 * (remaining / FLY_LEG_SECS).clamp(0.0, 1.0);
            (clk / 2.0, clk / 2.0)
        }
        MenuPhase::IntroReveal | MenuPhase::PostIntro { .. } => (0.0, 0.0),
        _ => (0.0, 0.0),
    }
}

/// Dynamic model callback channels 1 and 2 during a menu fly transition
/// (`LAB_0043B610`). The original supplies elapsed travel on fly-out and the
/// remaining clock on fly-in. Wrapper props consume this as an authored child
/// rotation: notably `screenop` turns its `screeno2` monitor one way while it
/// leaves the ring, then unwinds on the submenu fly-in.
pub fn fly_model_anim_raw(phase: MenuPhase) -> u16 {
    let value = match phase {
        MenuPhase::FlyOut { remaining } | MenuPhase::GameStart { remaining } => {
            0x7000 as f32 * (1.0 - remaining / FLY_LEG_SECS).clamp(0.0, 1.0)
        }
        MenuPhase::FlyIn { remaining } => {
            0x7000 as f32 * (remaining / FLY_LEG_SECS).clamp(0.0, 1.0)
        }
        MenuPhase::IntroReveal | MenuPhase::PostIntro { .. } => 0.0,
        _ => 0.0,
    };
    value.round() as u16
}

/// Signed number of ring notches to rotate when the selection moves from
/// `old` to `new` on an `n`-item ring, taking the SHORTEST way around. A
/// wrap (last→first) is +1, not −(n−1): the ring loops continuously instead
/// of spinning all the way back to the start.
fn ring_step_delta(old: usize, new: usize, n: usize) -> i32 {
    let mut delta = new as i32 - old as i32;
    let n = n as i32;
    if n > 0 {
        if delta * 2 > n {
            delta -= n;
        } else if delta * 2 < -n {
            delta += n;
        }
    }
    delta
}

/// Retail list-window policy: keep one row of look-ahead in the navigation
/// direction whenever the list bounds allow it. Selecting the bottom visible
/// row scrolls the window down; selecting the top visible row scrolls it up.
fn window_top(top: usize, selected: usize, n: usize, win: usize) -> usize {
    let win = win.min(n.max(1));
    if win <= 1 {
        return selected.min(n.saturating_sub(1));
    }
    let mut top = top;
    if selected <= top {
        top = selected.saturating_sub(1);
    } else if selected >= top.saturating_add(win - 1) {
        top = selected.saturating_add(2).saturating_sub(win);
    }
    top.min(n.saturating_sub(win))
}

// ── Menu state (view model) ──

/// Renderable state for the current menu screen.
#[derive(Clone)]
pub struct MenuState {
    /// Layout mode.
    pub layout: MenuLayout,
    /// Currently selected item index (into `items`).
    pub selected: usize,
    /// Visible items.
    pub items: Vec<MenuItem>,
    /// Screen title (displayed for submenus).
    pub title: String,
    /// Selection-scroll interpolator, mirroring DAT_004DCEBC/C0: cursor
    /// moves add ±200 per notch (circular-shortest, so a ring wrap is one
    /// notch, not a spin back — see [`ring_step_delta`]), then the value
    /// decays with τ ≈ 0.21 s. Ring rotation offset = (scroll / 200) · item
    /// spacing; list rows shift by step_y · scroll / 200 while the highlight
    /// snaps.
    pub scroll: f32,
    /// Global prop spin angle (radians; one revolution per 3.436 s —
    /// shared by ring props and screen backdrop props like the original's
    /// DAT_004DCEB0).
    pub spin: f32,
    /// Decorative 3D prop shown on this screen (global model pool id) —
    /// the non-selectable Prop item in the screen record (e.g. `screenop`
    /// on the Display screen), drawn at layout pt6 spinning at the global
    /// rate.
    pub backdrop_model: Option<u32>,
    /// Screen flag 0x10: draw the full-screen `optionsh` backdrop model
    /// (global model 73) behind the rows.
    pub has_backdrop_panel: bool,
    /// Authored row-group window size. Retail list navigation keeps a
    /// one-row look-ahead inside this window.
    list_window_rows: usize,
    /// Row-group flag 0x08.
    typewriter_rows: bool,
    /// Row-group flag 0x08 without flag 0x10: the row entering during a
    /// window scroll uses the independent DCEC8 reveal clock.
    retype_scrolled_rows: bool,
}

impl MenuState {
    fn empty() -> Self {
        Self {
            layout: MenuLayout::VerticalList,
            selected: 0,
            items: Vec::new(),
            title: String::new(),
            scroll: 0.0,
            spin: 0.0,
            backdrop_model: None,
            has_backdrop_panel: false,
            list_window_rows: LIST_WINDOW_ROWS,
            typewriter_rows: false,
            retype_scrolled_rows: false,
        }
    }
}

// ── View-building context ──

/// Borrowed lookups needed to build the menu view model.
pub struct MenuCtx<'a> {
    pub cache: &'a ResourceCache,
    pub config: &'a GameConfig,
    pub saves: Option<&'a SaveManager>,
}

impl<'a> MenuCtx<'a> {
    fn string(&self, id: usize) -> Option<String> {
        self.cache.global_string(id).map(|s| s.to_string())
    }

    fn string_or(&self, id: usize, fallback: &str) -> String {
        self.string(id).unwrap_or_else(|| fallback.to_string())
    }
}

// ── Portable save/load slot picker ──

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotOverlayMode {
    Load,
    Save,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SlotOverlayReturn {
    /// The synthetic rows replaced a real pushed screen in place. Back must
    /// therefore take the ordinary engine pop path, including its fly and
    /// Intro Sequence command.
    EnginePop,
    /// The synthetic rows are owned directly by their caller (currently the
    /// pause Save flow) and Back only dismisses them.
    Dismiss,
}

struct SlotOverlay {
    mode: SlotOverlayMode,
    selected: usize,
    return_path: SlotOverlayReturn,
    page: SlotOverlayPage,
}

/// Screen currently replacing the portable slot rows. These states mirror
/// the decoded retail records while retaining the synthetic rows underneath
/// for Load's pushed error acknowledgement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SlotOverlayPage {
    Slots,
    OverwriteConfirm { yes_selected: bool },
    Saving,
    SaveSucceeded,
    AccessFailed,
}

/// Portable slot data prepared while the retail access screen flies in.
///
/// `FUN_0043B9A0` waits for `_DAT_004CB4E0` to reach zero before replacing
/// `0x4C1740` with the 15-row load screen at `0x4C1D28`.  Building the view at
/// selection time lets [`MenuShell::update`] perform that exact clock-boundary
/// switch without borrowing the resource/save context from the main loop.
struct PendingSlotOverlay {
    overlay: SlotOverlay,
    view: MenuState,
}

/// Deferred action owned by the pause root's switch-away callback. Continue
/// first arms the ordinary 0x7000 fly clock; gameplay resumes only when that
/// clock reaches the callback boundary.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum PauseSwitchAway {
    #[default]
    Inactive,
    ResumeAfterFlyOut,
    ResumeReady,
}

// ── Menu shell ──

/// Presentation shell around the data-driven menu engine.
pub struct MenuShell {
    pub engine: MenuEngine,
    /// Presentation phase (intro / fly transition / interactive).
    pub phase: MenuPhase,
    view: MenuState,
    /// Snapshot of the outgoing screen, rendered during [`MenuPhase::FlyOut`]
    /// (the engine's stack has already switched — the original defers the
    /// switch instead; visually equivalent).
    prev_view: Option<MenuState>,
    /// Milliseconds since the current screen became interactive (DCEC4);
    /// drives the ordinary staggered typewriter reveal. Held at 0 during
    /// transitions.
    pub ms_open: f32,
    /// Milliseconds since the committed list window last moved (DCEC8).
    /// Unlike `ms_open`, retail resets this only when navigation changes the
    /// window top, not for an ordinary selection change inside the window.
    ms_since_list_scroll: f32,
    /// Vertical-list scrolling window (settings screens), mirroring the
    /// original's per-screen scroll offset (DAT_004DCA16) + DCEC0 smooth
    /// slide. `list_top` is the committed integer index of the top visible
    /// row; it changes when selection reaches either window edge so retail can
    /// retain a one-row navigation margin where list bounds allow it.
    /// `list_scroll` chases `list_top` smoothly (τ) — the rendered window
    /// origin — so a scroll glides one row instead of jumping.
    list_top: usize,
    list_scroll: f32,
    /// Engine item index for each view item.
    view_map: Vec<usize>,
    overlay: Option<SlotOverlay>,
    pending_slot_overlay: Option<PendingSlotOverlay>,
    klaus_sequence_state: KlausSequenceState,
    /// Whole-model mouth/iris morph, independent of selector 9's spin hinge.
    klaus_morph_progress_raw: u32,
    /// Single process-global command word (`DAT_004DB210`). Menu input writes
    /// it after the current actor callback; the following callback consumes
    /// the last value written.
    pending_intro_sequence_command: Option<u8>,
    /// Exact integer state from Klaus's 0x24-byte `FUN_0042C090` block.
    /// Command 4 holds the primary pose for about 0.786 seconds, then moves
    /// the actor away/up. The chase camera and background-billboard formulas
    /// consume these raw position words directly.
    klaus_primary_phase_raw: u32,
    klaus_position_y_offset_raw: i16,
    klaus_position_z_raw: i16,
    /// Intro Sequence word 5. Command 5 drains this from 0xFFFF to zero by
    /// `frame_delta_us / 8`, shrinking the V2000 flame during game start.
    frontend_background_zoom_raw: u32,
    /// Secondary random pose phase from FUN_0042C090. The original starts a
    /// 0..0xFFFF twitch roughly once the previous 2.1-4.1 second phase ends.
    klaus_twitch_phase_raw: u32,
    klaus_twitch_divisor: u32,
    /// `DAT_004DB218`: one in the interactive frontend, zero once game start
    /// commits. C090 copies it to model animation channel 2 before C710.
    klaus_model_state: u16,
    /// Pre-command/pre-advance values presented by C610/C710 this frame.
    klaus_render_anim_state: KlausBackdropAnimState,
    /// Set when a GameStart/PostIntro phase reaches its verified cutoff.
    transition_ready: bool,
    pause_switch_away: PauseSwitchAway,
    /// Whether this shell owns the live Klaus/frontend actor scene. This is
    /// deliberately separate from `from_gameplay`: the frontend always owns
    /// Klaus and its ring Load flow, while the pause shell owns gameplay Save
    /// without drawing the frontend actor.
    frontend_actor_active: bool,
    /// True when the menu was entered from gameplay (enables saving).
    pub from_gameplay: bool,
}

/// Fallback/synthetic-overlay row count. Decoded retail screens use their
/// authored row-group `window_rows` value directly.
pub const LIST_WINDOW_ROWS: usize = 5;

/// Retail's signed Q31 multiply used by the Klaus presentation callback.
/// The frame delta is clamped by the main loop before reaching this module,
/// so each shifted microsecond operand remains inside `i32`.
fn klaus_frame_q31(frame_delta_micros: u32, shift: u32, coefficient: i32) -> i32 {
    debug_assert!(frame_delta_micros <= (i32::MAX as u32 >> shift));
    (((frame_delta_micros << shift) as i64 * i64::from(coefficient)) >> 31) as i32
}

/// Advance one non-zero packed phase and clear it after its 0xFFFF endpoint.
fn advance_klaus_phase(phase: &mut u32, step: u32) {
    if *phase != 0 {
        *phase += step;
        if *phase > 0xFFFF {
            *phase = 0;
        }
    }
}

/// Effects invoked synchronously by Klaus's FUN_0042C090 callback.
///
/// Sound aliases and twitch sampling consume the same process RNG. A single
/// context preserves that interleaving, including when no audio device exists.
pub trait KlausEffects {
    fn next_random_u16(&mut self) -> u16;
    /// Resolve aliases now using the same stream as `next_random_u16`.
    /// Playback may be absent or deferred, but alias draws cannot be skipped.
    fn play_sound(&mut self, global_sound_id: u32);
}

/// Command 2's setup callback has no native sound or RNG calls.
struct OpeningPrimeEffects;

impl KlausEffects for OpeningPrimeEffects {
    fn next_random_u16(&mut self) -> u16 {
        unreachable!("command 2 primes Opening without a random draw")
    }

    fn play_sound(&mut self, _global_sound_id: u32) {
        unreachable!("command 2 primes Opening without a sound")
    }
}

/// Distinct retail roots for Escape in a world and S on the campaign map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SinglePlayerMenuKind {
    Pause,
    CampaignSave,
}

impl MenuShell {
    /// Frontend menu rooted at the main prop ring.
    ///
    /// `intro` selects the boot presentation: after the intro AVI the
    /// original shows the live 3D scene alone for 3.0 s before the ring
    /// flies in; returning from gameplay skips straight to the fly-in.
    pub fn new_frontend(ctx: &MenuCtx, intro: bool) -> Self {
        let phase = if intro {
            MenuPhase::Intro {
                remaining: INTRO_SECS,
            }
        } else {
            MenuPhase::FlyIn {
                remaining: FLY_LEG_SECS,
            }
        };
        Self::new_with_engine(ctx, MenuEngine::main_menu(), phase, false)
    }

    /// Escape owns 4D1190; campaign-map S enters 4D0AD8, whose 4557D0
    /// initializer pushes 4D1090 (Save Game / Continue / Options).
    pub fn new_single_player_menu(ctx: &MenuCtx, kind: SinglePlayerMenuKind) -> Self {
        let root = match kind {
            SinglePlayerMenuKind::Pause => PAUSE_FULL,
            SinglePlayerMenuKind::CampaignSave => PAUSE_SIMPLE,
        };
        let engine = MenuEngine::new(root, VIS_INGAME | VIS_INGAME_SP);
        Self::new_with_engine(
            ctx,
            engine,
            MenuPhase::FlyIn {
                remaining: FLY_LEG_SECS,
            },
            true,
        )
    }

    fn new_with_engine(
        ctx: &MenuCtx,
        mut engine: MenuEngine,
        phase: MenuPhase,
        from_gameplay: bool,
    ) -> Self {
        // The resolution spinner indexes the port's preset list (the
        // original indexed Section 5 display modes).
        engine.resolution_count = v2k_render::RESOLUTIONS.len() as u32;
        sync_settings_from_config(&mut engine, ctx.config);
        let mut shell = Self {
            engine,
            phase,
            view: MenuState::empty(),
            prev_view: None,
            ms_open: 0.0,
            ms_since_list_scroll: 0.0,
            list_top: 0,
            list_scroll: 0.0,
            view_map: Vec::new(),
            overlay: None,
            pending_slot_overlay: None,
            klaus_sequence_state: KlausSequenceState::Idle,
            klaus_morph_progress_raw: 0,
            pending_intro_sequence_command: None,
            klaus_primary_phase_raw: 0,
            klaus_position_y_offset_raw: 0,
            klaus_position_z_raw: 0x0A00,
            frontend_background_zoom_raw: 0xFFFF,
            klaus_twitch_phase_raw: 0,
            // FUN_0042BFD0 constructor word 3, before the first random
            // twitch replaces it with a sampled 0x20..0x3F divisor.
            klaus_twitch_divisor: 0x1E,
            klaus_model_state: 1,
            klaus_render_anim_state: KlausBackdropAnimState {
                morph_progress: 0,
                sway_amplitude_raw: 0xFFFF / 0x32,
                primary_phase: 0,
                twitch_phase: 0,
                twitch_strength: (0x30 - 0x1E) * 4,
                model_state: 1,
                visible: true,
            },
            transition_ready: false,
            pause_switch_away: PauseSwitchAway::Inactive,
            frontend_actor_active: !from_gameplay,
            from_gameplay,
        };
        shell.rebuild_view(ctx);
        shell
    }

    /// Scene-only Klaus interstitial used after Intro2.
    pub fn new_post_intro(ctx: &MenuCtx) -> Self {
        let mut shell = Self::new_frontend(ctx, false);
        shell.begin_post_intro_leg(PostIntroStage::BeforeWorldLoad);
        shell
    }

    /// Normal Intro2 exit enters 4D0A30/453C20 (command 3), waits for the
    /// closing morph, then loads through 4D0918/451710 (command 2). Preserve
    /// the persistent component across both halves, including its Y/Z pose.
    pub fn begin_post_intro_leg(&mut self, stage: PostIntroStage) {
        self.phase = MenuPhase::PostIntro { stage };
        self.klaus_model_state = 0;
        self.transition_ready = false;
        match stage {
            PostIntroStage::BeforeWorldLoad => self.pending_intro_sequence_command = Some(3),
            PostIntroStage::AfterWorldLoad => self.prime_klaus_opening(),
        }
    }

    /// Resume the persistent frontend presentation over Intro2's opening.
    /// This is normally entered by carrying the live shell
    /// through [`LoadingPurpose::BeginIntro`], rather than constructing a new
    /// Klaus animation state at the level boundary.
    pub fn begin_intro_reveal(&mut self) {
        self.phase = MenuPhase::IntroReveal;
        self.frontend_background_zoom_raw = 0;
        self.klaus_model_state = 0;
        self.transition_ready = false;
        self.prime_klaus_opening();
    }

    /// FUN_00451710 queues command 2 and requests one 125,000-us actor
    /// update. Publish the old pose first, then advance +0x10 by 7,812. This
    /// primes the retained, active frontend component without moving the UI
    /// or the reset world clock through an extra frame.
    fn prime_klaus_opening(&mut self) {
        self.pending_intro_sequence_command = Some(2);
        // C090 consumes command 2 before its state switch. Opening never
        // calls Random_Next, including this synchronous setup callback.
        self.update_klaus(125_000, &mut OpeningPrimeEffects);
    }

    /// Committed top row of the settings-list window (integer index).
    pub fn list_top(&self) -> usize {
        self.list_top
    }

    /// Smoothly-animated top-of-window origin, in row units (the render
    /// offset — `list_scroll.floor()` is the first drawn row, the fraction
    /// is the sub-row slide).
    pub fn list_scroll(&self) -> f32 {
        self.list_scroll
    }

    /// Authored row count of the active vertical-list window.
    pub fn list_window_rows(&self) -> usize {
        self.view.list_window_rows
    }

    /// Effective elapsed milliseconds for one row's reveal. Normally this is
    /// the staggered DCEC4 screen clock. While a list window is moving, the
    /// newly entering edge row instead uses DCEC8 unless row-group flag 0x10
    /// suppresses that behavior.
    pub fn list_row_typewriter_elapsed_ms(&self, item_index: usize) -> f32 {
        if !self.view.typewriter_rows {
            return f32::MAX;
        }

        let delta = self.list_top as f32 - self.list_scroll;
        if self.view.retype_scrolled_rows && delta.abs() >= 0.01 {
            let entering = if delta > 0.0 {
                self.list_top
                    .saturating_add(self.view.list_window_rows.saturating_sub(1))
                    .min(self.view.items.len().saturating_sub(1))
            } else {
                self.list_top
            };
            if item_index == entering {
                return self.ms_since_list_scroll;
            }
        }

        self.ms_open - item_index as f32 * 100.0
    }

    /// Klaus's captured frontend depth is the private Z word owned by
    /// `FUN_0042C090`: 0xA00 at rest and 0x1400 in the submenu-away pose.
    /// Active Camera is a gameplay chase-camera setting; the accepted
    /// 20260724 menu-pose sweep held this frontend output constant for every
    /// setting value from zero through ten.
    pub fn menu_backdrop_depth_raw(&self) -> f32 {
        f32::from(self.klaus_position_z_raw)
    }

    /// `42B5D0` samples this before C090; `42B870` samples it again after.
    /// Preserve the exact return-value comparison, including a pending command
    /// which has not yet been consumed by the actor callback.
    pub fn frontend_depth_fade_mode(&self) -> FrontendDepthFadeMode {
        if matches!(
            self.klaus_sequence_state,
            KlausSequenceState::Idle | KlausSequenceState::Opening
        ) && self.klaus_morph_progress_raw == 0
        {
            FrontendDepthFadeMode::RingPulse
        } else {
            FrontendDepthFadeMode::Receded
        }
    }

    /// Raw C090 inputs for the emblem's integer placement and D030 sizing.
    pub fn menu_billboard_pose(&self) -> crate::menu_billboard::MenuBillboardPose {
        crate::menu_billboard::MenuBillboardPose {
            zoom_raw: self.frontend_background_zoom_raw,
            height_raw: self.klaus_position_y_offset_raw,
        }
    }

    /// Low-resolution billboard Y adjustment from FUN_0042C090 (about -Y/24
    /// in the 320x240 composition space).
    pub fn menu_billboard_y_offset(&self) -> f32 {
        -f32::from(self.klaus_position_y_offset_raw) / 24.0
    }

    /// Primary click-driven pose phase and secondary random-twitch inputs for
    /// Klaus's FUN_0042C710 animation callback.
    pub fn menu_backdrop_anim_state(&self) -> KlausBackdropAnimState {
        self.klaus_render_anim_state
    }

    /// Consume a completed new-game/post-intro handoff.
    pub fn take_transition_ready(&mut self) -> bool {
        std::mem::take(&mut self.transition_ready)
    }

    /// Consume the pause root's switch-away action once its authored fly-out
    /// clock reaches zero. Before that boundary Continue has produced only its
    /// select sound and the shell remains fully paused.
    pub fn take_switch_away_event(&mut self) -> Option<ShellEvent> {
        if self.pause_switch_away == PauseSwitchAway::ResumeReady {
            self.pause_switch_away = PauseSwitchAway::Inactive;
            Some(ShellEvent::ResumeGame)
        } else {
            None
        }
    }

    pub fn view(&self) -> &MenuState {
        &self.view
    }

    /// The view to RENDER this frame: the outgoing screen while flying out,
    /// the current screen otherwise. `None` during the scene-only intro.
    pub fn render_view(&self) -> Option<&MenuState> {
        match self.phase {
            MenuPhase::Intro { .. } | MenuPhase::IntroReveal | MenuPhase::PostIntro { .. } => None,
            // Retail leaves the outgoing ring screen live while its 0x7000
            // pop clock sends selected model 41 toward the camera and the
            // other props away. Text and branding are gated separately.
            MenuPhase::GameStart { .. } => Some(&self.view),
            MenuPhase::FlyOut { .. } => Some(self.prev_view.as_ref().unwrap_or(&self.view)),
            _ => Some(&self.view),
        }
    }

    /// The Frontier/copyright/publisher strip is absent during both mouth
    /// bridges. It remains visible during the ordinary boot scene intro.
    pub fn frontend_branding_visible(&self) -> bool {
        !matches!(
            self.phase,
            MenuPhase::GameStart { .. } | MenuPhase::IntroReveal | MenuPhase::PostIntro { .. }
        )
    }

    /// True when retail's plain-Escape pop has no screen or overlay to remove.
    pub fn back_is_inert(&self) -> bool {
        self.overlay.is_none() && self.engine.depth() <= 1
    }

    pub fn is_transitioning(&self) -> bool {
        self.phase != MenuPhase::Idle
    }

    /// Consume the one command word after C610/C710 has captured this frame's
    /// pose, matching `FUN_0042C090` rather than mutating Klaus from the input
    /// callback itself.
    fn consume_pending_intro_sequence_command(&mut self, effects: &mut impl KlausEffects) {
        let Some(command) = self.pending_intro_sequence_command.take() else {
            return;
        };
        match command {
            2 => {
                self.klaus_sequence_state = KlausSequenceState::Opening;
                self.klaus_morph_progress_raw = 0;
            }
            3 => {
                self.klaus_sequence_state = KlausSequenceState::Idle;
                self.klaus_morph_progress_raw = 0xFFFF;
                self.frontend_background_zoom_raw = 0xFFFF;
                self.klaus_render_anim_state.visible = true;
                // C090 explicitly overwrites selector 1 after C610 for this
                // command only. The first closing frame shows the full pose.
                self.klaus_render_anim_state.morph_progress = 0xFFFF;
            }
            4 => {
                self.klaus_sequence_state = KlausSequenceState::Selected;
                self.klaus_primary_phase_raw = 1;
                effects.play_sound(SOUND_WHOOSH);
            }
            1 => {
                if self.klaus_sequence_state == KlausSequenceState::Selected {
                    // The selected-state branch restarts the packed pose and
                    // plays the transform cue before returning to state zero.
                    self.klaus_primary_phase_raw = 1;
                    effects.play_sound(SOUND_WHOOSH);
                } else {
                    // The other reachable branch snaps Z and zoom; Y settles
                    // through the ordinary state-zero path below.
                    self.klaus_position_z_raw = 0x0A00;
                    self.frontend_background_zoom_raw = 0xFFFF;
                    self.klaus_render_anim_state.visible = true;
                }
                self.klaus_sequence_state = KlausSequenceState::Idle;
            }
            5 => {
                self.klaus_sequence_state = KlausSequenceState::FlyOff;
            }
            _ => {}
        }
    }

    /// Advance phase, scroll interpolator, Klaus state and typewriter clocks.
    /// Sound aliases and twitch draws share `effects` in callback order;
    /// C090's random cadence follows callbacks, independent of dt.
    pub fn update(&mut self, frame_delta_micros: u32, effects: &mut impl KlausEffects) {
        let dt = frame_delta_micros as f32 / 1_000_000.0;
        let mut activate_pending_slot_overlay = false;
        self.phase = match self.phase {
            MenuPhase::Intro { remaining } => {
                let r = remaining - dt;
                if r <= 0.0 {
                    MenuPhase::FlyIn {
                        remaining: FLY_LEG_SECS,
                    }
                } else {
                    MenuPhase::Intro { remaining: r }
                }
            }
            MenuPhase::FlyOut { remaining } => {
                let r = remaining - dt;
                if r <= 0.0 {
                    if self.pause_switch_away == PauseSwitchAway::ResumeAfterFlyOut {
                        // PAUSE_FULL's switch-away callback runs at this exact
                        // 0x7000 clock boundary. Keep the fully flown-out
                        // frame renderable until the main loop consumes the
                        // deferred ResumeGame event.
                        self.pause_switch_away = PauseSwitchAway::ResumeReady;
                        MenuPhase::FlyOut { remaining: 0.0 }
                    } else {
                        // Ordinary stack switch: drop the old view, restart
                        // the typewriter clock, reset the newly shown list
                        // window, and fly the new screen in.
                        self.prev_view = None;
                        self.ms_open = 0.0;
                        self.list_top = 0;
                        self.list_scroll = 0.0;
                        MenuPhase::FlyIn {
                            remaining: FLY_LEG_SECS,
                        }
                    }
                } else {
                    MenuPhase::FlyOut { remaining: r }
                }
            }
            MenuPhase::FlyIn { remaining } => {
                let r = remaining - dt;
                if r <= 0.0 {
                    self.ms_open = 0.0;
                    activate_pending_slot_overlay = self.pending_slot_overlay.is_some();
                    MenuPhase::Idle
                } else {
                    MenuPhase::FlyIn { remaining: r }
                }
            }
            MenuPhase::GameStart { remaining } => {
                let r = remaining - dt;
                if r <= 0.0 {
                    self.transition_ready = true;
                    MenuPhase::GameStart { remaining: 0.0 }
                } else {
                    MenuPhase::GameStart { remaining: r }
                }
            }
            MenuPhase::IntroReveal => MenuPhase::IntroReveal,
            MenuPhase::PostIntro { stage } => {
                // Retail selects the next descriptor before this frame's
                // actor update. D210 closes at zero; opening clears 0x800.
                self.transition_ready |= self.pending_intro_sequence_command.is_none()
                    && match stage {
                        PostIntroStage::BeforeWorldLoad => self.klaus_morph_progress_raw == 0,
                        PostIntroStage::AfterWorldLoad => {
                            self.klaus_sequence_state == KlausSequenceState::Hidden
                        }
                    };
                MenuPhase::PostIntro { stage }
            }
            MenuPhase::Idle => {
                self.ms_open += dt * 1000.0;
                self.ms_since_list_scroll += dt * 1000.0;
                MenuPhase::Idle
            }
        };

        if activate_pending_slot_overlay {
            self.activate_pending_slot_overlay();
        }

        // Ring selection scroll: exponential decay with the original's
        // minimum step (terminates instead of asymptoting).
        let v = self.view.scroll;
        if v != 0.0 {
            let decayed = v * (-dt / SCROLL_TAU).exp();
            self.view.scroll = if decayed.abs() < 1.0 { 0.0 } else { decayed };
        }
        // Global prop spin (ring items + backdrop props).
        let spin_step = std::f32::consts::TAU * dt / PROP_SPIN_SECS;
        self.view.spin = (self.view.spin + spin_step) % std::f32::consts::TAU;
        // During fly-out `render_view()` deliberately returns the outgoing
        // snapshot. DAT_004DCEB0 is global and keeps advancing, so that
        // snapshot must follow it too; otherwise the selected prop visibly
        // freezes until the stack commits.
        if let Some(prev) = &mut self.prev_view {
            prev.spin = (prev.spin + spin_step) % std::f32::consts::TAU;
        }

        self.update_klaus(frame_delta_micros, effects);

        // The world already renders underneath Klaus. Retire the cover on
        // the callback that clears render admission, independent of ticks.
        if self.phase == MenuPhase::IntroReveal
            && self.klaus_sequence_state == KlausSequenceState::Hidden
        {
            self.transition_ready = true;
        }

        // Settings-list window: commit retail's one-row navigation margin,
        // then glide `list_scroll` toward that integer top.
        if self.view.layout == MenuLayout::VerticalList {
            self.commit_list_window();
            let target = self.list_top as f32;
            let d = target - self.list_scroll;
            self.list_scroll = if d.abs() < 0.01 {
                target
            } else {
                self.list_scroll + d * (1.0 - (-dt / SCROLL_TAU).exp())
            };
        } else {
            self.list_top = 0;
            self.list_scroll = 0.0;
        }
    }

    fn update_klaus(&mut self, frame_delta_micros: u32, effects: &mut impl KlausEffects) {
        // C090 calls C610/C710 first. Commands, phase advancement, random
        // twitch starts, and entity-position easing all occur afterwards.
        self.klaus_render_anim_state = KlausBackdropAnimState {
            morph_progress: self.klaus_morph_progress_raw as u16,
            sway_amplitude_raw: ((0xFFFF - self.klaus_morph_progress_raw) / 0x32) as u16,
            primary_phase: self.klaus_primary_phase_raw as u16,
            twitch_phase: self.klaus_twitch_phase_raw as u16,
            twitch_strength: (0x30 - self.klaus_twitch_divisor as i32) * 4,
            model_state: self.klaus_model_state,
            visible: self.klaus_render_anim_state.visible,
        };

        self.consume_pending_intro_sequence_command(effects);

        // Klaus's exact integer `FUN_0042C090` state. The callback advances
        // both packed phases from the clamped microsecond delta, then applies
        // its signed Q31 position easing. Command 4 selects the persistent
        // actor; command 1 returns it to the constructor pose.
        advance_klaus_phase(&mut self.klaus_primary_phase_raw, frame_delta_micros >> 5);
        advance_klaus_phase(
            &mut self.klaus_twitch_phase_raw,
            frame_delta_micros / self.klaus_twitch_divisor.max(1),
        );
        if self.klaus_sequence_state == KlausSequenceState::Selected {
            if (self.klaus_primary_phase_raw == 0 || self.klaus_primary_phase_raw > 0x6000)
                && self.klaus_position_z_raw < 0x1400
            {
                let step = klaus_frame_q31(
                    frame_delta_micros,
                    12,
                    0x15F4 - i32::from(self.klaus_position_z_raw),
                ) * 4
                    / 3;
                self.klaus_position_z_raw =
                    (i32::from(self.klaus_position_z_raw) + step).min(0x1400) as i16;
                self.klaus_position_y_offset_raw =
                    (i32::from(self.klaus_position_y_offset_raw) + step / 2) as i16;
            }
        } else if matches!(
            self.klaus_sequence_state,
            KlausSequenceState::Idle | KlausSequenceState::FlyOff
        ) {
            self.klaus_morph_progress_raw = self
                .klaus_morph_progress_raw
                .saturating_sub(frame_delta_micros >> 4);
            if self.klaus_sequence_state == KlausSequenceState::FlyOff {
                self.frontend_background_zoom_raw = self
                    .frontend_background_zoom_raw
                    .saturating_sub(frame_delta_micros / 8);
            }
            let y = i32::from(self.klaus_position_y_offset_raw);
            if y < 0 {
                self.klaus_position_y_offset_raw =
                    (y + klaus_frame_q31(frame_delta_micros, 12, 700)).min(0) as i16;
            } else if y > 0 {
                self.klaus_position_y_offset_raw =
                    (y - klaus_frame_q31(frame_delta_micros, 12, y * 2 + 400)).max(0) as i16;
            }

            let z = i32::from(self.klaus_position_z_raw);
            if z < 0x0A00 {
                self.klaus_position_z_raw =
                    (z + klaus_frame_q31(frame_delta_micros, 11, 2500)).min(0x0A00) as i16;
            } else if z > 0x0A00 {
                self.klaus_position_z_raw =
                    (z - klaus_frame_q31(frame_delta_micros, 12, z - 0x0230)).max(0x0A00) as i16;
            }
        } else if self.klaus_sequence_state == KlausSequenceState::Opening {
            self.klaus_morph_progress_raw += frame_delta_micros >> 4;
            if self.klaus_morph_progress_raw > 0xFFFF {
                self.klaus_morph_progress_raw = 0;
                self.klaus_sequence_state = KlausSequenceState::Hidden;
                self.klaus_render_anim_state.visible = false;
            }
        }

        // C090 calls the process-global 457930 RNG once per invocation in
        // states 0/1/3, even at dt=0 or while a twitch is running. Only a
        // successful phase-zero gate consumes the second divisor word.
        // Opening/Hidden and shells without the frontend actor consume none.
        if self.frontend_actor_active
            && matches!(
                self.klaus_sequence_state,
                KlausSequenceState::Idle
                    | KlausSequenceState::Selected
                    | KlausSequenceState::FlyOff
            )
        {
            let sample = effects.next_random_u16();
            if sample & 0x1F == 0 && self.klaus_twitch_phase_raw == 0 {
                self.klaus_twitch_phase_raw = 1;
                let sample = effects.next_random_u16();
                self.klaus_twitch_divisor = 0x20 + u32::from(sample >> 11);
            }
        }
    }

    /// Rebuild the view model (after input or external changes).
    pub fn refresh(&mut self, ctx: &MenuCtx) {
        self.rebuild_view(ctx);
        self.commit_list_window();
    }

    /// Resolve the synchronous portable write represented by retail's
    /// 0x4C17B0 progress screen. Success replaces it with "Game Saved";
    /// failure replaces it with "Access Failed". Both acknowledgements pop
    /// back to the pause root when selected.
    pub fn complete_save(&mut self, succeeded: bool, ctx: &MenuCtx) {
        let Some(overlay) = self.overlay.as_mut() else {
            return;
        };
        if overlay.mode != SlotOverlayMode::Save || overlay.page != SlotOverlayPage::Saving {
            return;
        }
        overlay.page = if succeeded {
            SlotOverlayPage::SaveSucceeded
        } else {
            SlotOverlayPage::AccessFailed
        };
        self.ms_open = 0.0;
        self.list_top = 0;
        self.list_scroll = 0.0;
        self.rebuild_view(ctx);
        self.commit_list_window();
    }

    /// Handle a menu input, returning side effects for the main loop.
    ///
    /// Ignored during transitions (the original's screen-engine tick
    /// early-outs while the fly clock runs).
    pub fn input(&mut self, input: MenuInput, ctx: &MenuCtx) -> Vec<ShellEvent> {
        if self.is_transitioning() {
            return Vec::new();
        }
        let old_depth = self.engine.depth();
        let old_sel = self.view.selected;
        let old_view = self.view.clone();
        let events = if self.overlay.is_some() {
            self.overlay_input(input, ctx)
        } else {
            let commands = self.engine.handle(input);
            self.apply_commands(&commands)
        };

        // The retail frontend does not jump straight from the ring to its
        // load rows. It pushes 0x4C1740, performs the ordinary 0.6264-second
        // fly-out and fly-in, then FUN_0043B9A0 replaces the access screen
        // exactly when the fly clock reaches zero. Prepare the portable rows
        // now, but leave the decoded access screen active for both legs.
        if self.overlay.is_none()
            && self.pending_slot_overlay.is_none()
            && old_depth < self.engine.depth()
            && self.engine.current_va() == Some(MEMCARD_ACCESS)
        {
            let overlay = SlotOverlay {
                mode: if self.from_gameplay {
                    SlotOverlayMode::Save
                } else {
                    SlotOverlayMode::Load
                },
                selected: 0,
                return_path: if !self.from_gameplay && old_depth == 1 {
                    SlotOverlayReturn::EnginePop
                } else {
                    SlotOverlayReturn::Dismiss
                },
                page: SlotOverlayPage::Slots,
            };
            if !self.from_gameplay && old_depth == 1 {
                self.pending_slot_overlay = Some(PendingSlotOverlay {
                    view: build_overlay_view(&overlay, &self.engine, ctx),
                    overlay,
                });
            } else {
                // Only the frontend ring-to-Load sequence was captured. Keep
                // the existing immediate portable substitution for deeper or
                // gameplay-owned memory-card paths until their own evidence
                // is available, rather than stranding a pending overlay in a
                // transition that does not have the root fly legs.
                self.engine.pop();
                self.overlay = Some(overlay);
            }
        }
        self.rebuild_view(ctx);

        let new_depth = self.engine.depth();
        if new_depth != old_depth && self.overlay.is_none() {
            // Screen switched. Ring↔submenu (depth 1↔2, both screens carry
            // fly flag 0x40 in the frontend) runs the fly transition; deeper
            // pushes/pops switch instantly (flag 0x20 base-depth rule).
            if old_depth.min(new_depth) == 1 && old_depth.max(new_depth) == 2 {
                self.prev_view = Some(old_view);
                self.phase = MenuPhase::FlyOut {
                    remaining: FLY_LEG_SECS,
                };
                // The list window resets when the fly commits (see update),
                // so the outgoing list keeps its scroll during the fly-out.
            } else {
                self.ms_open = 0.0;
                // Non-flying switch shows instantly — reset the list window.
                self.list_top = 0;
                self.list_scroll = 0.0;
            }
        } else if self.view.selected != old_sel
            && self.view.layout == old_view.layout
            && self.view.layout == MenuLayout::Carousel
        {
            // Ring cursor moved: bump the scroll interpolator by the
            // CIRCULAR-SHORTEST index delta ×200 so the ring is a seamless
            // infinite carousel — stepping off the last item rotates one
            // notch forward into the first, rather than spinning all the way
            // back to the beginning. (User ground truth 2026-07-05: the
            // original loops continuously; the earlier "long-way wrap" RE
            // reading was wrong.) The vertical settings list does NOT use
            // this — its window scrolls independently (see update()), so the
            // highlight just snaps between rows while the list stays put.
            let delta = ring_step_delta(old_sel, self.view.selected, self.view.items.len());
            self.view.scroll = old_view.scroll + delta as f32 * 200.0;
        }
        // Retail commits DAT_004DCA16 and resets DCEC8 in the same navigation
        // callback that changes the selection, before the next rendered tick.
        self.commit_list_window();
        events
    }

    fn apply_commands(&mut self, commands: &[MenuCommand]) -> Vec<ShellEvent> {
        let mut events = Vec::new();
        for &cmd in commands {
            match cmd {
                MenuCommand::PlaySound(id) => events.push(ShellEvent::Sound(id)),
                MenuCommand::IntroSequenceCommand(4) => {
                    self.pending_intro_sequence_command = Some(4);
                }
                MenuCommand::IntroSequenceCommand(1) => {
                    self.pending_intro_sequence_command = Some(1);
                }
                MenuCommand::IntroSequenceCommand(_) => {}
                // `input` recognizes MEMCARD_ACCESS after the commands have
                // committed, prepares the portable rows, and preserves the
                // real pushed screen for its normal fly sequence.
                MenuCommand::ScreenPushed(_) => {}
                // Fly transitions are decided in `input()` from the depth
                // change; nothing to do per command.
                MenuCommand::ScreenPopped => {}
                MenuCommand::SettingChanged(id, v) => {
                    events.push(ShellEvent::SettingChanged(id, v))
                }
                MenuCommand::StartGame => {
                    // Command 5's background zoom reaches zero after 524 ms,
                    // but the outgoing ring remains live until the ordinary
                    // 0x7000 fly clock completes at 626.4 ms.
                    self.phase = MenuPhase::GameStart {
                        remaining: FLY_LEG_SECS,
                    };
                    self.pending_intro_sequence_command = Some(5);
                    self.transition_ready = false;
                }
                MenuCommand::QuitToDesktop => events.push(ShellEvent::QuitToDesktop),
                MenuCommand::QuitToFrontend => events.push(ShellEvent::QuitToFrontend),
                MenuCommand::ResumeGame if self.from_gameplay => {
                    // FUN_00456170 plays sound 3 (the preceding command),
                    // marks the live session for resume, then FUN_0043A8E0
                    // arms the 0x7000 fly-out. PAUSE_FULL's 0x450E40
                    // switch-away callback owns the eventual resume boundary.
                    self.prev_view = Some(self.view.clone());
                    self.phase = MenuPhase::FlyOut {
                        remaining: FLY_LEG_SECS,
                    };
                    self.pause_switch_away = PauseSwitchAway::ResumeAfterFlyOut;
                }
                MenuCommand::ResumeGame => events.push(ShellEvent::ResumeGame),
                MenuCommand::OpenSaveFlow => {
                    self.overlay = Some(SlotOverlay {
                        mode: SlotOverlayMode::Save,
                        selected: 0,
                        return_path: SlotOverlayReturn::Dismiss,
                        page: SlotOverlayPage::Slots,
                    });
                }
                MenuCommand::GrantCheat(code) => events.push(ShellEvent::CheatGranted(code)),
                // Engine-side memory-card slot flow is superseded by the
                // overlay; world cheats await gameplay wiring.
                MenuCommand::SaveSlotPicked
                | MenuCommand::LoadSlotPicked
                | MenuCommand::CheatWorld(_) => {}
            }
        }
        events
    }

    fn overlay_input(&mut self, input: MenuInput, ctx: &MenuCtx) -> Vec<ShellEvent> {
        let Some((mode, page)) = self
            .overlay
            .as_ref()
            .map(|overlay| (overlay.mode, overlay.page))
        else {
            return Vec::new();
        };

        match page {
            SlotOverlayPage::Slots => {
                if input == MenuInput::Back {
                    return self.dismiss_slot_overlay();
                }

                let overlay = self.overlay.as_mut().expect("overlay was checked above");
                match input {
                    MenuInput::Up => {
                        overlay.selected = (overlay.selected + NUM_SAVE_SLOTS - 1) % NUM_SAVE_SLOTS;
                        vec![ShellEvent::Sound(0)]
                    }
                    MenuInput::Down => {
                        overlay.selected = (overlay.selected + 1) % NUM_SAVE_SLOTS;
                        vec![ShellEvent::Sound(0)]
                    }
                    MenuInput::Select => {
                        let slot = overlay.selected;
                        if slot >= NUM_SAVE_SLOTS {
                            return vec![ShellEvent::Sound(0)];
                        }
                        match mode {
                            SlotOverlayMode::Load => {
                                let (status, loadable) = ctx
                                    .saves
                                    .map(|saves| (saves.slot_status(slot), saves.is_loadable(slot)))
                                    .unwrap_or((SaveSlotStatus::Missing, false));
                                if loadable {
                                    // Frontend Select dispatches FUN_0043BFC0
                                    // without the pause wrapper's sound 3.
                                    vec![ShellEvent::LoadSlot(slot)]
                                } else if status == SaveSlotStatus::Missing {
                                    // FUN_0043BFC0 states 1/2: feedback only;
                                    // the slot list remains current.
                                    vec![ShellEvent::Sound(2)]
                                } else {
                                    // State 3 and a state-4 full-load failure
                                    // both push 0x4C1890 over the Load list.
                                    overlay.page = SlotOverlayPage::AccessFailed;
                                    vec![ShellEvent::Sound(2)]
                                }
                            }
                            SlotOverlayMode::Save => {
                                let confirm_overwrite = ctx.saves.is_some_and(|saves| {
                                    saves.requires_overwrite_confirmation(slot)
                                });
                                if confirm_overwrite {
                                    // FUN_0043BF30 replaces the slot list with
                                    // 0x4C1598; the pause Select wrapper plays
                                    // sound 3 before its first selectable Yes.
                                    overlay.page =
                                        SlotOverlayPage::OverwriteConfirm { yes_selected: true };
                                    vec![ShellEvent::Sound(3)]
                                } else {
                                    overlay.page = SlotOverlayPage::Saving;
                                    vec![ShellEvent::Sound(3), ShellEvent::SaveSlot(slot)]
                                }
                            }
                        }
                    }
                    MenuInput::Back => unreachable!("Back is handled before the overlay borrow"),
                    MenuInput::Left | MenuInput::Right => Vec::new(),
                }
            }
            SlotOverlayPage::OverwriteConfirm { yes_selected } => match input {
                MenuInput::Back => self.dismiss_slot_overlay(),
                MenuInput::Up | MenuInput::Down | MenuInput::Left | MenuInput::Right => {
                    self.overlay
                        .as_mut()
                        .expect("overlay was checked above")
                        .page = SlotOverlayPage::OverwriteConfirm {
                        yes_selected: !yes_selected,
                    };
                    vec![ShellEvent::Sound(0)]
                }
                MenuInput::Select if yes_selected => {
                    let overlay = self.overlay.as_mut().expect("overlay was checked above");
                    overlay.page = SlotOverlayPage::Saving;
                    vec![ShellEvent::Sound(3), ShellEvent::SaveSlot(overlay.selected)]
                }
                MenuInput::Select => self.dismiss_slot_overlay(),
            },
            SlotOverlayPage::AccessFailed => match input {
                MenuInput::Select if mode == SlotOverlayMode::Load => {
                    // FUN_0043BFC0 pushed the load error, so acknowledgement
                    // returns to the existing rows rather than leaving Load.
                    // Frontend Select and FUN_0043B850 are both silent.
                    self.overlay
                        .as_mut()
                        .expect("overlay was checked above")
                        .page = SlotOverlayPage::Slots;
                    Vec::new()
                }
                MenuInput::Back if mode == SlotOverlayMode::Load => {
                    // The global Back wrapper supplies sound 3 even though the
                    // same screen's Select callback is silent.
                    self.overlay
                        .as_mut()
                        .expect("overlay was checked above")
                        .page = SlotOverlayPage::Slots;
                    vec![ShellEvent::Sound(3)]
                }
                MenuInput::Back | MenuInput::Select => self.dismiss_slot_overlay(),
                MenuInput::Up | MenuInput::Down | MenuInput::Left | MenuInput::Right => Vec::new(),
            },
            SlotOverlayPage::SaveSucceeded => match input {
                MenuInput::Back | MenuInput::Select => self.dismiss_slot_overlay(),
                MenuInput::Up | MenuInput::Down | MenuInput::Left | MenuInput::Right => Vec::new(),
            },
            // The synchronous write completes through `complete_save`; input
            // cannot replace the progress screen while that operation owns it.
            SlotOverlayPage::Saving => Vec::new(),
        }
    }

    fn dismiss_slot_overlay(&mut self) -> Vec<ShellEvent> {
        let Some(return_path) = self.overlay.as_ref().map(|overlay| overlay.return_path) else {
            return Vec::new();
        };
        self.overlay = None;
        match return_path {
            SlotOverlayReturn::EnginePop => {
                let commands = self.engine.handle(MenuInput::Back);
                self.apply_commands(&commands)
            }
            SlotOverlayReturn::Dismiss => vec![ShellEvent::Sound(3)],
        }
    }

    fn activate_pending_slot_overlay(&mut self) {
        let Some(mut pending) = self.pending_slot_overlay.take() else {
            return;
        };

        // FUN_0043A8A0 replaces the depth-2 screen in place. Keep the decoded
        // access screen as the synthetic rows' logical stack owner so Back
        // can use the ordinary depth-2 -> depth-1 pop/fly path.
        pending.view.spin = self.view.spin;
        self.overlay = Some(pending.overlay);
        self.view = pending.view;
        self.view_map.clear();
        self.list_top = 0;
        self.list_scroll = 0.0;
        self.ms_open = 0.0;
    }

    // ── View building ──

    fn rebuild_view(&mut self, ctx: &MenuCtx) {
        let (scroll, spin) = (self.view.scroll, self.view.spin);
        let mut view = if let Some(overlay) = &self.overlay {
            build_overlay_view(overlay, &self.engine, ctx)
        } else if let Some(screen) = self.engine.current() {
            let (v, map) = build_screen_view(screen, &self.engine, ctx);
            self.view_map = map;
            v
        } else {
            MenuState::empty()
        };

        // Preserve animation continuity across rebuilds.
        view.scroll = scroll;
        view.spin = spin;
        self.view = view;
    }

    fn commit_list_window(&mut self) {
        if self.view.layout != MenuLayout::VerticalList {
            self.list_top = 0;
            self.list_scroll = 0.0;
            return;
        }

        let new_top = window_top(
            self.list_top,
            self.view.selected,
            self.view.items.len(),
            self.view.list_window_rows,
        );
        if new_top != self.list_top {
            self.list_top = new_top;
            self.ms_since_list_scroll = 0.0;
        }
    }
}

// ── View builders ──

fn build_overlay_view(overlay: &SlotOverlay, engine: &MenuEngine, ctx: &MenuCtx) -> MenuState {
    let decoded = match overlay.page {
        SlotOverlayPage::Slots => None,
        SlotOverlayPage::OverwriteConfirm { yes_selected } => Some((
            SAVE_OVERWRITE_CONFIRM,
            // Items 0/1 are the heading and spacer; 2/3 are Yes/No.
            if yes_selected { 2 } else { 3 },
        )),
        SlotOverlayPage::Saving => Some((SAVE_PROGRESS, 0)),
        SlotOverlayPage::SaveSucceeded => Some((SAVE_SUCCEEDED, 0)),
        SlotOverlayPage::AccessFailed => Some((SAVE_ACCESS_FAILED, 0)),
    };
    if let Some((screen_va, selected_item)) = decoded {
        let screen = engine
            .tree()
            .screen(screen_va)
            .expect("decoded save/load feedback screen must exist");
        // Memory-card screens use the frontend/slopt presentation tier even
        // when the owning caller is the in-game pause menu. The decoded prop
        // records therefore carry VIS_FRONTEND while their text also admits
        // the gameplay mask.
        return build_screen_view_with_selected(
            screen,
            engine,
            ctx,
            selected_item,
            engine.vis_mask | VIS_FRONTEND,
        )
        .0;
    }

    let title = match overlay.mode {
        SlotOverlayMode::Load => ctx.string_or(menu_strings::LOAD_GAME, "Load Game"),
        SlotOverlayMode::Save => ctx.string_or(menu_strings::SAVE_GAME, "Save Game"),
    };
    let mut items = Vec::with_capacity(NUM_SLOTS);
    for i in 0..NUM_SLOTS {
        let settings_row = i == SETTINGS_SLOT;
        let label = if settings_row {
            ctx.string_or(
                menu_strings::USED_FOR_GAME_SETTINGS,
                "Used for game settings",
            )
        } else {
            ctx.saves.map(|s| s.slot_label(i)).unwrap_or_default()
        };
        items.push(MenuItem {
            label,
            model_id: None,
            selectable: !settings_row,
            // Retail gives all fourteen slot rows flags 4 plus the selection
            // callback, including missing Load slots. Row 14 is informational:
            // it draws normally but has no selection callback.
            enabled: true,
            value_label: None,
            bar: None,
            icon_rgba: vec![],
            icon_width: 0,
            icon_height: 0,
        });
    }
    MenuState {
        layout: MenuLayout::VerticalList,
        selected: overlay.selected,
        items,
        title,
        // The original memory-card screens display the slopt prop.
        backdrop_model: Some(320),
        has_backdrop_panel: true,
        list_window_rows: LIST_WINDOW_ROWS,
        typewriter_rows: true,
        retype_scrolled_rows: true,
        ..MenuState::empty()
    }
}

fn build_screen_view(
    screen: &ScreenDef,
    engine: &MenuEngine,
    ctx: &MenuCtx,
) -> (MenuState, Vec<usize>) {
    build_screen_view_with_selected(screen, engine, ctx, engine.selected(), engine.vis_mask)
}

fn build_screen_view_with_selected(
    screen: &ScreenDef,
    engine: &MenuEngine,
    ctx: &MenuCtx,
    selected_item: usize,
    mask: u32,
) -> (MenuState, Vec<usize>) {
    let ring = screen.is_ring();
    let mut items = Vec::new();
    let mut map = Vec::new();
    let mut backdrop_model = None;

    for (idx, item) in screen.items.iter().enumerate() {
        if !item.visible(mask) {
            continue;
        }
        // Decorative props on list screens are the screen's 3D backdrop,
        // not list rows.
        if !ring && matches!(item.draw, DrawKind::Prop { .. }) && item.select == SelectAction::None
        {
            if let DrawKind::Prop { model_id, .. } = item.draw {
                backdrop_model = Some(model_id);
            }
            continue;
        }

        let label = item_label(item, ctx);
        let value_label = item_value(item, engine, ctx);
        let bar = item_bar(item, engine);
        let (icon_rgba, icon_width, icon_height) = if ring {
            ring_icon(items.len())
        } else {
            (vec![], 0, 0)
        };

        items.push(MenuItem {
            label,
            model_id: match item.draw {
                DrawKind::Prop { model_id, .. } => Some(model_id),
                _ => None,
            },
            selectable: item.selectable(mask),
            enabled: engine.item_available(item),
            value_label,
            bar,
            icon_rgba,
            icon_width,
            icon_height,
        });
        map.push(idx);
    }

    let selected = map
        .iter()
        .position(|&engine_idx| engine_idx == selected_item)
        .unwrap_or(0);
    let text_group = screen.groups.iter().find(|group| group.flags & 0x08 != 0);
    let list_window_rows = text_group
        .map(|group| usize::from(group.window_rows).max(1))
        .unwrap_or(LIST_WINDOW_ROWS);
    let typewriter_rows = text_group.is_some();
    let retype_scrolled_rows = text_group
        .map(|group| group.flags & 0x10 == 0)
        .unwrap_or(false);

    let state = MenuState {
        layout: if ring {
            MenuLayout::Carousel
        } else {
            MenuLayout::VerticalList
        },
        selected,
        items,
        // The original draws no screen titles — context comes from the
        // backdrop/decoration props. (Synthetic overlay screens keep one.)
        title: String::new(),
        backdrop_model,
        has_backdrop_panel: screen.flags & 0x10 != 0,
        list_window_rows,
        typewriter_rows,
        retype_scrolled_rows,
        ..MenuState::empty()
    };
    (state, map)
}

/// Placeholder ring icon colors (the renderer substitutes OVL trophy
/// sprites by position; these only show if sprite decode failed).
fn ring_icon(position: usize) -> (Vec<u8>, u32, u32) {
    const COLORS: [(u8, u8, u8); 7] = [
        (50, 180, 80),
        (200, 160, 40),
        (60, 120, 200),
        (140, 60, 180),
        (60, 180, 180),
        (160, 160, 160),
        (180, 60, 60),
    ];
    let (r, g, b) = COLORS[position % COLORS.len()];
    generate_placeholder_icon(r, g, b, 64, 64)
}

fn item_label(item: &crate::menu_data::ItemDef, ctx: &MenuCtx) -> String {
    let resolved = match item.draw {
        DrawKind::PortText { text } => Some(text.to_string()),
        DrawKind::Text { string_id }
        | DrawKind::ColumnText { string_id }
        | DrawKind::SaveSlotText { string_id }
        | DrawKind::NetworkText { string_id }
        | DrawKind::StatusText { string_id }
        | DrawKind::SettingRow { string_id, .. }
        | DrawKind::SettingRowNumeric { string_id, .. }
        | DrawKind::SettingRowNamed { string_id, .. }
        | DrawKind::Unknown { string_id, .. } => ctx.string(string_id as usize),
        DrawKind::Prop { label_id, .. } => label_id.and_then(|id| ctx.string(id as usize)),
        DrawKind::DynamicText => None,
    };
    resolved
        .or_else(|| {
            item.label
                .clone()
                // Extractor prop fallbacks look like "<prop:optexit>".
                .map(|l| {
                    l.trim_start_matches("<prop:")
                        .trim_end_matches('>')
                        .to_string()
                })
        })
        .unwrap_or_default()
}

/// Value text for setting rows. Value-string conventions follow the
/// observed Section 2 layout (e.g. Self Righting → Off/Level/Angled at
/// label+1..; Display → In a Window / Full Screen at 6/7).
fn item_value(
    item: &crate::menu_data::ItemDef,
    engine: &MenuEngine,
    ctx: &MenuCtx,
) -> Option<String> {
    let setting = match item.select {
        SelectAction::Spinner { setting, .. } => setting,
        SelectAction::Toggle { setting } => setting,
        SelectAction::ResolutionSpinner => SettingId::Resolution,
        _ => return None,
    };
    let v = engine.settings.get(setting);
    let label_id = match item.draw {
        DrawKind::SettingRow { string_id, .. }
        | DrawKind::SettingRowNumeric { string_id, .. }
        | DrawKind::SettingRowNamed { string_id, .. } => string_id as usize,
        _ => 0,
    };

    let text = match setting {
        SettingId::Resolution => ctx.config.resolution_label(),
        // Retail's values are Software (string 9) and Direct3D (string 10);
        // the port's hardware renderer is OpenGL.
        SettingId::Rendering => {
            if v == 0 {
                ctx.string_or(9, "Software")
            } else {
                "OpenGL".to_string()
            }
        }
        SettingId::FullScreen => ctx.string_or(
            6 + v as usize,
            if v == 0 { "In a Window" } else { "Full Screen" },
        ),
        SettingId::Scaling => v2k_render::ScalingMode::from_index(v).label().to_string(),
        SettingId::Bilinear | SettingId::Targetter | SettingId::AbsoluteMode => {
            ctx.string_or(12 + v as usize, if v == 0 { "Disabled" } else { "Enabled" })
        }
        // Joystick has named Absolute/Relative values. Self Righting uses
        // this text only at zero ("Off"); positive values are replaced by
        // FUN_0043B740's segmented bar in `item_bar`.
        SettingId::Joystick | SettingId::SelfRighting => {
            ctx.string_or(label_id + 1 + v as usize, if v == 0 { "Off" } else { "On" })
        }
        // Volumes: Off at 0 (string right after the label), else numeric.
        SettingId::SoundVolume | SettingId::AmbientVolume => {
            if v == 0 {
                ctx.string_or(label_id + 1, "Off")
            } else {
                format!("{v}")
            }
        }
        _ => format!("{v}"),
    };
    Some(text)
}

/// Spinner-bar values. The original Active Camera row calls FUN_0043B690
/// directly, so its value is the fill count (including a zero-filled bar).
/// Volume/sensitivity rows use the FUN_0043B740 wrapper: "Off" at 0, then
/// `filled = value - 1`. The total glyph width is the draw row's packed high
/// word, not the spinner's clamp maximum.
fn item_bar(item: &crate::menu_data::ItemDef, engine: &MenuEngine) -> Option<(u32, u32)> {
    let (setting, max) = match item.select {
        SelectAction::Spinner { setting, max } => (setting, max),
        _ => return None,
    };
    if !matches!(
        setting,
        SettingId::SoundVolume
            | SettingId::AmbientVolume
            | SettingId::Sensitivity
            | SettingId::SelfRighting
            | SettingId::ActiveCamera
    ) {
        return None;
    }
    let v = engine.settings.get(setting);
    if v == 0 && setting != SettingId::ActiveCamera {
        return None; // "Off" text instead
    }
    let total_chars = match item.draw {
        DrawKind::SettingRowNumeric { value_hint, .. }
        | DrawKind::SettingRowNamed { value_hint, .. }
            if value_hint != 0 =>
        {
            value_hint
        }
        _ => max.saturating_sub(1),
    };
    let filled = if setting == SettingId::ActiveCamera {
        v
    } else {
        v - 1
    };
    Some((filled, total_chars))
}

// ── Settings ↔ GameConfig bridge ──

/// Recover the original integer Ambient setting from its persisted normalized
/// representation. Positive values remain distinct even though retail uses
/// them all as the same CD-audio resume gate.
pub fn ambient_setting_value(config: &GameConfig) -> u32 {
    if config.ambient_enabled {
        ((config.music_volume.clamp(0.0, 1.0) * 15.0).round() as u32).min(15)
    } else {
        0
    }
}

/// Seed the engine's settings struct from the persisted GameConfig.
pub fn sync_settings_from_config(engine: &mut MenuEngine, config: &GameConfig) {
    let s = &mut engine.settings;
    s.set(
        SettingId::SoundVolume,
        if config.sound_enabled {
            (config.sfx_volume * 15.0).round() as u32
        } else {
            0
        },
    );
    s.set(SettingId::AmbientVolume, ambient_setting_value(config));
    s.set(
        SettingId::Sensitivity,
        (config.sensitivity * 15.0).round() as u32,
    );
    if let Some(idx) = config.resolution_index() {
        s.set(SettingId::Resolution, idx as u32);
    }
    s.set(SettingId::SelfRighting, config.self_righting.min(15) as u32);
    s.set(SettingId::FullScreen, config.fullscreen as u32);
    s.set(SettingId::Scaling, config.scaling.index());
    s.set(SettingId::Bilinear, config.bilinear_filtering as u32);
    s.set(SettingId::Joystick, config.joystick_mode.min(1) as u32);
    s.set(SettingId::AbsoluteMode, config.absolute_mode as u32);
    s.set(SettingId::ActiveCamera, config.active_camera.min(10) as u32);
    s.set(SettingId::Targetter, config.targetter as u32);
    s.set(SettingId::Hud, config.hud as u32);
    s.set(SettingId::Language, config.language as u32);
    // Retail's toggle: 0 = Software, 1 = hardware (OpenGL in the port).
    s.set(
        SettingId::Rendering,
        u32::from(config.renderer != v2k_render::RendererChoice::Software),
    );
}

/// Apply a changed setting back to the GameConfig.
pub fn apply_setting_to_config(config: &mut GameConfig, id: SettingId, v: u32) {
    match id {
        SettingId::SoundVolume => {
            config.sound_enabled = v > 0;
            config.sfx_volume = v as f32 / 15.0;
        }
        SettingId::AmbientVolume => {
            // Preserve the exact 0..15 menu value even though the original
            // playback path only tests whether it is zero.
            let v = v.min(15);
            config.ambient_enabled = v != 0;
            config.music_volume = v as f32 / 15.0;
        }
        SettingId::Sensitivity => config.sensitivity = v as f32 / 15.0,
        SettingId::SelfRighting => config.self_righting = v.min(15) as u8,
        SettingId::FullScreen => config.fullscreen = v == 1,
        SettingId::Scaling => config.scaling = v2k_render::ScalingMode::from_index(v),
        SettingId::Bilinear => config.bilinear_filtering = v != 0,
        SettingId::Joystick => config.joystick_mode = v.min(1) as u8,
        SettingId::AbsoluteMode => config.absolute_mode = v != 0,
        SettingId::ActiveCamera => config.active_camera = v.min(10) as u8,
        SettingId::Targetter => config.targetter = v != 0,
        SettingId::Hud => config.hud = v != 0,
        SettingId::Language => config.language = v as u8,
        SettingId::Rendering => {
            config.renderer = if v == 0 {
                v2k_render::RendererChoice::Software
            } else {
                v2k_render::RendererChoice::OpenGL
            };
        }
        SettingId::Resolution => {
            // The port spinner selects output size, independently of Low's
            // authored 320x240 artwork. NativeSettings' retail tier byte keeps
            // its separate import semantics.
            config.set_resolution_index(v as usize);
        }
        // Vibration is hidden in the PC screen and unknown settings addresses
        // have no portable representation.
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::FrontendDepthFadeMode;
    use super::{
        apply_setting_to_config, fly_model_anim_raw, fly_offsets, item_bar, ring_step_delta,
        sync_settings_from_config, window_top, Intro2PresentationStage, KlausBackdropAnimState,
        KlausEffects, KlausSequenceState, MenuCtx, MenuItem, MenuLayout, MenuPhase, MenuShell,
        MenuState, PauseSwitchAway, PostIntroStage, ShellEvent, SlotOverlayPage, FLY_LEG_SECS,
        LIST_WINDOW_ROWS,
    };
    use crate::hover::RETAIL_FRAME_DELTA_MAX_US;
    use crate::menu_data::{
        DrawKind, ItemDef, SelectAction, SettingId, MEMCARD_ACCESS, PAUSE_FULL, VIS_INGAME,
        VIS_INGAME_SP,
    };
    use crate::menu_engine::{MenuCommand, MenuEngine, MenuInput, SOUND_WHOOSH};
    use crate::opening::MENU_BACKGROUND_ZOOM_SECS;
    use crate::resource_cache::ResourceCache;
    use crate::save::{
        SaveManager, SaveSlot, SaveSlotStatus, SaveSource, SavedPlayerState, NUM_SAVE_SLOTS,
        NUM_SLOTS, SETTINGS_SLOT,
    };
    use std::env;
    use v2k_render::GameConfig;

    struct TestKlausEffects<R> {
        next_random: R,
        sounds: Vec<u32>,
    }

    impl<R: FnMut() -> u16> KlausEffects for TestKlausEffects<R> {
        fn next_random_u16(&mut self) -> u16 {
            (self.next_random)()
        }

        fn play_sound(&mut self, global_sound_id: u32) {
            self.sounds.push(global_sound_id);
        }
    }

    fn test_effects(next_random: impl FnMut() -> u16) -> TestKlausEffects<impl FnMut() -> u16> {
        TestKlausEffects {
            next_random,
            sounds: Vec::new(),
        }
    }

    fn presentation_test_shell() -> MenuShell {
        MenuShell {
            engine: MenuEngine::main_menu(),
            phase: MenuPhase::Idle,
            view: MenuState::empty(),
            prev_view: None,
            ms_open: 0.0,
            ms_since_list_scroll: 0.0,
            list_top: 0,
            list_scroll: 0.0,
            view_map: Vec::new(),
            overlay: None,
            pending_slot_overlay: None,
            klaus_sequence_state: KlausSequenceState::Idle,
            klaus_morph_progress_raw: 0,
            pending_intro_sequence_command: None,
            klaus_primary_phase_raw: 0,
            klaus_position_y_offset_raw: 0,
            klaus_position_z_raw: 0x0A00,
            frontend_background_zoom_raw: 0xFFFF,
            klaus_twitch_phase_raw: 0,
            klaus_twitch_divisor: 0x1E,
            klaus_model_state: 1,
            klaus_render_anim_state: KlausBackdropAnimState {
                morph_progress: 0,
                sway_amplitude_raw: 0xFFFF / 0x32,
                primary_phase: 0,
                twitch_phase: 0,
                twitch_strength: (0x30 - 0x1E) * 4,
                model_state: 1,
                visible: true,
            },
            transition_ready: false,
            pause_switch_away: PauseSwitchAway::Inactive,
            frontend_actor_active: true,
            from_gameplay: false,
        }
    }

    /// Advance long synthetic test durations through the same maximum frame
    /// delta accepted by production. This avoids inventing a giant callback
    /// that retail and the main loop could never issue.
    fn update_shell_for(shell: &mut MenuShell, seconds: f32) {
        assert!(seconds >= 0.0);
        let mut remaining_micros = (seconds * 1_000_000.0).round() as u32;
        while remaining_micros != 0 {
            let frame_delta_micros = remaining_micros.min(RETAIL_FRAME_DELTA_MAX_US);
            shell.update(frame_delta_micros, &mut test_effects(|| 1));
            remaining_micros -= frame_delta_micros;
        }
    }

    fn enter_frontend_load_overlay(shell: &mut MenuShell, ctx: &MenuCtx<'_>) {
        let mut effects = test_effects(|| 1);
        update_shell_for(shell, FLY_LEG_SECS);
        assert_eq!(shell.phase, MenuPhase::Idle);
        shell.input(MenuInput::Right, ctx);
        let events = shell.input(MenuInput::Select, ctx);
        assert!(events.contains(&ShellEvent::Sound(3)));

        // Consume Load's command 4 before testing the later Back command 1.
        shell.update(0, &mut effects);
        assert_eq!(effects.sounds.pop(), Some(SOUND_WHOOSH));
        update_shell_for(shell, FLY_LEG_SECS);
        update_shell_for(shell, FLY_LEG_SECS);

        assert_eq!(shell.phase, MenuPhase::Idle);
        assert!(shell.overlay.is_some());
        assert_eq!(shell.engine.depth(), 2);
    }

    #[test]
    fn normal_single_player_pause_uses_full_authored_screen_and_mask() {
        let cache = ResourceCache::new(Vec::new());
        let config = GameConfig::default();
        let ctx = MenuCtx {
            cache: &cache,
            config: &config,
            saves: None,
        };

        let shell = MenuShell::new_single_player_menu(&ctx, super::SinglePlayerMenuKind::Pause);

        assert_eq!(shell.engine.current_va(), Some(PAUSE_FULL));
        assert_eq!(shell.engine.vis_mask, VIS_INGAME | VIS_INGAME_SP);
        assert!(shell.from_gameplay);
        assert_eq!(
            shell.phase,
            MenuPhase::FlyIn {
                remaining: FLY_LEG_SECS
            }
        );
        let labels = shell
            .view()
            .items
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            labels,
            [
                "Continue",
                "Options",
                "Mission Briefing",
                "Player Status",
                "Replay this World",
                "Quit",
            ]
        );
    }

    #[test]
    fn campaign_save_uses_simple_root_and_keeps_slot_navigation() {
        let cache = ResourceCache::new(Vec::new());
        let config = GameConfig::default();
        let ctx = MenuCtx {
            cache: &cache,
            config: &config,
            saves: None,
        };
        let mut shell =
            MenuShell::new_single_player_menu(&ctx, super::SinglePlayerMenuKind::CampaignSave);
        assert_eq!(
            shell.engine.current_va(),
            Some(crate::menu_data::PAUSE_SIMPLE)
        );
        assert!(shell.from_gameplay);
        assert_eq!(shell.engine.vis_mask, VIS_INGAME | VIS_INGAME_SP);
        shell.phase = MenuPhase::Idle;
        let labels: Vec<_> = shell
            .view()
            .items
            .iter()
            .map(|item| item.label.as_str())
            .collect();
        assert_eq!(labels, ["", "Save Game", "Continue", "Options"]);
        shell.input(MenuInput::Select, &ctx);
        let overlay = shell
            .overlay
            .as_ref()
            .expect("Save Game opens slot selection");
        assert_eq!(overlay.mode, super::SlotOverlayMode::Save);
        assert_eq!(overlay.page, SlotOverlayPage::Slots);
        shell.input(MenuInput::Back, &ctx);
        assert!(shell.overlay.is_none());
        assert_eq!(
            shell.engine.current_va(),
            Some(crate::menu_data::PAUSE_SIMPLE)
        );
    }

    #[test]
    fn normal_single_player_pause_continue_resumes_at_fly_out_commit() {
        let cache = ResourceCache::new(Vec::new());
        let config = GameConfig::default();
        let ctx = MenuCtx {
            cache: &cache,
            config: &config,
            saves: None,
        };
        let mut shell = MenuShell::new_single_player_menu(&ctx, super::SinglePlayerMenuKind::Pause);
        update_shell_for(&mut shell, FLY_LEG_SECS);

        let events = shell.input(MenuInput::Select, &ctx);

        assert_eq!(
            shell.phase,
            MenuPhase::FlyOut {
                remaining: FLY_LEG_SECS
            }
        );
        assert_eq!(shell.view().selected, 0);
        assert!(events.contains(&ShellEvent::Sound(3)));
        assert!(!events.contains(&ShellEvent::ResumeGame));
        assert!(!events.contains(&ShellEvent::QuitToFrontend));
        assert_eq!(shell.take_switch_away_event(), None);

        update_shell_for(&mut shell, FLY_LEG_SECS * 0.5);
        assert_eq!(shell.take_switch_away_event(), None);
        assert_eq!(
            shell.phase,
            MenuPhase::FlyOut {
                remaining: FLY_LEG_SECS * 0.5
            }
        );

        update_shell_for(&mut shell, FLY_LEG_SECS * 0.5);
        assert_eq!(shell.phase, MenuPhase::FlyOut { remaining: 0.0 });
        assert_eq!(shell.take_switch_away_event(), Some(ShellEvent::ResumeGame));
        assert_eq!(shell.take_switch_away_event(), None);
    }

    #[test]
    fn normal_single_player_pause_root_back_is_inert() {
        let cache = ResourceCache::new(Vec::new());
        let config = GameConfig::default();
        let ctx = MenuCtx {
            cache: &cache,
            config: &config,
            saves: None,
        };
        let mut shell = MenuShell::new_single_player_menu(&ctx, super::SinglePlayerMenuKind::Pause);
        update_shell_for(&mut shell, FLY_LEG_SECS);

        let events = shell.input(MenuInput::Back, &ctx);

        assert!(events.is_empty());
        assert_eq!(shell.engine.current_va(), Some(PAUSE_FULL));
        assert_eq!(shell.engine.depth(), 1);
    }

    #[test]
    fn quit_confirmation_heading_is_decorative_not_disabled() {
        let cache = ResourceCache::new(Vec::new());
        let config = GameConfig::default();
        let ctx = MenuCtx {
            cache: &cache,
            config: &config,
            saves: None,
        };
        let mut shell = MenuShell::new_single_player_menu(&ctx, super::SinglePlayerMenuKind::Pause);
        assert!(shell.engine.push(crate::menu_data::QUIT_CONFIRM));
        shell.refresh(&ctx);

        let title = shell
            .view()
            .items
            .iter()
            .find(|item| item.label == "Quit Game?")
            .expect("quit confirmation title");
        assert!(!title.selectable);
        assert!(title.enabled);
        assert!(!title.is_interactive());

        let choices = shell
            .view()
            .items
            .iter()
            .filter(|item| item.is_interactive())
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>();
        assert_eq!(choices, ["Yes", "No"]);
    }

    #[test]
    fn frontend_root_back_is_inert_but_submenu_back_pops() {
        let cache = ResourceCache::new(Vec::new());
        let config = GameConfig::default();
        let ctx = MenuCtx {
            cache: &cache,
            config: &config,
            saves: None,
        };
        let mut shell = MenuShell::new_frontend(&ctx, false);
        update_shell_for(&mut shell, FLY_LEG_SECS);

        assert!(shell.back_is_inert());
        let root_va = shell.engine.current_va();
        let root_selection = shell.view().selected;
        assert!(shell.input(MenuInput::Back, &ctx).is_empty());
        assert_eq!(shell.engine.current_va(), root_va);
        assert_eq!(shell.engine.depth(), 1);
        assert_eq!(shell.view().selected, root_selection);
        assert_eq!(shell.phase, MenuPhase::Idle);

        assert!(shell.engine.push(crate::menu_data::DISPLAY));
        shell.refresh(&ctx);
        assert!(!shell.back_is_inert());
        let events = shell.input(MenuInput::Back, &ctx);
        assert!(events.contains(&ShellEvent::Sound(3)));
        assert_eq!(shell.engine.depth(), 1);
        assert_eq!(
            shell.phase,
            MenuPhase::FlyOut {
                remaining: FLY_LEG_SECS
            }
        );
    }

    #[test]
    fn active_camera_uses_direct_ten_cell_bar() {
        let item = ItemDef {
            visibility: 1,
            available: true,
            draw: DrawKind::SettingRowNumeric {
                string_id: 102,
                value_hint: 10,
                setting: SettingId::ActiveCamera,
            },
            select: SelectAction::Spinner {
                setting: SettingId::ActiveCamera,
                max: 10,
            },
            arg1: 10,
            arg2: 0,
            label: Some("Active Camera".to_string()),
        };
        let mut engine = MenuEngine::main_menu();

        assert_eq!(item_bar(&item, &engine), Some((6, 10)));
        engine.settings.set(SettingId::ActiveCamera, 0);
        assert_eq!(item_bar(&item, &engine), Some((0, 10)));
        engine.settings.set(SettingId::ActiveCamera, 7);
        assert_eq!(item_bar(&item, &engine), Some((7, 10)));
    }

    #[test]
    fn list_window_keeps_retail_one_row_navigation_margin() {
        // FUN_0043C400/FUN_0043C560 scroll as soon as the selection reaches
        // the bottom/top visible row, leaving one row of look-ahead whenever
        // the list bounds permit it.
        assert_eq!(window_top(0, 0, 6, 5), 0);
        assert_eq!(window_top(0, 3, 6, 5), 0);
        assert_eq!(window_top(0, 4, 6, 5), 1);
        assert_eq!(window_top(1, 4, 6, 5), 1);
        assert_eq!(window_top(1, 1, 6, 5), 0);
        assert_eq!(window_top(1, 0, 6, 5), 0);
        assert_eq!(window_top(0, 5, 6, 5), 1);
        // A list that fits entirely never scrolls.
        assert_eq!(window_top(0, 3, 4, 5), 0);
        assert_eq!(window_top(0, 0, 2, 5), 0);
    }

    #[test]
    fn list_reveal_clock_resets_only_when_the_window_moves() {
        let mut shell = presentation_test_shell();
        shell.view = MenuState {
            layout: MenuLayout::VerticalList,
            selected: 3,
            items: (0..6)
                .map(|index| MenuItem {
                    label: format!("Row {index}"),
                    model_id: None,
                    selectable: true,
                    enabled: true,
                    value_label: None,
                    bar: None,
                    icon_rgba: Vec::new(),
                    icon_width: 0,
                    icon_height: 0,
                })
                .collect(),
            list_window_rows: 5,
            typewriter_rows: true,
            retype_scrolled_rows: true,
            ..MenuState::empty()
        };
        shell.ms_open = 2_000.0;
        shell.ms_since_list_scroll = 750.0;

        // Ordinary selection inside the window does not reset DCEC8.
        shell.commit_list_window();
        assert_eq!(shell.list_top, 0);
        assert_eq!(shell.ms_since_list_scroll, 750.0);

        // Selecting the bottom visible row commits top=1 and resets DCEC8.
        // Only the new bottom row uses that clock while DCEC0 is non-zero.
        shell.view.selected = 4;
        shell.commit_list_window();
        assert_eq!(shell.list_top, 1);
        assert_eq!(shell.ms_since_list_scroll, 0.0);
        assert_eq!(shell.list_row_typewriter_elapsed_ms(5), 0.0);
        assert_eq!(shell.list_row_typewriter_elapsed_ms(4), 1_600.0);

        shell.update(100_000, &mut test_effects(|| 1));
        assert!((shell.list_row_typewriter_elapsed_ms(5) - 100.0).abs() < 0.01);

        // Once the glide has settled, the row returns to the ordinary
        // per-index DCEC4 clock rather than remaining tied to DCEC8.
        shell.update(2_000_000, &mut test_effects(|| 1));
        assert!((shell.list_row_typewriter_elapsed_ms(5) - 3_600.0).abs() < 0.01);

        // Row-group flag 0x10 suppresses only the scroll re-reveal policy;
        // the window still moves and the row keeps its ordinary DCEC4 time.
        shell.view.retype_scrolled_rows = false;
        shell.view.selected = 4;
        shell.list_top = 0;
        shell.list_scroll = 0.0;
        shell.ms_open = 2_000.0;
        shell.ms_since_list_scroll = 750.0;
        shell.commit_list_window();
        assert_eq!(shell.ms_since_list_scroll, 0.0);
        assert_eq!(shell.list_row_typewriter_elapsed_ms(5), 1_500.0);
    }

    #[test]
    fn ring_wrap_is_one_notch_not_a_spin() {
        let n = 7;
        // Ordinary forward/backward steps.
        assert_eq!(ring_step_delta(5, 6, n), 1);
        assert_eq!(ring_step_delta(3, 2, n), -1);
        // Wrap forward: last (6) -> first (0) is +1, not -6.
        assert_eq!(ring_step_delta(6, 0, n), 1);
        // Wrap backward: first (0) -> last (6) is -1, not +6.
        assert_eq!(ring_step_delta(0, 6, n), -1);
        // A genuine two-step move (e.g. skipping a hidden row) stays 2.
        assert_eq!(ring_step_delta(0, 2, n), 2);
        // No move.
        assert_eq!(ring_step_delta(3, 3, n), 0);
    }

    #[test]
    fn ring_step_delta_handles_even_rings() {
        // 6-item ring: 5 -> 0 forward is +1; 0 -> 3 is the ambiguous halfway
        // case, resolved to +3 (delta*2 == n is NOT wrapped).
        assert_eq!(ring_step_delta(5, 0, 6), 1);
        assert_eq!(ring_step_delta(0, 3, 6), 3);
    }

    #[test]
    fn frontend_load_exposes_fourteen_saves_and_the_reserved_settings_row() {
        let mut effects = test_effects(|| 1);
        let cache = ResourceCache::new(Vec::new());
        let config = GameConfig::default();
        let ctx = MenuCtx {
            cache: &cache,
            config: &config,
            saves: None,
        };
        let mut shell = MenuShell::new_frontend(&ctx, false);

        // Finish the ordinary frontend fly-in, then select the adjacent Load
        // prop (slopt) from the ring's initial New Game position.
        update_shell_for(&mut shell, FLY_LEG_SECS);
        assert_eq!(shell.phase, MenuPhase::Idle);
        shell.input(MenuInput::Right, &ctx);
        let events = shell.input(MenuInput::Select, &ctx);
        assert!(events.contains(&ShellEvent::Sound(3)));
        assert!(!events.contains(&ShellEvent::Sound(SOUND_WHOOSH)));
        assert_eq!(effects.sounds.pop(), None);
        shell.update(0, &mut effects);
        assert_eq!(effects.sounds.pop(), Some(SOUND_WHOOSH));
        assert_eq!(
            shell.phase,
            MenuPhase::FlyOut {
                remaining: FLY_LEG_SECS
            }
        );
        assert_eq!(shell.engine.current_va(), Some(MEMCARD_ACCESS));
        assert!(shell.overlay.is_none());
        assert!(shell.pending_slot_overlay.is_some());
        assert_eq!(shell.render_view().unwrap().layout, MenuLayout::Carousel);

        // Retail commits 0x4C1740 after the first leg and displays its slopt
        // access screen for the complete fly-in leg.
        update_shell_for(&mut shell, FLY_LEG_SECS);
        assert_eq!(
            shell.phase,
            MenuPhase::FlyIn {
                remaining: FLY_LEG_SECS
            }
        );
        let access = shell.render_view().unwrap();
        assert_eq!(access.layout, MenuLayout::VerticalList);
        assert_eq!(access.backdrop_model, Some(320));
        assert_eq!(access.items.len(), 1);
        assert!(shell.overlay.is_none());

        // FUN_0043B9A0 replaces that screen only when the second 0x7000 clock
        // reaches zero. The portable view contains retail ids 0..14, but
        // executable row 14 has no select callback and draws string id 80.
        update_shell_for(&mut shell, FLY_LEG_SECS);
        assert_eq!(shell.phase, MenuPhase::Idle);
        assert_eq!(shell.engine.depth(), 2);
        assert_eq!(shell.engine.current_va(), Some(MEMCARD_ACCESS));
        assert!(shell.pending_slot_overlay.is_none());
        assert!(shell.overlay.is_some());
        assert_eq!(shell.view().items.len(), NUM_SLOTS);
        assert_eq!(shell.view().selected, 0);
        for row in &shell.view().items[..NUM_SAVE_SLOTS] {
            assert!(row.selectable);
            assert!(row.enabled);
            assert!(row.is_interactive());
        }
        let settings_row = &shell.view().items[SETTINGS_SLOT];
        assert_eq!(settings_row.label, "Used for game settings");
        assert!(!settings_row.selectable);
        assert!(settings_row.enabled);

        // Navigation and the five-row viewport cover the full list, including
        // both wrap directions, while skipping the informational final row.
        for _ in 1..NUM_SAVE_SLOTS {
            shell.input(MenuInput::Down, &ctx);
        }
        assert_eq!(shell.view().selected, NUM_SAVE_SLOTS - 1);
        update_shell_for(&mut shell, 1.0);
        assert_eq!(shell.list_top(), NUM_SLOTS - LIST_WINDOW_ROWS);
        shell.input(MenuInput::Down, &ctx);
        update_shell_for(&mut shell, 1.0);
        assert_eq!(shell.view().selected, 0);
        assert_eq!(shell.list_top(), 0);
        shell.input(MenuInput::Up, &ctx);
        assert_eq!(shell.view().selected, NUM_SAVE_SLOTS - 1);
    }

    #[test]
    fn frontend_load_missing_slot_plays_error_without_dispatching_load() {
        let cache = ResourceCache::new(Vec::new());
        let config = GameConfig::default();
        let ctx = MenuCtx {
            cache: &cache,
            config: &config,
            saves: None,
        };
        let mut shell = MenuShell::new_frontend(&ctx, false);
        enter_frontend_load_overlay(&mut shell, &ctx);

        let events = shell.input(MenuInput::Select, &ctx);

        assert_eq!(events, vec![ShellEvent::Sound(2)]);
        assert!(!events
            .iter()
            .any(|event| matches!(event, ShellEvent::LoadSlot(_))));
        assert!(shell.overlay.is_some());
        assert_eq!(shell.engine.depth(), 2);
        assert_eq!(shell.phase, MenuPhase::Idle);
    }

    #[test]
    fn frontend_load_corrupt_slot_pushes_access_failed_then_returns_to_rows() {
        let cache = ResourceCache::new(Vec::new());
        let config = GameConfig::default();
        let data_dir = env::temp_dir().join("v2k_frontend_load_corrupt_feedback_test");
        let _ = std::fs::remove_dir_all(&data_dir);
        std::fs::create_dir_all(data_dir.join("saves")).unwrap();
        std::fs::write(data_dir.join("saves/slot_0.json"), b"{not-json").unwrap();
        let saves = SaveManager::load_all(&data_dir, None);
        assert_eq!(saves.slot_status(0), SaveSlotStatus::Corrupt);
        let ctx = MenuCtx {
            cache: &cache,
            config: &config,
            saves: Some(&saves),
        };
        let mut shell = MenuShell::new_frontend(&ctx, false);
        enter_frontend_load_overlay(&mut shell, &ctx);

        let events = shell.input(MenuInput::Select, &ctx);

        assert_eq!(events, vec![ShellEvent::Sound(2)]);
        assert_eq!(shell.engine.depth(), 2);
        assert_eq!(
            shell.overlay.as_ref().unwrap().page,
            SlotOverlayPage::AccessFailed
        );
        assert_eq!(shell.view().items.len(), 1);
        assert_eq!(shell.view().items[0].label, "Access Failed");
        assert_eq!(shell.view().backdrop_model, Some(320));

        assert!(shell.input(MenuInput::Select, &ctx).is_empty());
        assert_eq!(shell.overlay.as_ref().unwrap().page, SlotOverlayPage::Slots);
        assert_eq!(shell.view().items.len(), NUM_SLOTS);
        assert_eq!(shell.engine.depth(), 2);

        assert_eq!(
            shell.input(MenuInput::Select, &ctx),
            vec![ShellEvent::Sound(2)]
        );
        assert_eq!(
            shell.input(MenuInput::Back, &ctx),
            vec![ShellEvent::Sound(3)]
        );
        assert_eq!(shell.overlay.as_ref().unwrap().page, SlotOverlayPage::Slots);

        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn frontend_load_valid_slot_dispatches_without_pause_select_sound() {
        let cache = ResourceCache::new(Vec::new());
        let config = GameConfig::default();
        let data_dir = env::temp_dir().join("v2k_frontend_load_valid_sound_test");
        let _ = std::fs::remove_dir_all(&data_dir);
        let mut saves = SaveManager::load_all(&data_dir, None);
        saves
            .save_to_slot(
                0,
                SaveSlot {
                    level_id: 18,
                    level_name: "Amazon".into(),
                    timestamp: "test".into(),
                    player: Some(SavedPlayerState {
                        position_raw: [1, 2, 3],
                        velocity_raw: [0; 3],
                        heading_raw: 0x4000,
                        pitch_raw: 0,
                        roll_raw: 0,
                        health_raw: 40_000,
                    }),
                    source: SaveSource::Portable,
                    native: None,
                },
            )
            .unwrap();
        let ctx = MenuCtx {
            cache: &cache,
            config: &config,
            saves: Some(&saves),
        };
        let mut shell = MenuShell::new_frontend(&ctx, false);
        enter_frontend_load_overlay(&mut shell, &ctx);

        assert_eq!(
            shell.input(MenuInput::Select, &ctx),
            vec![ShellEvent::LoadSlot(0)]
        );

        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn frontend_load_back_uses_engine_pop_fly_and_intro_reset() {
        let mut effects = test_effects(|| 1);
        let cache = ResourceCache::new(Vec::new());
        let config = GameConfig::default();
        let ctx = MenuCtx {
            cache: &cache,
            config: &config,
            saves: None,
        };
        let mut shell = MenuShell::new_frontend(&ctx, false);
        enter_frontend_load_overlay(&mut shell, &ctx);

        let events = shell.input(MenuInput::Back, &ctx);

        assert_eq!(events, vec![ShellEvent::Sound(3)]);
        assert!(shell.overlay.is_none());
        assert_eq!(shell.engine.depth(), 1);
        assert_eq!(shell.pending_intro_sequence_command, Some(1));
        assert_eq!(
            shell.phase,
            MenuPhase::FlyOut {
                remaining: FLY_LEG_SECS
            }
        );
        assert_eq!(shell.render_view().unwrap().items.len(), NUM_SLOTS);

        shell.update(0, &mut effects);
        assert_eq!(effects.sounds.pop(), Some(SOUND_WHOOSH));
        assert_eq!(shell.klaus_sequence_state, KlausSequenceState::Idle);
        update_shell_for(&mut shell, FLY_LEG_SECS);
        assert_eq!(
            shell.phase,
            MenuPhase::FlyIn {
                remaining: FLY_LEG_SECS
            }
        );
        update_shell_for(&mut shell, FLY_LEG_SECS);
        assert_eq!(shell.phase, MenuPhase::Idle);
        assert_eq!(shell.view().layout, MenuLayout::Carousel);
    }

    #[test]
    fn pause_save_back_dismisses_overlay_without_popping_pause_root() {
        let cache = ResourceCache::new(Vec::new());
        let config = GameConfig::default();
        let ctx = MenuCtx {
            cache: &cache,
            config: &config,
            saves: None,
        };
        let mut shell = MenuShell::new_single_player_menu(&ctx, super::SinglePlayerMenuKind::Pause);
        update_shell_for(&mut shell, FLY_LEG_SECS);
        assert!(shell
            .apply_commands(&[MenuCommand::OpenSaveFlow])
            .is_empty());
        shell.refresh(&ctx);
        assert!(shell.overlay.is_some());

        let events = shell.input(MenuInput::Back, &ctx);

        assert_eq!(events, vec![ShellEvent::Sound(3)]);
        assert!(shell.overlay.is_none());
        assert_eq!(shell.engine.depth(), 1);
        assert_eq!(shell.engine.current_va(), Some(PAUSE_FULL));
        assert_eq!(shell.phase, MenuPhase::Idle);
        assert_eq!(shell.pending_intro_sequence_command, None);
    }

    #[test]
    fn pause_save_corrupt_slot_skips_probe_valid_overwrite_confirmation() {
        let cache = ResourceCache::new(Vec::new());
        let config = GameConfig::default();
        let data_dir = env::temp_dir().join("v2k_pause_save_corrupt_direct_test");
        let _ = std::fs::remove_dir_all(&data_dir);
        std::fs::create_dir_all(data_dir.join("saves")).unwrap();
        std::fs::write(data_dir.join("saves/slot_0.json"), b"{not-json").unwrap();
        let saves = SaveManager::load_all(&data_dir, None);
        assert_eq!(saves.slot_status(0), SaveSlotStatus::Corrupt);
        assert!(saves.is_occupied(0));
        assert!(!saves.requires_overwrite_confirmation(0));
        let ctx = MenuCtx {
            cache: &cache,
            config: &config,
            saves: Some(&saves),
        };
        let mut shell = MenuShell::new_single_player_menu(&ctx, super::SinglePlayerMenuKind::Pause);
        update_shell_for(&mut shell, FLY_LEG_SECS);
        shell.apply_commands(&[MenuCommand::OpenSaveFlow]);
        shell.refresh(&ctx);

        assert_eq!(
            shell.input(MenuInput::Select, &ctx),
            vec![ShellEvent::Sound(3), ShellEvent::SaveSlot(0)]
        );
        assert_eq!(
            shell.overlay.as_ref().unwrap().page,
            SlotOverlayPage::Saving
        );

        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn pause_save_occupied_slot_uses_decoded_confirmation_and_result_screens() {
        let cache = ResourceCache::new(Vec::new());
        let config = GameConfig::default();
        let data_dir = env::temp_dir().join("v2k_pause_save_overwrite_block_test");
        let _ = std::fs::remove_dir_all(&data_dir);
        let mut saves = SaveManager::load_all(&data_dir, None);
        saves
            .save_to_slot(
                0,
                SaveSlot {
                    level_id: 18,
                    level_name: "Amazon".into(),
                    timestamp: "test".into(),
                    player: Some(SavedPlayerState {
                        position_raw: [1, 2, 3],
                        velocity_raw: [0; 3],
                        heading_raw: 0x4000,
                        pitch_raw: 0,
                        roll_raw: 0,
                        health_raw: 40_000,
                    }),
                    source: SaveSource::Portable,
                    native: None,
                },
            )
            .unwrap();
        let ctx = MenuCtx {
            cache: &cache,
            config: &config,
            saves: Some(&saves),
        };
        let mut shell = MenuShell::new_single_player_menu(&ctx, super::SinglePlayerMenuKind::Pause);
        update_shell_for(&mut shell, FLY_LEG_SECS);
        assert!(shell
            .apply_commands(&[MenuCommand::OpenSaveFlow])
            .is_empty());
        shell.refresh(&ctx);

        assert_eq!(
            shell.input(MenuInput::Select, &ctx),
            vec![ShellEvent::Sound(3)]
        );
        assert_eq!(
            shell.overlay.as_ref().unwrap().page,
            SlotOverlayPage::OverwriteConfirm { yes_selected: true }
        );
        assert_eq!(
            shell
                .view()
                .items
                .iter()
                .map(|item| item.label.as_str())
                .collect::<Vec<_>>(),
            vec!["Overwrite game?", "", "Yes", "No"]
        );
        assert_eq!(shell.view().selected, 2);
        assert_eq!(shell.view().backdrop_model, Some(320));

        assert_eq!(
            shell.input(MenuInput::Down, &ctx),
            vec![ShellEvent::Sound(0)]
        );
        assert_eq!(shell.view().selected, 3);
        assert_eq!(shell.input(MenuInput::Up, &ctx), vec![ShellEvent::Sound(0)]);
        let events = shell.input(MenuInput::Select, &ctx);
        assert_eq!(events, vec![ShellEvent::Sound(3), ShellEvent::SaveSlot(0)]);
        assert_eq!(
            shell.overlay.as_ref().unwrap().page,
            SlotOverlayPage::Saving
        );
        assert_eq!(shell.view().items[0].label, "Saving game");

        shell.complete_save(true, &ctx);
        assert_eq!(
            shell.overlay.as_ref().unwrap().page,
            SlotOverlayPage::SaveSucceeded
        );
        assert_eq!(shell.view().items[0].label, "Game Saved");
        assert_eq!(
            shell.input(MenuInput::Select, &ctx),
            vec![ShellEvent::Sound(3)]
        );
        assert!(shell.overlay.is_none());
        assert_eq!(shell.engine.current_va(), Some(PAUSE_FULL));

        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn pause_save_overwrite_no_returns_directly_to_pause_root() {
        let cache = ResourceCache::new(Vec::new());
        let config = GameConfig::default();
        let data_dir = env::temp_dir().join("v2k_pause_save_overwrite_no_test");
        let _ = std::fs::remove_dir_all(&data_dir);
        let mut saves = SaveManager::load_all(&data_dir, None);
        saves
            .save_to_slot(
                0,
                SaveSlot {
                    level_id: 18,
                    level_name: "Amazon".into(),
                    timestamp: "test".into(),
                    player: Some(SavedPlayerState {
                        position_raw: [1, 2, 3],
                        velocity_raw: [0; 3],
                        heading_raw: 0x4000,
                        pitch_raw: 0,
                        roll_raw: 0,
                        health_raw: 40_000,
                    }),
                    source: SaveSource::Portable,
                    native: None,
                },
            )
            .unwrap();
        let ctx = MenuCtx {
            cache: &cache,
            config: &config,
            saves: Some(&saves),
        };
        let mut shell = MenuShell::new_single_player_menu(&ctx, super::SinglePlayerMenuKind::Pause);
        update_shell_for(&mut shell, FLY_LEG_SECS);
        shell.apply_commands(&[MenuCommand::OpenSaveFlow]);
        shell.refresh(&ctx);

        assert_eq!(
            shell.input(MenuInput::Select, &ctx),
            vec![ShellEvent::Sound(3)]
        );
        shell.input(MenuInput::Down, &ctx);
        assert_eq!(
            shell.input(MenuInput::Select, &ctx),
            vec![ShellEvent::Sound(3)]
        );
        assert!(shell.overlay.is_none());
        assert_eq!(shell.engine.depth(), 1);
        assert_eq!(shell.engine.current_va(), Some(PAUSE_FULL));

        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn fly_model_channels_reverse_at_the_stack_switch() {
        assert_eq!(
            fly_model_anim_raw(MenuPhase::FlyOut {
                remaining: FLY_LEG_SECS
            }),
            0
        );
        assert_eq!(
            fly_model_anim_raw(MenuPhase::FlyOut {
                remaining: FLY_LEG_SECS * 0.5
            }),
            0x3800
        );
        assert_eq!(
            fly_model_anim_raw(MenuPhase::FlyIn {
                remaining: FLY_LEG_SECS
            }),
            0x7000
        );
        assert_eq!(
            fly_model_anim_raw(MenuPhase::FlyIn {
                remaining: FLY_LEG_SECS * 0.5
            }),
            0x3800
        );
        assert_eq!(fly_model_anim_raw(MenuPhase::Idle), 0);
    }

    #[test]
    fn post_intro_closes_before_loading_then_opens_the_retained_actor() {
        let mut first = presentation_test_shell();
        let draws = std::cell::Cell::new(0);
        let mut next_random = test_effects(|| {
            draws.set(draws.get() + 1);
            1
        });
        first.klaus_sequence_state = KlausSequenceState::Hidden;
        first.klaus_render_anim_state.visible = false;
        first.begin_post_intro_leg(PostIntroStage::BeforeWorldLoad);
        assert!(!first.frontend_branding_visible());
        // Command 3 publishes FFFF immediately, then subtracts this frame's
        // delta. The selector runs before C090 and cannot finish this frame.
        first.update(20_000, &mut next_random);
        assert_eq!(first.menu_backdrop_anim_state().morph_progress, 0xFFFF);
        assert!(first.menu_backdrop_anim_state().visible);
        assert_eq!(first.klaus_morph_progress_raw, 0xFFFF - 1_250);
        assert!(!first.take_transition_ready());
        for _ in 2..=53 {
            first.update(20_000, &mut next_random);
            assert!(!first.take_transition_ready());
        }
        assert_eq!(first.klaus_morph_progress_raw, 0);
        first.update(20_000, &mut next_random);
        assert!(first.take_transition_ready());
        assert!(!first.take_transition_ready());

        let primary_before_second_leg = first.klaus_primary_phase_raw;
        let twitch_before_second_leg = first.klaus_twitch_phase_raw;
        assert_eq!(draws.get(), 54);
        first.begin_post_intro_leg(PostIntroStage::AfterWorldLoad);
        assert_eq!(
            first.menu_backdrop_anim_state().primary_phase,
            primary_before_second_leg as u16
        );
        assert_eq!(
            first.menu_backdrop_anim_state().twitch_phase,
            twitch_before_second_leg as u16
        );
        assert_eq!(first.menu_backdrop_anim_state().morph_progress, 0);
        assert_eq!(first.klaus_morph_progress_raw, 7_812);
        assert!(!first.take_transition_ready());
        assert_eq!(draws.get(), 54);
        for _ in 55..=101 {
            first.update(20_000, &mut next_random);
            assert!(!first.take_transition_ready());
        }
        assert!(!first.menu_backdrop_anim_state().visible);
        first.update(20_000, &mut next_random);
        assert!(first.take_transition_ready());
        assert_eq!(draws.get(), 54, "Opening and Hidden consume no RNG");

        first.phase = MenuPhase::GameStart {
            remaining: FLY_LEG_SECS,
        };
        assert!(!first.frontend_branding_visible());
        first.phase = MenuPhase::Intro { remaining: 1.0 };
        assert!(first.frontend_branding_visible());
    }

    #[test]
    fn frontend_depth_fade_requires_exact_ready_status_not_any_nonzero_status() {
        let mut shell = presentation_test_shell();
        for state in [
            KlausSequenceState::Idle,
            KlausSequenceState::Selected,
            KlausSequenceState::Opening,
            KlausSequenceState::FlyOff,
            KlausSequenceState::Hidden,
        ] {
            shell.klaus_sequence_state = state;
            for progress in [0, 1, 0xFFFF] {
                shell.klaus_morph_progress_raw = progress;
                let expected = if matches!(
                    state,
                    KlausSequenceState::Idle | KlausSequenceState::Opening
                ) && progress == 0
                {
                    FrontendDepthFadeMode::RingPulse
                } else {
                    FrontendDepthFadeMode::Receded
                };
                assert_eq!(
                    shell.frontend_depth_fade_mode(),
                    expected,
                    "{state:?}, progress={progress}"
                );
            }
        }
        shell.klaus_sequence_state = KlausSequenceState::Idle;
        shell.klaus_morph_progress_raw = 0;
        shell.apply_commands(&[MenuCommand::IntroSequenceCommand(4)]);
        assert_eq!(
            shell.frontend_depth_fade_mode(),
            FrontendDepthFadeMode::RingPulse
        );
        shell.update(20_000, &mut test_effects(|| 1));
        assert_eq!(
            shell.frontend_depth_fade_mode(),
            FrontendDepthFadeMode::Receded
        );
        shell.apply_commands(&[MenuCommand::IntroSequenceCommand(1)]);
        assert_eq!(
            shell.frontend_depth_fade_mode(),
            FrontendDepthFadeMode::Receded
        );
        shell.update(20_000, &mut test_effects(|| 1));
        assert_eq!(
            shell.frontend_depth_fade_mode(),
            FrontendDepthFadeMode::RingPulse
        );
    }

    #[test]
    fn klaus_intro_sequence_reset_whoosh_depends_on_selection_state() {
        let mut effects = test_effects(|| 1);
        let mut shell = presentation_test_shell();

        assert_eq!(shell.menu_backdrop_depth_raw(), 0xA00 as f32);
        shell.engine.settings.set(SettingId::ActiveCamera, 0);
        assert_eq!(shell.menu_backdrop_depth_raw(), 0xA00 as f32);
        shell.engine.settings.set(SettingId::ActiveCamera, 10);
        assert_eq!(shell.menu_backdrop_depth_raw(), 0xA00 as f32);

        assert!(shell
            .apply_commands(&[MenuCommand::IntroSequenceCommand(1)])
            .is_empty());
        assert!(shell
            .apply_commands(&[MenuCommand::IntroSequenceCommand(4)])
            .is_empty());
        shell.update(0, &mut effects);
        assert_eq!(effects.sounds.pop(), Some(SOUND_WHOOSH));
        update_shell_for(&mut shell, 0.7);
        assert_eq!(shell.menu_backdrop_depth_raw(), 0xA00 as f32);
        update_shell_for(&mut shell, 0.1);
        assert!(shell.menu_backdrop_depth_raw() > 0xA00 as f32);
        for _ in 0..120 {
            update_shell_for(&mut shell, 1.0 / 60.0);
        }
        assert!((shell.menu_backdrop_depth_raw() - 0x1400 as f32).abs() < 1.0);
        assert!(
            (shell.menu_billboard_pose().scale_raw() as f32 / 0x20500 as f32 - 0.69).abs() < 0.02
        );
        assert!(shell
            .apply_commands(&[MenuCommand::IntroSequenceCommand(1)])
            .is_empty());
        shell.update(0, &mut effects);
        assert_eq!(effects.sounds.pop(), Some(SOUND_WHOOSH));
        shell.update(0, &mut effects);
        assert_eq!(shell.menu_backdrop_anim_state().primary_phase, 1);
        for _ in 0..360 {
            update_shell_for(&mut shell, 1.0 / 60.0);
        }
        assert_eq!(shell.menu_backdrop_depth_raw(), 0xA00 as f32);
        assert_eq!(shell.menu_billboard_pose().scale_raw(), 132349);
        assert!(shell
            .apply_commands(&[MenuCommand::IntroSequenceCommand(1)])
            .is_empty());
        shell.update(0, &mut effects);
        assert_eq!(effects.sounds.pop(), None);

        // New Game retains the outgoing ring for the complete 0x7000 fly leg.
        // Command 5's independent background zoom reaches zero sooner and
        // must not arm the Intro2 load by itself.
        let events = shell.apply_commands(&[MenuCommand::StartGame]);
        assert!(events.is_empty());
        assert_eq!(
            shell.phase,
            MenuPhase::GameStart {
                remaining: FLY_LEG_SECS
            }
        );
        assert!(shell.render_view().is_some());
        assert!(!shell.frontend_branding_visible());
        assert_eq!(shell.menu_backdrop_anim_state().morph_progress, 0);
        assert_eq!(shell.menu_billboard_pose().scale_raw(), 132349);
        update_shell_for(&mut shell, MENU_BACKGROUND_ZOOM_SECS * 0.5);
        assert_eq!(shell.menu_backdrop_anim_state().morph_progress, 0);
        assert!(shell.menu_billboard_pose().scale_raw() < 0x20500 * 51 / 100);
        assert_eq!(shell.menu_backdrop_anim_state().model_state, 1);
        let (selected, others) = fly_offsets(shell.phase);
        assert!(selected < 0.0);
        assert!(others > 0.0);
        update_shell_for(&mut shell, MENU_BACKGROUND_ZOOM_SECS * 0.5);
        assert!(!shell.take_transition_ready());
        assert!(shell.menu_billboard_pose().scale_raw() < 0x20500 / 1000);
        update_shell_for(&mut shell, FLY_LEG_SECS - MENU_BACKGROUND_ZOOM_SECS - 0.001);
        assert!(!shell.take_transition_ready());
        update_shell_for(&mut shell, 0.002);
        assert!(shell.take_transition_ready());
        assert!(!shell.take_transition_ready());
    }

    #[test]
    fn klaus_render_pose_precedes_command_phase_and_twitch_updates() {
        let mut effects = test_effects(|| 1);
        let mut shell = presentation_test_shell();
        let initial = shell.menu_backdrop_anim_state();
        assert_eq!(initial.primary_phase, 0);
        assert_eq!(initial.model_state, 1);

        assert!(shell
            .apply_commands(&[MenuCommand::IntroSequenceCommand(4)])
            .is_empty());
        assert_eq!(effects.sounds.pop(), None);

        // C090 calls C610/C710 before consuming command 4 or advancing its
        // phases. The first rendered pose therefore remains the old phase 0,
        // while the live state is ready for the following callback.
        shell.update(20_000, &mut effects);
        assert_eq!(shell.menu_backdrop_anim_state(), initial);
        assert_eq!(shell.klaus_primary_phase_raw, 626);
        assert_eq!(effects.sounds.pop(), Some(SOUND_WHOOSH));

        shell.update(20_000, &mut effects);
        assert_eq!(shell.menu_backdrop_anim_state().primary_phase, 626);

        shell.klaus_twitch_phase_raw = 100;
        shell.klaus_twitch_divisor = 0x20;
        shell.update(20_000, &mut effects);
        assert_eq!(shell.menu_backdrop_anim_state().twitch_phase, 100);
        assert_eq!(shell.klaus_twitch_phase_raw, 725);

        assert!(shell.apply_commands(&[MenuCommand::StartGame]).is_empty());
        shell.update(20_000, &mut effects);
        assert_eq!(shell.menu_backdrop_anim_state().model_state, 1);
        assert_eq!(shell.klaus_model_state, 1);
        shell.update(20_000, &mut effects);
        assert_eq!(shell.menu_backdrop_anim_state().model_state, 1);
        // FUN_0042CF40 selects model state zero at the load commit, rather
        // than C090's earlier command-5 fly-off.
        shell.begin_intro_reveal();
        assert_eq!(shell.menu_backdrop_anim_state().model_state, 0);
    }

    #[test]
    fn klaus_intro_sequence_motion_matches_c_integer_truncation() {
        let mut shell = presentation_test_shell();
        shell.frontend_actor_active = false;
        shell.klaus_sequence_state = KlausSequenceState::Selected;
        shell.klaus_primary_phase_raw = 1;

        for frame in 1..=80 {
            shell.update(20_000, &mut test_effects(|| 1));
            match frame {
                39 => assert_eq!(
                    (
                        shell.klaus_primary_phase_raw,
                        shell.klaus_position_y_offset_raw,
                        shell.klaus_position_z_raw,
                    ),
                    (24_376, 0, 2_560)
                ),
                // `FUN_0042C090` computes `(q31_step * 4) / 3` as C integer
                // division. At this frame q31_step is 116, so retail truncates
                // 464/3 to 154: Y gains 77 and Z gains 154. Do not replace
                // these with PowerShell `[int]` casts, which round to 155.
                40 => assert_eq!(
                    (
                        shell.klaus_primary_phase_raw,
                        shell.klaus_position_y_offset_raw,
                        shell.klaus_position_z_raw,
                    ),
                    (25_001, 77, 2_714)
                ),
                80 => assert_eq!(
                    (
                        shell.klaus_primary_phase_raw,
                        shell.klaus_position_y_offset_raw,
                        shell.klaus_position_z_raw,
                    ),
                    (50_001, 1_284, 5_120)
                ),
                _ => {}
            }
        }

        shell.klaus_sequence_state = KlausSequenceState::Idle;
        for frame in 1..=25 {
            shell.update(20_000, &mut test_effects(|| 1));
            let expected = match frame {
                1 => Some((1_171, 4_947)),
                10 => Some((474, 3_656)),
                20 => Some((109, 2_661)),
                25 => Some((10, 2_560)),
                _ => None,
            };
            if let Some((y, z)) = expected {
                assert_eq!(
                    (
                        shell.klaus_position_y_offset_raw,
                        shell.klaus_position_z_raw,
                    ),
                    (y, z)
                );
            }
        }
    }

    #[test]
    fn game_start_zoom_uses_retail_integer_eighth_microseconds() {
        let mut shell = presentation_test_shell();
        shell.apply_commands(&[MenuCommand::StartGame]);

        shell.update(20_000, &mut test_effects(|| 1));
        assert_eq!(shell.frontend_background_zoom_raw, 0xFFFF - 2_500);
        shell.update(125_000, &mut test_effects(|| 1));
        assert_eq!(shell.frontend_background_zoom_raw, 0xFFFF - 2_500 - 15_625);

        update_shell_for(&mut shell, 1.0);
        assert_eq!(shell.frontend_background_zoom_raw, 0);
    }

    #[test]
    fn intro2_reveal_follows_callback_overflow_without_restarting_the_clock() {
        let mut shell = presentation_test_shell();
        shell.begin_intro_reveal();
        assert_eq!(shell.klaus_morph_progress_raw, 7_812);
        assert_eq!(shell.menu_backdrop_anim_state().morph_progress, 0);
        assert!(shell.render_view().is_none());
        assert!(!shell.frontend_branding_visible());
        let mut stage = Intro2PresentationStage::Covered { shell };
        let mut elapsed = 0.0;
        // At a fixed 20ms callback partition the initialized opening needs
        // 47 updates; a recorded 44-tick landmark is not an authored cutoff.
        for _ in 1..=46 {
            stage.advance(&mut elapsed, 20_000, &mut test_effects(|| 1));
            assert!(matches!(stage, Intro2PresentationStage::Covered { .. }));
        }
        let previous = stage.advance(&mut elapsed, 20_000, &mut test_effects(|| 1));
        assert!(matches!(
            stage,
            Intro2PresentationStage::Visible { shell: Some(_) }
        ));
        assert!((previous - 0.92).abs() < 0.0001);
        assert!((elapsed - 0.94).abs() < 0.0001);
        let previous = stage.advance(&mut elapsed, 20_000, &mut test_effects(|| 1));
        assert!((previous - 0.94).abs() < 0.0001);
        assert!((elapsed - 0.96).abs() < 0.0001);
    }
    #[test]
    fn intro2_keeps_advancing_and_transfers_the_persistent_klaus_state() {
        let mut shell = presentation_test_shell();
        shell.klaus_primary_phase_raw = 7_000;
        shell.klaus_twitch_phase_raw = 900;
        shell.begin_intro_reveal();

        let mut stage = Intro2PresentationStage::Covered { shell };
        let mut elapsed = 0.0;
        let mut next_random = test_effects(|| panic!("Opening and Hidden consume no RNG"));
        for _ in 1..=47 {
            stage.advance(&mut elapsed, 20_000, &mut next_random);
        }
        let phase_at_reveal = match &stage {
            Intro2PresentationStage::Visible { shell: Some(shell) } => {
                shell.klaus_primary_phase_raw
            }
            _ => panic!("frontend-started Intro2 must retain Klaus after reveal"),
        };

        stage.advance(&mut elapsed, 20_000, &mut next_random);
        let mut shell = stage
            .take_shell()
            .expect("frontend-started Intro2 must transfer Klaus to the handoff");
        assert!(shell.klaus_primary_phase_raw > phase_at_reveal);

        let phase_before_bridge = shell.klaus_primary_phase_raw;
        let twitch_before_bridge = shell.klaus_twitch_phase_raw;
        shell.begin_post_intro_leg(PostIntroStage::BeforeWorldLoad);
        assert_eq!(shell.klaus_primary_phase_raw, phase_before_bridge);
        assert_eq!(shell.klaus_twitch_phase_raw, twitch_before_bridge);
        assert_eq!(shell.klaus_position_z_raw, 0x0A00);
        assert_eq!(shell.frontend_background_zoom_raw, 0);
        assert_eq!(shell.klaus_model_state, 0);
    }

    #[test]
    fn klaus_morph_observes_shift_threshold_visibility_and_publication_order() {
        let mut shell = presentation_test_shell();
        shell.klaus_morph_progress_raw = 0xFFF0;
        shell.klaus_sequence_state = KlausSequenceState::Opening;
        shell.klaus_position_y_offset_raw = 321;
        shell.klaus_position_z_raw = 0x1400;
        let mut next_random = test_effects(|| panic!("Opening and Hidden consume no RNG"));

        shell.update_klaus(15, &mut next_random); // each callback truncates dt >> 4
        assert_eq!(shell.klaus_morph_progress_raw, 0xFFF0);
        shell.update_klaus(240, &mut next_random);
        assert_eq!(shell.klaus_morph_progress_raw, 0xFFFF);
        assert_eq!(shell.menu_backdrop_anim_state().morph_progress, 0xFFF0);
        assert!(shell.menu_backdrop_anim_state().visible);
        shell.update_klaus(0, &mut next_random);
        assert_eq!(shell.menu_backdrop_anim_state().morph_progress, 0xFFFF);
        assert_eq!(shell.menu_backdrop_anim_state().sway_amplitude_raw, 0);
        assert!(shell.menu_backdrop_anim_state().visible);
        shell.update_klaus(16, &mut next_random);
        assert_eq!(shell.menu_backdrop_anim_state().morph_progress, 0xFFFF);
        assert!(!shell.menu_backdrop_anim_state().visible);
        assert_eq!(shell.klaus_morph_progress_raw, 0);
        assert_eq!(shell.klaus_sequence_state, KlausSequenceState::Hidden);
        shell.update_klaus(20_000, &mut next_random);
        assert!(!shell.menu_backdrop_anim_state().visible);
        assert_eq!(shell.klaus_position_y_offset_raw, 321);
        assert_eq!(shell.klaus_position_z_raw, 0x1400);

        shell.pending_intro_sequence_command = Some(3);
        shell.update_klaus(20_000, &mut test_effects(|| 1));
        let first_closing_frame = shell.menu_backdrop_anim_state();
        assert!(first_closing_frame.visible);
        assert_eq!(first_closing_frame.morph_progress, 0xFFFF);
        assert_eq!(first_closing_frame.sway_amplitude_raw, 0xFFFF / 0x32);
        shell.update_klaus(20_000, &mut test_effects(|| 1));
        assert_eq!(
            shell.menu_backdrop_anim_state().morph_progress,
            0xFFFF - 1_250
        );
        assert_eq!(shell.menu_backdrop_anim_state().sway_amplitude_raw, 25);
    }

    #[test]
    fn klaus_random_cadence_counts_callbacks_including_zero_delta() {
        let mut batched = presentation_test_shell();
        let mut partitioned = presentation_test_shell();
        let mut batched_draws = 0;
        let mut partitioned_draws = 0;

        batched.update(
            40_000,
            &mut test_effects(|| {
                batched_draws += 1;
                1
            }),
        );
        for _ in 0..2 {
            partitioned.update(
                20_000,
                &mut test_effects(|| {
                    partitioned_draws += 1;
                    1
                }),
            );
        }
        assert_eq!(batched_draws, 1);
        assert_eq!(partitioned_draws, 2);

        batched.update(
            0,
            &mut test_effects(|| {
                batched_draws += 1;
                1
            }),
        );
        assert_eq!(batched_draws, 2);
    }

    #[test]
    fn klaus_random_draw_admission_uses_the_callback_state() {
        for state in [
            KlausSequenceState::Idle,
            KlausSequenceState::Selected,
            KlausSequenceState::FlyOff,
            KlausSequenceState::Opening,
            KlausSequenceState::Hidden,
        ] {
            for phase in [0, 1_000] {
                for elapsed in [0, 20_000, 125_000] {
                    let mut shell = presentation_test_shell();
                    shell.klaus_sequence_state = state;
                    shell.klaus_twitch_phase_raw = phase;
                    let mut draws = 0;
                    shell.update(
                        elapsed,
                        &mut test_effects(|| {
                            draws += 1;
                            1 // a failed gate, regardless of the phase
                        }),
                    );
                    let expected = match state {
                        KlausSequenceState::Idle
                        | KlausSequenceState::Selected
                        | KlausSequenceState::FlyOff => 1,
                        KlausSequenceState::Opening | KlausSequenceState::Hidden => 0,
                    };
                    assert_eq!(draws, expected, "{state:?}, phase={phase}, dt={elapsed}");
                }
            }
        }
    }

    #[test]
    fn klaus_twitch_divisor_consumes_a_second_word_only_for_a_phase_zero_gate() {
        for state in [
            KlausSequenceState::Idle,
            KlausSequenceState::Selected,
            KlausSequenceState::FlyOff,
        ] {
            for (sample, rate, divisor) in [(0, 0, 32), (0x20, 0xF800, 63), (0xFFE0, 0xFFFF, 63)] {
                let mut shell = presentation_test_shell();
                shell.klaus_sequence_state = state;
                let mut words = [sample, rate].into_iter();
                shell.update(
                    0,
                    &mut test_effects(|| words.next().expect("only two RNG draws")),
                );
                assert_eq!(words.next(), None);
                assert_eq!(shell.klaus_twitch_phase_raw, 1);
                assert_eq!(shell.klaus_twitch_divisor, divisor);
                assert_eq!(shell.menu_backdrop_anim_state().twitch_phase, 0);
            }
            for (phase, sample) in [(0, 1), (1, 0), (0xFFFF, 0x20)] {
                let mut shell = presentation_test_shell();
                shell.klaus_sequence_state = state;
                shell.klaus_twitch_phase_raw = phase;
                let mut words = [sample].into_iter();
                shell.update(
                    0,
                    &mut test_effects(|| words.next().expect("only one RNG draw")),
                );
                assert_eq!(words.next(), None);
                assert_eq!(shell.klaus_twitch_phase_raw, phase);
                assert_eq!(shell.klaus_twitch_divisor, 30);
            }
        }
    }

    #[test]
    fn klaus_twitch_gate_observes_phase_overflow_in_the_same_callback() {
        let mut shell = presentation_test_shell();
        shell.klaus_twitch_phase_raw = 0xFFFE;
        shell.klaus_twitch_divisor = 32;
        let mut words = [0, 0x8800].into_iter();
        shell.update(
            64,
            &mut test_effects(|| words.next().expect("only two RNG draws")),
        );
        assert_eq!(words.next(), None);
        assert_eq!(shell.klaus_twitch_phase_raw, 1);
        assert_eq!(shell.klaus_twitch_divisor, 49);
        assert_eq!(shell.menu_backdrop_anim_state().twitch_phase, 0xFFFE);
    }

    #[test]
    fn menu_and_intro2_presentation_consume_the_callers_process_rng_in_place() {
        use crate::retail_rng::retail_random_u16;
        use crate::world_fx::WorldFx;

        let mut world_fx = WorldFx::new();
        let mut expected_state = 0;
        assert_eq!(
            world_fx.next_shared_retail_random_u16(),
            retail_random_u16(&mut expected_state)
        );

        let mut shell = presentation_test_shell();
        shell.klaus_twitch_phase_raw = 1; // busy phase still consumes one word
        shell.update(
            0,
            &mut test_effects(|| world_fx.next_shared_retail_random_u16()),
        );
        retail_random_u16(&mut expected_state);
        assert_eq!(
            world_fx.next_shared_retail_random_u16(),
            retail_random_u16(&mut expected_state)
        );

        let mut stage = Intro2PresentationStage::Visible { shell: Some(shell) };
        let mut elapsed = 0.0;
        stage.advance(
            &mut elapsed,
            0,
            &mut test_effects(|| world_fx.next_shared_retail_random_u16()),
        );
        retail_random_u16(&mut expected_state);
        assert_eq!(
            world_fx.next_shared_retail_random_u16(),
            retail_random_u16(&mut expected_state)
        );

        let _ = stage.take_shell();
        stage.advance(
            &mut elapsed,
            20_000,
            &mut test_effects(|| panic!("shell-less Intro2 has no Klaus callback")),
        );
    }

    #[v2k_test_support::retail_test]
    fn klaus_resolves_the_real_whoosh_before_twitch_without_an_audio_device() {
        use v2k_formats::anim_sound::SoundPool;
        use v2k_render::sound::{
            resolve_sound_playback, ResolvedSoundPlayback, SoundPlaybackRequest,
        };

        let data_root = v2k_test_support::retail_dir();
        let preload_path = data_root.join("PRELOAD.DAT");
        let level3_path = data_root.join("Overlay/1X3XX.OVL");
        let preload =
            v2k_formats::preload::PreloadDat::parse(&std::fs::read(preload_path).unwrap()).unwrap();
        let level2 = v2k_formats::sections::parse_anim_sound(
            &preload
                .embedded_ovls
                .iter()
                .find(|ovl| ovl.index == 2)
                .unwrap()
                .ovl,
        )
        .unwrap();
        let level3_ovl =
            v2k_formats::ovl::OvlFile::parse(&std::fs::read(level3_path).unwrap()).unwrap();
        let level3 = v2k_formats::sections::parse_anim_sound(&level3_ovl).unwrap();
        let pool = SoundPool::from_tables(&[&level2, &level3]);
        let whoosh = pool.resolve(SOUND_WHOOSH as usize).unwrap();
        assert_eq!(whoosh.resolution_chain, [87, 23]);
        assert_eq!(whoosh.alias_hops[0].frequency_multiplier_16_16, 0xB333);
        assert_eq!(whoosh.alias_hops[0].frequency_variance_16_16, 0x3333);

        #[derive(Debug, PartialEq, Eq)]
        enum Effect {
            Sound(u32),
            Random(u16),
        }
        struct NativeEffects<'a> {
            pool: &'a SoundPool<'a>,
            words: std::vec::IntoIter<u16>,
            trace: Vec<Effect>,
            resolved: Vec<ResolvedSoundPlayback>,
        }
        impl KlausEffects for NativeEffects<'_> {
            fn next_random_u16(&mut self) -> u16 {
                let word = self.words.next().expect("no extra sound or twitch word");
                self.trace.push(Effect::Random(word));
                word
            }

            fn play_sound(&mut self, global_sound_id: u32) {
                self.trace.push(Effect::Sound(global_sound_id));
                let pool = self.pool;
                let request = resolve_sound_playback(
                    pool,
                    SoundPlaybackRequest::centered(global_sound_id as usize),
                    &mut || self.next_random_u16(),
                )
                .unwrap();
                self.resolved.push(request);
            }
        }

        for (previous_state, command) in [
            (KlausSequenceState::Idle, 4),
            (KlausSequenceState::Selected, 1),
        ] {
            let mut shell = presentation_test_shell();
            shell.klaus_sequence_state = previous_state;
            shell.pending_intro_sequence_command = Some(command);
            let mut effects = NativeEffects {
                pool: &pool,
                words: vec![0x8000, 0, 0xF800].into_iter(),
                trace: Vec::new(),
                resolved: Vec::new(),
            };
            shell.update(0, &mut effects);
            assert_eq!(effects.words.next(), None);
            assert_eq!(
                effects.trace,
                [
                    Effect::Sound(87),
                    Effect::Random(0x8000),
                    Effect::Random(0),
                    Effect::Random(0xF800),
                ]
            );
            assert_eq!(effects.resolved[0].pcm_global_id, 23);
            assert_eq!(effects.resolved[0].frequency_q16, 50_462);
            assert_eq!(effects.resolved[0].frequency_hz(), 16_978);
            assert_eq!(shell.klaus_twitch_phase_raw, 1);
            assert_eq!(shell.klaus_twitch_divisor, 63);
        }
    }

    #[test]
    fn command_two_prime_enters_opening_without_sampling_any_previous_state() {
        for state in [
            KlausSequenceState::Idle,
            KlausSequenceState::Selected,
            KlausSequenceState::FlyOff,
            KlausSequenceState::Opening,
            KlausSequenceState::Hidden,
        ] {
            for stage in [None, Some(PostIntroStage::AfterWorldLoad)] {
                let mut shell = presentation_test_shell();
                shell.klaus_sequence_state = state;
                shell.klaus_twitch_phase_raw = 0;
                shell.klaus_morph_progress_raw = 123;
                // The prime's RNG callback fails immediately if reached.
                if let Some(stage) = stage {
                    shell.begin_post_intro_leg(stage);
                } else {
                    shell.begin_intro_reveal();
                }
                assert_eq!(shell.klaus_sequence_state, KlausSequenceState::Opening);
                assert_eq!(shell.klaus_morph_progress_raw, 7_812);
                assert_eq!(shell.menu_backdrop_anim_state().morph_progress, 123);
                assert_eq!(shell.klaus_twitch_phase_raw, 0);
            }
        }
    }

    #[test]
    fn returned_frontend_advances_klaus_but_pause_shell_does_not() {
        let cache = ResourceCache::new(Vec::new());
        let config = GameConfig::default();
        let ctx = MenuCtx {
            cache: &cache,
            config: &config,
            saves: None,
        };

        let mut returned_frontend = MenuShell::new_frontend(&ctx, false);
        returned_frontend.from_gameplay = true;
        let mut returned_draws = 0;
        returned_frontend.update(
            0,
            &mut test_effects(|| {
                returned_draws += 1;
                1
            }),
        );
        assert_eq!(returned_draws, 1);

        let mut pause = MenuShell::new_single_player_menu(&ctx, super::SinglePlayerMenuKind::Pause);
        pause.update(
            125_000,
            &mut test_effects(|| panic!("pause has no frontend Klaus callback")),
        );
    }

    #[test]
    fn ambient_setting_round_trips_every_raw_retail_value() {
        for raw in 0..=15 {
            let mut config = GameConfig::default();
            apply_setting_to_config(&mut config, SettingId::AmbientVolume, raw);

            assert_eq!(config.ambient_enabled, raw != 0);
            assert!((config.music_volume - raw as f32 / 15.0).abs() < f32::EPSILON);

            let mut engine = MenuEngine::main_menu();
            sync_settings_from_config(&mut engine, &config);
            assert_eq!(engine.settings.get(SettingId::AmbientVolume), raw);
        }

        let mut contradictory = GameConfig {
            ambient_enabled: true,
            music_volume: 0.0,
            ..GameConfig::default()
        };
        assert_eq!(super::ambient_setting_value(&contradictory), 0);
        contradictory.ambient_enabled = false;
        contradictory.music_volume = 1.0;
        assert_eq!(super::ambient_setting_value(&contradictory), 0);
    }

    #[test]
    fn rendering_row_toggles_between_software_and_opengl() {
        for (renderer, row) in [
            (v2k_render::RendererChoice::Auto, 1),
            (v2k_render::RendererChoice::OpenGL, 1),
            (v2k_render::RendererChoice::Software, 0),
            (v2k_render::RendererChoice::Wgpu, 1),
        ] {
            let mut config = GameConfig {
                renderer,
                ..GameConfig::default()
            };
            let mut engine = MenuEngine::main_menu();
            sync_settings_from_config(&mut engine, &config);
            assert_eq!(engine.settings.get(SettingId::Rendering), row);
            apply_setting_to_config(&mut config, SettingId::Rendering, 0);
            assert_eq!(config.renderer, v2k_render::RendererChoice::Software);
            apply_setting_to_config(&mut config, SettingId::Rendering, 1);
            assert_eq!(config.renderer, v2k_render::RendererChoice::OpenGL);
        }
    }

    #[test]
    fn output_resolution_keeps_the_explicit_artwork_tier() {
        for detail in [
            v2k_render::config::GraphicsDetail::Low,
            v2k_render::config::GraphicsDetail::High,
        ] {
            let mut config = GameConfig {
                detail,
                ..GameConfig::default()
            };
            for (index, &(width, height)) in v2k_render::config::RESOLUTIONS.iter().enumerate() {
                apply_setting_to_config(&mut config, SettingId::Resolution, index as u32);
                assert_eq!((config.width, config.height), (width, height));
                assert_eq!(config.detail, detail);
            }
        }
    }
}
