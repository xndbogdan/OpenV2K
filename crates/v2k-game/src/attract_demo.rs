//! The idle frontend's attract demo.
//!
//! `FUN_0042B5D0` counts the frontend down (`004DB21C`, reloaded from
//! `004DB204`); the frontend's Return/Space, Escape and arrow handlers set both
//! to 60 s on each edge. At zero both become 10 s and one screen pops
//! (`FUN_0043A9A0`): from the ring the game starts in mode 4, or in mode 1 when
//! the random word's low three bits are zero, with Klaus's command 5; one
//! screen deeper Klaus gets command 1. Mode 1 plays Intro2 in phase 1 and
//! returns to the frontend at its end. Mode 4 (`FUN_0044F650`) picks a row of
//! the table at `004D0CC8` and plays its world in phase 4, where
//! `FUN_004503C0` roams the camera and ends the world after `0xB22` ticks.

use crate::entity::EntityManager;
use crate::resource_cache::ResourceCache;
use crate::world_complete_results::fun_0042e210_entity_is_live;

/// `FUN_0042A890` and the frontend's key handlers: `004DB204` and `004DB21C`.
pub const IDLE_PERIOD_MS: i32 = 60_000;
/// `FUN_0042B5D0` on expiry.
pub const IDLE_RELOAD_MS: i32 = 10_000;
/// `FUN_004503C0` sets session `+290` once the 50-Hz clock passes this in phase 4.
pub const ATTRACT_WORLD_TICKS: u32 = 0xB22;
/// `FUN_00452CB0` draws phase 4's records from this global string, not the world's.
pub const ATTRACT_RECORDS_STRING: usize = 0x77;
/// `FUN_00450390`: logical world 11 builds no player in phase 4.
pub const PLAYERLESS_WORLD: u8 = 11;
/// Logical world to overlay id.
pub const LOGICAL_WORLD_OVERLAY_OFFSET: u32 = 12;

/// The frontend's countdown (`004DB21C`) and its reload (`004DB204`). Both
/// are process globals, so the reload survives the frontend's teardown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdleCountdown {
    remaining_ms: i32,
    reload_ms: i32,
}

impl Default for IdleCountdown {
    /// `FUN_0042A890` at startup, then the first `FUN_0042B230`.
    fn default() -> Self {
        Self {
            remaining_ms: IDLE_PERIOD_MS,
            reload_ms: IDLE_PERIOD_MS,
        }
    }
}

impl IdleCountdown {
    /// `FUN_0042B230`: every frontend entry restarts from the reload.
    pub fn enter_frontend(&mut self) {
        self.remaining_ms = self.reload_ms;
    }

    /// `FUN_0042BAA0`, `FUN_0042BAD0`, `FUN_0042BB30`..`FUN_0042BBC0` and
    /// `FUN_0042BD90`, on a press or a release.
    pub fn key(&mut self) {
        self.remaining_ms = IDLE_PERIOD_MS;
        self.reload_ms = IDLE_PERIOD_MS;
    }

    /// One `FUN_0042B5D0` visit: the frame's microseconds, clamped to 125 ms,
    /// in whole milliseconds. It runs while nonzero, the window active
    /// (`004FE60C`) and no network game open (`004F741C`). True on expiry.
    pub fn tick(&mut self, frame_micros: u32, suspended: bool, network: bool) -> bool {
        if self.remaining_ms == 0 || suspended || network {
            return false;
        }
        self.remaining_ms -= (frame_micros.min(125_000) / 1000) as i32;
        if self.remaining_ms >= 1 {
            return false;
        }
        self.remaining_ms = IDLE_RELOAD_MS;
        self.reload_ms = IDLE_RELOAD_MS;
        true
    }
}

/// What an expired countdown does, by the screen depth `FUN_0043A9A0`
/// returns (the depth its pop starts from).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdleExpiry {
    /// At the ring: the game start, with this mode.
    Start(AttractMode),
    /// One screen pops; `klaus_command` is Klaus's command 1 from depth 2.
    Pop { klaus_command: bool },
}

impl IdleExpiry {
    /// `random` is drawn only at the ring.
    pub fn at_depth(depth: usize, random: impl FnOnce() -> u16) -> Self {
        if depth < 2 {
            Self::Start(if random() & 7 == 0 {
                AttractMode::Intro2
            } else {
                AttractMode::World
            })
        } else {
            Self::Pop {
                klaus_command: depth == 2,
            }
        }
    }
}

/// The game modes `FUN_0042B5D0` starts (`004DB224`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttractMode {
    /// Mode 1: Intro2 in phase 1.
    Intro2,
    /// Mode 4: a table world in phase 4.
    World,
}

/// One 16-byte row of `004D0CC8`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttractRow {
    /// `+00`: the logical world (`FUN_0042E3B0`).
    pub logical_world: u8,
    /// `+01`: the demo player's projectile method, written to its Sub-E
    /// runtime `+10` (a weapon selector of `004CDC08`'s table).
    pub projectile_method: u8,
    /// `+02`: written to the player type's Sub-E `+17`, its auxiliary command.
    pub auxiliary_command: u8,
    /// `+03`: written to the word the Sub-E runtime's binding `+0C` names:
    /// the weapon record's `+08` selector for that method.
    pub weapon_selector: u8,
    /// `+04`: times 1000, the player type's Sub-E cadence `+04` in µs.
    pub cadence_ms: u32,
    /// `+08`: the Sub-E runtime's sound `+20`, the method's firing sound.
    pub sound_id: u16,
    /// `+0A`, `+0E`: copied to session `+38`/`+3C`, the player's arrival.
    pub arrival_raw: [i16; 3],
}

const ROWS_RAW: [[u8; 16]; 12] = [
    [7, 14, 0, 6, 100, 0, 0, 0, 0x4E, 0, 0, 0x5F, 0, 1, 0, 0x4D],
    [
        27, 13, 0, 7, 100, 0, 0, 0, 0x4D, 0, 0, 0x73, 0, 0xFE, 0, 0x2F,
    ],
    [2, 18, 1, 0, 50, 0, 0, 0, 0x58, 0, 0, 0x42, 0, 0xFE, 0, 0x7E],
    [3, 18, 1, 0, 50, 0, 0, 0, 0x58, 0, 0, 0x2B, 0, 0xFF, 0, 0x60],
    [5, 14, 0, 6, 100, 0, 0, 0, 0x4E, 0, 0, 0xCC, 0, 2, 0, 0xC3],
    [6, 13, 0, 7, 100, 0, 0, 0, 0x4D, 0, 0, 0x96, 0, 6, 0, 0x68],
    [
        11, 13, 0, 7, 100, 0, 0, 0, 0x4D, 0, 0, 0x80, 0, 0x0F, 0, 0x80,
    ],
    [12, 12, 0, 8, 100, 0, 0, 0, 0x4C, 0, 0, 0xDB, 0, 6, 0, 0xDD],
    [
        31, 12, 0, 8, 30, 0, 0, 0, 0x4C, 0, 0, 0x49, 0, 0x0E, 0, 0x2A,
    ],
    [
        11, 13, 0, 7, 100, 0, 0, 0, 0x4D, 0, 0, 0x80, 0, 0x0F, 0, 0x80,
    ],
    [24, 14, 0, 6, 100, 0, 0, 0, 0x4E, 0, 0, 0x2C, 0, 9, 0, 0x7E],
    [26, 12, 0, 8, 100, 0, 0, 0, 0x4C, 0, 0, 0x32, 0, 0, 0, 0xCA],
];

const fn word(row: [u8; 16], at: usize) -> i16 {
    i16::from_le_bytes([row[at], row[at + 1]])
}

/// The rows of `004D0CC8`, decoded.
pub const ATTRACT_ROWS: [AttractRow; 12] = {
    let mut rows = [AttractRow {
        logical_world: 0,
        projectile_method: 0,
        auxiliary_command: 0,
        weapon_selector: 0,
        cadence_ms: 0,
        sound_id: 0,
        arrival_raw: [0; 3],
    }; 12];
    let mut i = 0;
    while i < 12 {
        let r = ROWS_RAW[i];
        rows[i] = AttractRow {
            logical_world: r[0],
            projectile_method: r[1],
            auxiliary_command: r[2],
            weapon_selector: r[3],
            cadence_ms: u32::from_le_bytes([r[4], r[5], r[6], r[7]]),
            sound_id: u16::from_le_bytes([r[8], r[9]]),
            arrival_raw: [word(r, 10), word(r, 12), word(r, 14)],
        };
        i += 1;
    }
    rows
};

impl AttractRow {
    /// `FUN_0044F650` mode 4: `(random & 0xFFFF) * 3 >> 14`.
    pub fn index(random: u16) -> usize {
        (u32::from(random) * 3 >> 14) as usize
    }

    pub fn overlay_id(&self) -> u32 {
        u32::from(self.logical_world) + LOGICAL_WORLD_OVERLAY_OFFSET
    }

    /// `FUN_004503C0`'s writes: runtime `+10` and `+20`, type 46's Sub-E
    /// `+04` (times 1000) and `+17`; `+16` becomes 1.
    pub fn craft_arming(&self) -> crate::intro2_flyer_aim::CraftEmitterArming {
        crate::intro2_flyer_aim::CraftEmitterArming {
            projectile_method: u32::from(self.projectile_method),
            sound_id: u32::from(self.sound_id),
            cadence_us: self.cadence_ms * 1000,
            auxiliary_command: self.auxiliary_command,
            weapon_var_raw: i16::from(self.weapon_selector as i8),
        }
    }

    /// `FUN_00450390` in phase 4.
    pub fn has_player(&self) -> bool {
        self.logical_world != PLAYERLESS_WORLD
    }

    /// Where the demo player stands once built: session `+38`'s position,
    /// lifted by `FUN_00451C00` to 0x200 above the ground (`FUN_0041DE60`).
    pub fn player_position_raw(&self, terrain: &v2k_formats::terrain::TerrainGrid) -> [i16; 3] {
        let [x, y, z] = self.arrival_raw;
        let floor = crate::chase_camera::terrain_height_raw(terrain, x, z).wrapping_add(0x200);
        [x, y.max(floor), z]
    }
}

/// Phase 4's camera subject (`FUN_004503C0`): a countdown in ms
/// (`004F72D8`, zero after every load) and the subject (session `+308`,
/// null at every game start).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AttractCamera {
    countdown_ms: i32,
    subject: Option<u32>,
    /// `004F72C4`'s row, which `FUN_004503C0` writes into the craft.
    row: Option<AttractRow>,
}

/// A body whose eligibility the port cannot read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttractCameraStop {
    pub entity_id: u32,
    pub entity_type: u32,
}

impl AttractCamera {
    pub fn for_row(row: AttractRow) -> Self {
        Self {
            row: Some(row),
            ..Self::default()
        }
    }

    pub fn row(&self) -> Option<AttractRow> {
        self.row
    }

    pub fn subject(&self) -> Option<u32> {
        self.subject
    }

    /// One visit's subject choice. `player` is the player's body, if any.
    pub fn visit(
        &mut self,
        entities: &EntityManager,
        cache: &ResourceCache,
        world_style: u32,
        visit_micros: u32,
        player: Option<u32>,
        random: &mut impl FnMut() -> u16,
    ) -> Result<(), AttractCameraStop> {
        self.countdown_ms -= (visit_micros / 1000) as i32;
        if self.countdown_ms >= 0 {
            return Ok(());
        }
        let style_6 = world_style == 6;
        if !(style_6 || random() & 1 != 0) {
            self.subject = player;
            self.countdown_ms = i32::from(random() >> 5) + 4000;
            return Ok(());
        }
        self.countdown_ms = i32::from(random() >> 5) + 2000;
        self.subject = None;
        for entity in entities.iter_all() {
            if !fun_0042e210_entity_is_live(entity.collision.state_flags_at_0x08) {
                continue;
            }
            let eligible = entity.capability_flags & 8 != 0
                || (style_6
                    && cache
                        .global_entity_type(entity.entity_type as usize)
                        .ok_or(AttractCameraStop {
                            entity_id: entity.id,
                            entity_type: entity.entity_type,
                        })?
                        .subsections
                        .iter()
                        .any(|sub| sub.name == "F"));
            if !eligible {
                continue;
            }
            if random() & 7 == 0 {
                self.subject = Some(entity.id);
                break;
            }
            if !self
                .subject
                .is_some_and(|id| entities.iter_all().any(|other| other.id == id))
            {
                self.subject = Some(entity.id);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn countdown_expires_after_60_seconds_of_20_ms_frames_then_reloads_10() {
        let mut countdown = IdleCountdown::default();
        for _ in 0..2999 {
            assert!(!countdown.tick(20_000, false, false));
        }
        assert!(countdown.tick(20_000, false, false));
        countdown.enter_frontend();
        for _ in 0..499 {
            assert!(!countdown.tick(20_000, false, false));
        }
        assert!(countdown.tick(20_000, false, false));
        countdown.key();
        countdown.enter_frontend();
        assert_eq!(countdown.remaining_ms, IDLE_PERIOD_MS);
    }

    #[test]
    fn countdown_truncates_each_frame_to_whole_ms_and_waits_while_suspended() {
        let mut countdown = IdleCountdown::default();
        countdown.tick(4_166, false, false);
        assert_eq!(countdown.remaining_ms, IDLE_PERIOD_MS - 4);
        countdown.tick(400_000, false, false);
        assert_eq!(countdown.remaining_ms, IDLE_PERIOD_MS - 129);
        countdown.tick(20_000, true, false);
        countdown.tick(20_000, false, true);
        assert_eq!(countdown.remaining_ms, IDLE_PERIOD_MS - 129);
    }

    #[test]
    fn expiry_starts_mode_1_one_time_in_8_at_the_ring() {
        assert_eq!(
            IdleExpiry::at_depth(1, || 0x91AD),
            IdleExpiry::Start(AttractMode::World)
        );
        assert_eq!(
            IdleExpiry::at_depth(1, || 0x91A8),
            IdleExpiry::Start(AttractMode::Intro2)
        );
        assert_eq!(
            IdleExpiry::at_depth(2, || unreachable!()),
            IdleExpiry::Pop {
                klaus_command: true
            }
        );
        assert_eq!(
            IdleExpiry::at_depth(3, || unreachable!()),
            IdleExpiry::Pop {
                klaus_command: false
            }
        );
    }

    #[test]
    fn camera_counts_down_then_style_draws_choose_roaming_or_the_player() {
        let entities = EntityManager::from_entities_for_test(Vec::new());
        let cache = ResourceCache::new(Vec::new());
        let mut camera = AttractCamera::default();
        let mut words = [2u16, 0x20 << 5, 1, 0x40 << 5].into_iter();
        let mut random = || words.next().unwrap();
        // The first visit after a load: an even word takes the player for
        // 4000 + (word >> 5) ms.
        camera
            .visit(&entities, &cache, 0, 20_000, Some(7), &mut random)
            .unwrap();
        assert_eq!(
            (camera.subject(), camera.countdown_ms),
            (Some(7), 4000 + 0x20)
        );
        camera.countdown_ms = 10;
        camera
            .visit(&entities, &cache, 0, 10_999, Some(7), &mut random)
            .unwrap();
        assert_eq!(camera.countdown_ms, 0);
        // Odd: roam for 2000 + (word >> 5) ms; an empty walk leaves no subject.
        camera
            .visit(&entities, &cache, 0, 1_000, Some(7), &mut random)
            .unwrap();
        assert_eq!((camera.subject(), camera.countdown_ms), (None, 2000 + 0x40));
        assert!(words.next().is_none());
        // Style 6 always roams, without the parity word.
        let mut words = [0u16].into_iter();
        let mut camera = AttractCamera::default();
        camera
            .visit(&entities, &cache, 6, 20_000, None, &mut || {
                words.next().unwrap()
            })
            .unwrap();
        assert_eq!(camera.countdown_ms, 2000);
    }

    fn retail_session(level_id: u32) -> crate::session::GameSession {
        let mut session =
            crate::session::GameSession::init(&v2k_test_support::retail_dir()).unwrap();
        session.load_auxiliary_ovl(3, 1).unwrap();
        session.load_level_by_id(level_id, 1).unwrap();
        session
    }

    #[v2k_test_support::retail_test]
    fn the_origin_proxy_enters_intro2_at_its_traced_start() {
        let session = retail_session(50);
        let level = session.cache.level_desc().unwrap();
        let proxy = crate::opening::IntroCameraController::at_rest([0; 3], 0, Default::default());
        let arrived = proxy.arrive_at([0; 3], level.terrain_draw_depth, level.world_style);
        assert_eq!(arrived.position_raw(), [0, 0x200, -0x1100]);
    }

    #[v2k_test_support::retail_test]
    fn phase_4_records_are_the_demo_caption_and_world_11_is_style_6() {
        let session = retail_session(ATTRACT_ROWS[6].overlay_id());
        let records: Vec<_> = (ATTRACT_RECORDS_STRING..)
            .map_while(|id| session.cache.global_string(id))
            .take_while(|record| !record.starts_with('#'))
            .collect();
        assert_eq!(records, ["	<    *, *,16,30,   15>Demo"]);
        assert_eq!(session.cache.level_desc().unwrap().world_style, 6);
    }

    /// The lab's row-0 run: `FUN_00451AF0` builds the craft first, from RNG
    /// state `0x8BBF572E`, drawing Sub-G, Sub-A, the selector and B6C0's two
    /// 06070 suffixes (Sub-G then Sub-A each), leaving `0xBA7A18AF` for the
    /// world's next body. Its pose, health and Sub-G words are the lab dump's.
    #[v2k_test_support::retail_test]
    fn row_0_craft_is_the_first_body_and_takes_the_labs_seven_words() {
        use crate::entity::{
            AttractCraftArrival, AttractWorld, AuthoredWorldConstruction,
            EntityConstructionResources, EntityManager,
        };
        let row = ATTRACT_ROWS[0];
        let session = retail_session(row.overlay_id());
        let metadata: Vec<_> = (0..session.cache.global_entity_model_table().len())
            .map(|id| {
                crate::entity_collision_state::EntityTypeRuntimeMetadata::from_section12(
                    session.cache.global_entity_type(id).unwrap(),
                )
            })
            .collect();
        let terrain = session.cache.terrain().unwrap();
        let mut fx = crate::world_fx::WorldFx::new();
        fx.set_shared_retail_random_state(0x8BBF_572E);
        let position_raw = row.player_position_raw(terrain);
        let manager = EntityManager::from_authored_world(
            AuthoredWorldConstruction {
                level: session.cache.level_desc().unwrap(),
                logical_world_index: i32::from(row.logical_world),
                type_metadata: &metadata,
                player_arrival: None,
                retail_tick: 0,
                resources: EntityConstructionResources {
                    terrain: Some(terrain),
                    terrain_objects: session.cache.terrain_objects(),
                    model_extent_raw: Some(&|id| session.cache.global_model(id).map(|m| m.radius)),
                },
            }
            .with_attract(
                0,
                AttractWorld {
                    craft: Some(AttractCraftArrival {
                        position_raw,
                        self_righting: 9,
                        sub_e_record: None,
                    }),
                },
            ),
            &mut fx,
        )
        .unwrap();
        let craft = manager.iter_all().next().unwrap();
        assert_eq!(manager.attract_craft_id(), Some(craft.id));
        assert_eq!(craft.entity_type, 46);
        assert_eq!(craft.position_raw(), [24320, 256, 19712]);
        assert_eq!(craft.velocity_raw(), [0; 3]);
        assert_eq!(craft.rotation_heading_pitch_roll_raw(), [0x4000, 0, 0]);
        assert_eq!(
            craft.collision.health_raw,
            crate::entity_collision_state::RetailRuntimeValue::Known(40_000)
        );
        let owner = craft.intro2_flyer_frame_owner.unwrap();
        // +0x90: the controller's boot pose (DAT_004D0B30) at construction.
        assert_eq!(owner.anchor_raw, [19712, -500, 14848]);
        let crate::entity_collision_state::RetailRuntimeValue::Known(Some(sub_g)) =
            craft.sub_g_06070_runtime
        else {
            panic!("craft Sub-G");
        };
        // The second B6C0 06070 word 0xA411: clearance 0 + 0xA4.
        assert_eq!(
            sub_g.randomized_target_raw_at_0x38(),
            crate::entity_collision_state::RetailRuntimeValue::Known(164)
        );
        assert_eq!(
            sub_g.byte_at_0x41(),
            crate::entity_collision_state::RetailRuntimeValue::Known(9)
        );
        let aim = craft.intro2_flyer_aim_runtime.as_ref().unwrap();
        // FUN_00443560: the boot record's selector 1, Sub-E's sound 88.
        assert_eq!(aim.emitter_runtime().projectile_method, 1);
        assert_eq!(aim.emitter_runtime().sound_id, 88);
        // The world's next draw (a Type111's selector, 0x8953) follows.
        let mut after = crate::world_fx::WorldFx::new();
        after.set_shared_retail_random_state(0xBA7A_18AF);
        assert_eq!(after.next_shared_retail_random_u16(), 0x8953);
    }

    #[test]
    fn rows_decode_and_the_lab_word_picks_world_11() {
        // The lab's idle run draws 0x8BBF at the game start.
        let row = ATTRACT_ROWS[AttractRow::index(0x8BBF)];
        assert_eq!(AttractRow::index(0x8BBF), 6);
        assert_eq!(row.logical_world, 11);
        assert!(!row.has_player());
        assert_eq!(row.overlay_id(), 23);
        assert_eq!(AttractRow::index(0xFFFF), 11);
        let first = ATTRACT_ROWS[0];
        assert_eq!(
            (
                first.logical_world,
                first.projectile_method,
                first.weapon_selector,
                first.cadence_ms,
                first.sound_id
            ),
            (7, 14, 6, 100, 0x4E)
        );
        assert_eq!(first.arrival_raw, [0x5F00, 0x0100, 0x4D00]);
        assert_eq!(ATTRACT_ROWS[2].arrival_raw[1], -0x200);
        assert_eq!(
            ATTRACT_ROWS.iter().filter(|row| row.has_player()).count(),
            10
        );
    }
}
