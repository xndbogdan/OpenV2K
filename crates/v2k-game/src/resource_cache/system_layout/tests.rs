use super::*;
use crate::{
    actor_task_owner::ActorTaskSlot,
    entity::{EntityConstructionResources, EntityManager},
    entity_collision_state::EntityTypeRuntimeMetadata,
    session::GameSession,
    system_layout::{HighSystemLayoutTier as Tier, SystemLayoutSource},
    world_fx::WorldFx,
};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

fn retail_root() -> PathBuf {
    let root = v2k_test_support::retail_dir();
    assert!(root.join("PRELOAD.DAT").is_file(), "retail corpus required");
    root
}

fn session() -> GameSession {
    let mut session = GameSession::init(&retail_root()).unwrap();
    for level in [2, 3] {
        session.load_auxiliary_ovl(level, 1).unwrap();
    }
    session
}

fn layouts(
    cache: &ResourceCache,
) -> Vec<(
    Option<u32>,
    String,
    Option<SystemLayoutOrigin>,
    Option<Vec<u32>>,
    Option<Vec<u32>>,
)> {
    cache
        .layers
        .iter()
        .map(|layer| {
            (
                layer.state.system_level,
                layer.state.source_path.clone(),
                layer
                    .system_layout_receipt
                    .as_ref()
                    .map(|receipt| receipt.origin.clone()),
                layer
                    .state
                    .fixup_data
                    .as_ref()
                    .map(|table| table.entries.clone()),
                layer
                    .state
                    .fixup_code
                    .as_ref()
                    .map(|table| table.entries.clone()),
            )
        })
        .collect()
}

fn intrinsic_addresses(cache: &ResourceCache) -> Vec<usize> {
    fn address<T>(value: &Option<T>) -> usize {
        value.as_ref().map_or(0, |value| value as *const T as usize)
    }
    cache
        .layers
        .iter()
        .flat_map(|layer| {
            let state = &layer.state;
            [
                state as *const LevelState as usize,
                state.strings.as_ptr() as usize,
                address(&state.sprites),
                address(&state.params),
                address(&state.display_modes),
                address(&state.fog_gradient),
                address(&state.color_palettes),
                address(&state.models),
                address(&state.anim_frames),
                address(&state.terrain),
                address(&state.anim_sound),
                address(&state.collision),
                address(&state.level),
                address(&state.linkage),
            ]
        })
        .collect()
}

/// Corruption probes touch only unique generated copies, never retail assets.
struct ScratchOverlays {
    root: PathBuf,
}
impl ScratchOverlays {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "v2k-system-layout-{}-{stamp}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).unwrap();
        std::fs::create_dir(root.join("Overlay")).unwrap();
        for level in [2, 3] {
            std::fs::copy(
                retail_root()
                    .join("Overlay")
                    .join(format!("2X{level}XX.OVL")),
                root.join("Overlay").join(format!("2X{level}XX.OVL")),
            )
            .unwrap();
        }
        Self { root }
    }
    fn path(&self, level: u32) -> PathBuf {
        let path = self.root.join("Overlay").join(format!("2X{level}XX.OVL"));
        assert!(path.starts_with(&self.root));
        path
    }
    fn mutate(&self, level: u32, f: impl FnOnce(&mut Vec<u8>, &v2k_formats::ovl::OvlFile)) {
        let path = self.path(level);
        let mut bytes = std::fs::read(&path).unwrap();
        let ovl = v2k_formats::ovl::OvlFile::parse(&bytes).unwrap();
        f(&mut bytes, &ovl);
        std::fs::write(path, bytes).unwrap();
    }
}
impl Drop for ScratchOverlays {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).unwrap();
    }
}

#[v2k_test_support::retail_test]
fn high_system_layout_all_tiers_stage_without_writes_and_commit_in_place() {
    let mut session = session();
    let original_paths = session
        .cache
        .layers
        .iter()
        .map(|layer| layer.state.source_path.clone())
        .collect::<Vec<_>>();
    let addresses = intrinsic_addresses(&session.cache);
    let layer_count = session.cache.layers.len();
    for tier in [
        Tier::High640,
        Tier::High800,
        Tier::High1024,
        Tier::High640,
        Tier::High1024,
        Tier::High800,
        Tier::High640,
    ] {
        let before = layouts(&session.cache);
        let stage = session.prepare_high_system_layout_refresh(tier).unwrap();
        assert_eq!(layouts(&session.cache), before, "preparation is read-only");
        assert_eq!(stage.tier(), tier);
        assert_eq!(
            (stage.system_data_value(2, 2), stage.system_data_value(2, 3)),
            (Some(tier.size().0), Some(tier.size().1))
        );
        let expected_point = stage.system_layout_point(3, 10).unwrap();
        let origins = stage.origins().clone();
        assert_eq!(
            session
                .cache
                .commit_high_system_layout_refresh(stage)
                .unwrap(),
            origins
        );
        assert_eq!(
            session.cache.system_layout_origin(2),
            Some(&origins.system2)
        );
        assert_eq!(
            session.cache.system_layout_origin(3),
            Some(&origins.system3)
        );
        assert_eq!(
            session.cache.system_layout_point(3, 10),
            Some(expected_point)
        );
        assert_eq!(intrinsic_addresses(&session.cache), addresses);
        assert_eq!(session.cache.layers.len(), layer_count);
        assert_eq!(
            session
                .cache
                .layers
                .iter()
                .map(|layer| layer.state.source_path.clone())
                .collect::<Vec<_>>(),
            original_paths
        );
    }
}

#[v2k_test_support::retail_test]
fn high_system_layout_missing_second_file_retains_both_resident_layouts() {
    let scratch = ScratchOverlays::new();
    std::fs::remove_file(scratch.path(3)).unwrap();
    let mut session = session();
    let before = layouts(&session.cache);
    session.data_dir = scratch.root.clone();
    assert!(matches!(
        session.prepare_high_system_layout_refresh(Tier::High800),
        Err(SystemLayoutRefreshError::Read {
            system_level: 3,
            ..
        })
    ));
    assert_eq!(layouts(&session.cache), before);
}

#[v2k_test_support::retail_test]
fn high_system_layout_intrinsic_header_and_payload_corruption_reject_atomically() {
    for (level, section_index, header) in
        [(2, 3, false), (3, 12, false), (2, 12, true), (3, 12, true)]
    {
        let scratch = ScratchOverlays::new();
        // Empty-section allocation headers and populated intrinsic payloads
        // both remain part of the exact source receipt.
        scratch.mutate(level, |bytes, ovl| {
            let section = &ovl.sections[section_index];
            assert!(header || !section.data.is_empty());
            let offset = if header {
                section.marker_offset + 4
            } else {
                section.marker_offset + 8
            };
            bytes[offset] ^= 1;
        });
        let mut session = session();
        let before = layouts(&session.cache);
        session.data_dir = scratch.root.clone();
        assert!(
            matches!(session.prepare_high_system_layout_refresh(Tier::High800), Err(SystemLayoutRefreshError::IntrinsicSectionChanged { system_level, section }) if system_level == level && section == section_index)
        );
        assert_eq!(layouts(&session.cache), before);
    }
}

#[v2k_test_support::retail_test]
fn high_system_layout_table_shape_viewport_and_radar_config_reject_atomically() {
    for control in 0..5 {
        let scratch = ScratchOverlays::new();
        let level = if control == 2 { 3 } else { 2 };
        scratch.mutate(level, |bytes, ovl| match control {
            0 => bytes[ovl.sections[0].marker_offset + 4] ^= 1,
            1 => bytes[ovl.sections[1].marker_offset + 4] ^= 1,
            2 => bytes[ovl.sections[0].marker_offset + 8 + 2 * 4] ^= 1,
            3 => bytes[ovl.sections[0].marker_offset + 8 + 2 * 4] ^= 1,
            4 => {
                let section = &ovl.sections[1];
                bytes.remove(section.marker_offset + 8 + section.data.len() - 1);
            }
            _ => unreachable!(),
        });
        let mut session = session();
        let before = layouts(&session.cache);
        session.data_dir = scratch.root.clone();
        let error = session
            .prepare_high_system_layout_refresh(Tier::High800)
            .unwrap_err();
        assert!(
            match control {
                0 => matches!(
                    error,
                    SystemLayoutRefreshError::InvalidTable {
                        system_level: 2,
                        section: 0
                    }
                ),
                1 => matches!(
                    error,
                    SystemLayoutRefreshError::InvalidTable {
                        system_level: 2,
                        section: 1
                    }
                ),
                2 => matches!(
                    error,
                    SystemLayoutRefreshError::InvariantScalarChanged { index: 2 }
                ),
                3 => matches!(
                    error,
                    SystemLayoutRefreshError::ViewportMismatch {
                        tier: Tier::High800
                    }
                ),
                4 => matches!(
                    error,
                    SystemLayoutRefreshError::InvalidTable {
                        system_level: 2,
                        section: 1
                    }
                ),
                _ => false,
            },
            "control{control}: {error:?}"
        );
        assert_eq!(layouts(&session.cache), before);
    }
}

#[v2k_test_support::retail_test]
fn high_system_layout_stale_tables_and_rebound_intrinsic_receipt_reject_before_write() {
    for rebound in [false, true] {
        let mut session = session();
        let stale = session
            .prepare_high_system_layout_refresh(Tier::High800)
            .unwrap();
        if rebound {
            session.load_auxiliary_ovl(3, 1).unwrap();
        } else {
            let newer = session
                .prepare_high_system_layout_refresh(Tier::High1024)
                .unwrap();
            session
                .cache
                .commit_high_system_layout_refresh(newer)
                .unwrap();
        }
        let before = layouts(&session.cache);
        assert!(matches!(
            session.cache.commit_high_system_layout_refresh(stale),
            Err(SystemLayoutRefreshError::StaleStage { .. })
        ));
        assert_eq!(layouts(&session.cache), before);
    }
}

#[v2k_test_support::retail_test]
fn high_system_layout_low_and_unproven_parsed_high_sources_require_intrinsic_load() {
    for low in [false, true] {
        let mut session = GameSession::init(&retail_root()).unwrap();
        session.load_auxiliary_ovl(3, 1).unwrap();
        if low {
            session.load_auxiliary_ovl(2, 0).unwrap();
        } else {
            let mut parsed = session.load_ovl_by_id(2, 1).unwrap();
            parsed.system_level = Some(2);
            session.cache.add_auxiliary(parsed);
        }
        let before = layouts(&session.cache);
        assert!(matches!(
            session.prepare_high_system_layout_refresh(Tier::High800),
            Err(SystemLayoutRefreshError::ResidentSourceUnproven { system_level: 2 })
        ));
        assert_eq!(layouts(&session.cache), before);
    }
}

#[v2k_test_support::retail_test]
fn high_system_layout_preserves_live_world_terrain_radar_actor_clocks_tasks_and_rng() {
    let mut session = session();
    session.load_level_by_id(13, 1).unwrap();
    let metadata = session
        .cache
        .global_entity_model_table()
        .iter()
        .enumerate()
        .map(|(kind, _)| {
            EntityTypeRuntimeMetadata::from_section12(
                session.cache.global_entity_type(kind).unwrap(),
            )
        })
        .collect::<Vec<_>>();
    let mut fx = WorldFx::new();
    let entities = EntityManager::from_fresh_new_game_level_with_type_metadata(
        session.cache.level_desc().unwrap(),
        &metadata,
        EntityConstructionResources::new(session.cache.terrain(), session.cache.terrain_objects()),
        57,
        &mut fx,
    )
    .unwrap();
    session
        .cache
        .initialize_level_terrain_radar(&mut || fx.next_shared_retail_random_u16())
        .unwrap();
    let actors = || {
        entities
            .iter_all()
            .map(|entity| {
                (
                    entity.id,
                    entity.position_raw(),
                    entity.velocity_raw(),
                    format!("{:?}", entity.collision),
                    format!("{:?}", entity.current_behavior_context),
                    ActorTaskSlot::IN_RETAIL_TICK_ORDER
                        .into_iter()
                        .map(|slot| format!("{:?}", entity.actor_task_state(slot)))
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>()
    };
    let before_actors = actors();
    let before_world_path = session.cache.level().unwrap().source_path.clone();
    let before_terrain = session
        .cache
        .level_terrain()
        .unwrap()
        .cells
        .iter()
        .map(|cell| [cell.height, cell.attribute, cell.terrain_type])
        .collect::<Vec<_>>();
    let radar = session.cache.level_terrain_radar().unwrap();
    let before_radar = (
        radar.revision(),
        radar.packed_coverage().to_vec(),
        radar.indices().to_vec(),
        radar.indices().as_ptr() as usize,
    );
    let before_addresses = intrinsic_addresses(&session.cache);
    let before_flags = (
        session.cache.level_abort_terrain_transformed,
        session.cache.level_terrain_presentation_dirty,
    );
    let mut control_fx = fx.fork_for_main_base_abort_transaction();
    for tier in [Tier::High800, Tier::High1024, Tier::High640] {
        let stage = session.prepare_high_system_layout_refresh(tier).unwrap();
        session
            .cache
            .commit_high_system_layout_refresh(stage)
            .unwrap();
    }
    assert_eq!(actors(), before_actors);
    assert_eq!(
        session.cache.level().unwrap().source_path,
        before_world_path
    );
    assert_eq!(
        session
            .cache
            .level_terrain()
            .unwrap()
            .cells
            .iter()
            .map(|cell| [cell.height, cell.attribute, cell.terrain_type])
            .collect::<Vec<_>>(),
        before_terrain
    );
    let radar = session.cache.level_terrain_radar().unwrap();
    assert_eq!(
        (
            radar.revision(),
            radar.packed_coverage().to_vec(),
            radar.indices().to_vec(),
            radar.indices().as_ptr() as usize
        ),
        before_radar
    );
    assert_eq!(intrinsic_addresses(&session.cache), before_addresses);
    assert_eq!(
        (
            session.cache.level_abort_terrain_transformed,
            session.cache.level_terrain_presentation_dirty
        ),
        before_flags
    );
    assert_eq!(
        fx.next_shared_retail_random_u16(),
        control_fx.next_shared_retail_random_u16()
    );
}
