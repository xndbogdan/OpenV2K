//! Native player terrain/water dispatch at the 11AD0 manager boundaries.
//!
//! The outer walk retains entry geometry and gates; D7F0/D860 reread the live
//! style. A material or physical packet can publish 447280 synchronously before
//! the following suffix, without borrowing the allocation across that callback.

use crate::entity::{
    EntityManager, PlayerCheckedDamageFrame, PlayerDyingContactRuntime,
    PlayerSurfaceContactDispatch, PlayerSurfaceContactPhase,
};
use crate::entity_collision_state::{RetailRuntimeValue, TERRAIN_CONTACT_RESPONSE_STATE_BIT};
use crate::gameplay_notifications::GameplayNotifications;
use crate::player::VehicleMode;
use crate::player_contact_style::{
    apply_player_dying_surface_contact, player_surface_style, PlayerContactStyleBlock,
    PlayerContactStyleRequest, PlayerDyingSurfaceContactFrame, PlayerDyingSurfaceContactOutcome,
    PlayerSurfaceStyle,
};
use crate::player_hull::PlayerHull;
use crate::resource_cache::ResourceCache;
use crate::terrain_contact::{
    begin_player_terrain_contact, PlayerTerrainContactOutcome, PlayerTerrainStyleCallback,
    PlayerTerrainStyleCallbackRequest, TerrainContactError,
};
use crate::world_fx::WorldFx;
use v2k_formats::models::{AnimVars, ModelEntry};
use v2k_formats::terrain::TerrainGrid;

pub struct PlayerSurfaceContactRequest<'a> {
    pub resources: &'a ResourceCache,
    pub terrain: &'a TerrainGrid,
    pub entry_model: Option<&'a ModelEntry>,
    pub entry_anim_vars: &'a AnimVars,
    pub vehicle_mode: VehicleMode,
    pub controlled_entity_handle: Option<u32>,
    pub ground_response_selectors: [u8; 8],
    pub haptic_scale_raw: i32,
    pub retail_tick: u32,
    pub extra_lives: u8,
}

pub struct PlayerSurfaceContactFrame<'a> {
    pub request: PlayerSurfaceContactRequest<'a>,
    pub hull: &'a mut PlayerHull,
    pub world_fx: &'a mut WorldFx,
    pub notifications: &'a mut GameplayNotifications,
    pub dying_runtime: Option<PlayerDyingContactRuntime>,
    pub water_style_outcome:
        Option<Result<PlayerDyingSurfaceContactOutcome, PlayerContactStyleBlock>>,
}

type TerrainResult = Option<Result<Option<PlayerTerrainContactOutcome>, TerrainContactError>>;

impl PlayerSurfaceContactFrame<'_> {
    pub fn dispatch(
        &mut self,
        manager: &mut EntityManager,
        phase: PlayerSurfaceContactPhase,
    ) -> PlayerSurfaceContactDispatch<TerrainResult> {
        match phase {
            PlayerSurfaceContactPhase::Terrain => PlayerSurfaceContactDispatch::Terrain(
                self.request
                    .entry_model
                    .map(|model| self.terrain(manager, model)),
            ),
            PlayerSurfaceContactPhase::Water { position_raw, .. } => {
                if self.water(manager, position_raw) {
                    PlayerSurfaceContactDispatch::WaterComplete
                } else {
                    PlayerSurfaceContactDispatch::WaterBlocked
                }
            }
        }
    }

    pub fn commit_runtime(&self, manager: &mut EntityManager) -> bool {
        self.dying_runtime
            .is_none_or(|runtime| manager.commit_player_dying_contact_runtime(runtime))
    }

    fn begin_death(&mut self, manager: &mut EntityManager) -> Result<(), TerrainContactError> {
        if !self.hull.dying {
            return Ok(());
        }
        if !self.commit_runtime(manager) {
            return Err(TerrainContactError::NativeRuntime(
                "controlled dying burst latch",
            ));
        }
        manager.sync_player_hull_collision_state(self.hull);
        manager
            .begin_player_dying(PlayerCheckedDamageFrame {
                hull: self.hull,
                resources: self.request.resources,
                world_fx: self.world_fx,
                retail_tick: self.request.retail_tick,
                notifications: self.notifications,
                extra_lives: self.request.extra_lives,
            })
            .map_err(TerrainContactError::NativeDeath)?;
        self.dying_runtime = manager.player_dying_contact_runtime();
        Ok(())
    }

    fn terrain(
        &mut self,
        manager: &mut EntityManager,
        model: &ModelEntry,
    ) -> Result<Option<PlayerTerrainContactOutcome>, TerrainContactError> {
        let player = manager.player().ok_or(TerrainContactError::NativeRuntime(
            "terrain player allocation",
        ))?;
        let mut position = player.position_raw();
        let mut velocity = player.velocity_raw();
        let RetailRuntimeValue::Known(basis) = player.physical_body_basis_q31() else {
            return Err(TerrainContactError::NativeRuntime("terrain physical basis"));
        };
        let request = PlayerContactStyleRequest {
            behavior_context: player.current_behavior_context,
            entity_handle: player.id,
            controlled_entity_handle: self.request.controlled_entity_handle,
            haptic_scale_raw: self.request.haptic_scale_raw,
        };
        let style = if matches!(request.behavior_context,RetailRuntimeValue::Known(Some(context)) if player_surface_style(context)==Some(PlayerSurfaceStyle::DyingBounce))
        {
            let extent = player
                .model_index
                .and_then(|id| self.request.resources.global_model(id))
                .ok_or(TerrainContactError::NativeRuntime("dying effect model"))?
                .radius
                >> 1;
            PlayerTerrainStyleCallback::DyingBounce(PlayerDyingSurfaceContactFrame {
                request,
                runtime: &mut self.dying_runtime,
                world_fx: self.world_fx,
                source_extent_raw: extent,
                sea_level_raw: Some(self.request.terrain.sea_level_raw()),
                retail_tick: self.request.retail_tick,
            })
        } else {
            PlayerTerrainStyleCallback::PlayerControl(PlayerTerrainStyleCallbackRequest {
                behavior_context: request.behavior_context,
                vehicle_mode: self.request.vehicle_mode,
                entity_handle: request.entity_handle,
                controlled_entity_handle: request.controlled_entity_handle,
                ground_response_selectors: self.request.ground_response_selectors,
                haptic_scale_raw: request.haptic_scale_raw,
            })
        };
        let Some(continuation) = begin_player_terrain_contact(
            self.request.terrain,
            model,
            basis
                .orientation_world_from_model()
                .map(|row| row.map(f64::from)),
            self.request.entry_anim_vars,
            style,
            &mut position,
            &mut velocity,
            self.hull,
        )?
        else {
            return Ok(None);
        };
        manager
            .player_mut()
            .ok_or(TerrainContactError::NativeRuntime("terrain style survivor"))?
            .set_motion_raw(position, velocity);
        // 448280's material 411760 call finishes before D7F0 re-resolves XYZ.
        self.begin_death(manager)?;
        let player = manager
            .player()
            .ok_or(TerrainContactError::NativeRuntime("terrain tail survivor"))?;
        position = player.position_raw();
        velocity = player.velocity_raw();
        let outcome = continuation.finish(&mut position, &mut velocity, self.hull);
        let player = manager
            .player_mut()
            .ok_or(TerrainContactError::NativeRuntime(
                "terrain response survivor",
            ))?;
        player.collision.state_flags_at_0x08.overwrite(
            TERRAIN_CONTACT_RESPONSE_STATE_BIT,
            TERRAIN_CONTACT_RESPONSE_STATE_BIT,
        );
        player.set_motion_raw(position, velocity);
        self.begin_death(manager)?;
        Ok(Some(outcome))
    }

    fn water(&mut self, manager: &mut EntityManager, position: [i16; 3]) -> bool {
        let Some(player) = manager.player_mut() else {
            eprintln!("Player water entry lost its controlled allocation");
            return false;
        };
        let Some(record) = self
            .request
            .resources
            .global_entity_type(player.entity_type as usize)
        else {
            eprintln!("Player water entry cue has no type record");
            return false;
        };
        let cue = u16::from_le_bytes([record.raw_header[0x88], record.raw_header[0x89]]);
        self.world_fx.note_water_entry();
        if cue != 0 {
            self.world_fx
                .queue_fixed_positional_sound_raw(cue, position);
        }
        let context = player.current_behavior_context;
        if self.hull.dying && self.dying_runtime.is_none() {
            eprintln!("Player water suffix is blocked by unfinished native death publication");
            return false;
        }
        match context {
            RetailRuntimeValue::Known(None) => return true,
            RetailRuntimeValue::Known(Some(context))
                if matches!(
                    player_surface_style(context),
                    Some(PlayerSurfaceStyle::PlayerControl | PlayerSurfaceStyle::Null)
                ) =>
            {
                return true
            }
            RetailRuntimeValue::Known(Some(context))
                if player_surface_style(context) == Some(PlayerSurfaceStyle::DyingBounce) => {}
            _ => {
                eprintln!("Player water +14 callback has unowned behavior context: {context:?}");
                return false;
            }
        }
        let Some(model) = player
            .model_index
            .and_then(|id| self.request.resources.global_model(id))
        else {
            eprintln!("Player dying water effect model is unowned");
            return false;
        };
        let mut velocity = player.velocity_raw();
        let result = apply_player_dying_surface_contact(
            PlayerDyingSurfaceContactFrame {
                request: PlayerContactStyleRequest {
                    behavior_context: context,
                    entity_handle: player.id,
                    controlled_entity_handle: self.request.controlled_entity_handle,
                    haptic_scale_raw: self.request.haptic_scale_raw,
                },
                runtime: &mut self.dying_runtime,
                world_fx: self.world_fx,
                source_extent_raw: model.radius >> 1,
                sea_level_raw: Some(self.request.terrain.sea_level_raw()),
                retail_tick: self.request.retail_tick,
            },
            position,
            // 129B0 authors 0x7fff, not a normalized Q12 unit normal.
            // AF0 nevertheless consumes these exact words with its >>12 dot.
            [0, i16::MAX, 0],
            &mut velocity,
        );
        self.water_style_outcome = Some(result);
        match result {
            Ok(outcome) => {
                player.set_motion_raw(player.position_raw(), velocity);
                if let Some(haptic) = outcome.unsupported_haptic {
                    eprintln!("Player dying water haptic backend is unowned: {haptic:?}");
                }
                true
            }
            Err(block) => {
                eprintln!("Player dying water callback blocked: {block:?}");
                false
            }
        }
    }
}
