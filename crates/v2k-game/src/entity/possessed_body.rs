//! A possessed body's Player Control (`FUN_00446640`) and the craft's lost
//! controller.
//!
//! The takeover (`FUN_004480E0`) gives the target Type46's behavior source,
//! whose Hovering style (`0x4CD940`) installs Player Control with movement
//! style 0. Its update reads the controls and runs the hover integrator
//! (`FUN_00444CF0`) over the target's own components: for a Type17, Sub-D
//! turns it by `turn * 0x34 / 64` (`FUN_00420360`), Sub-C holds it over the
//! ground (`FUN_0041F1C0`), Sub-B removes sideways speed (`FUN_0041EB50`) and
//! Sub-A drives it while the throttle is held (`FUN_0041E9E0`, direction from
//! the throttle's sign); it has no Sub-O, Sub-E, Sub-F, Sub-K or Sub-L, so the
//! mode controller, weapon and the later dampers do nothing, and its Sub-H
//! steps (`FUN_0041D0A0`). The style's environment mask `0x10010` joins the
//! type's `+C0` flags for the common tail; its `+20` hook is null and its
//! `+1C` hook is `FUN_00447C90`.

use super::*;
use crate::common_mover::environment::{
    apply_common_wind_drag_raw, CommonWindDrag, CommonWindDragFrame,
};
use crate::common_mover::sub_c::sub_c_uses_wave_surface;
use crate::common_mover::type9_attitude::{
    plan_type9_terrain_attitude_raw, Type9TerrainAttitudeInput,
};
use crate::common_mover::SubAPropulsionRuntime;
use crate::damage::velocity_delta_impact_raw;
use crate::hover::{apply_horizontal_velocity_raw, steering_step_raw, HoverPhysicsConfig};
use crate::resource_cache::ResourceCache;
use crate::static_contact::{
    apply_contact_response_raw, scan_deepest_static_contact, static_contact_subject_eligible,
    StaticContactQuery,
};
use crate::sub_h_external_frame::commit_sub_h_geometry;

/// `FUN_00444CF0`'s Sub-D rate numerator.
const PLAYER_CONTROL_STEERING_NUMERATOR: i32 = 0x34;

/// Player Control's inputs for one possessed body's visit.
#[derive(Clone, Copy)]
pub struct PossessedBodyFrame<'a> {
    /// The reader's turn channel (`+280`).
    pub turn_channel: i16,
    /// The reader's throttle channel (`+288`): only its sign drives.
    pub throttle_q16: i32,
    pub elapsed_micros: u32,
    pub terrain: &'a TerrainGrid,
    pub terrain_objects: &'a v2k_formats::anim_frames::TerrainObjectTable,
    pub model_pool: &'a ResourceCache,
    /// Section-13 `+84`: the wave clock moves the sea.
    pub water_enabled: bool,
    pub retail_tick: u32,
}

/// A value the possessed body's visit needs that the port does not hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PossessedBodyBlock {
    BodyUnavailable,
    /// Only Type17's A/B/C/D/H/J bodies run the possessed Player Control.
    UnsupportedType(u32),
    Components,
    BodyBasis,
    StateFlags,
    SubARuntime,
    /// The body carries cargo, whose mass the lift reads (`FUN_004185C0`).
    Cargo,
    SubH,
    /// `FUN_0041D0A0` finished a stride, whose cue is not ported here.
    SubHCue,
    ActiveModel,
    StaticScan,
}

/// What a static-model contact did; the deliveries are held.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PossessedStaticContact {
    pub impact_raw: i32,
    pub cell: [u8; 2],
}

impl EntityManager {
    /// One possessed Type17's Player Control visit and its common tail.
    /// A static-model contact gets retail's physical response (`FUN_00411760`
    /// through `FUN_0040D920`); its hook and damage deliveries are returned
    /// for the caller to hold.
    pub fn possessed_body_player_control(
        &mut self,
        id: u32,
        frame: PossessedBodyFrame<'_>,
    ) -> Result<Option<PossessedStaticContact>, PossessedBodyBlock> {
        let elapsed = frame.elapsed_micros.min(RETAIL_FRAME_DELTA_MAX_US);
        let environment = self.common_environment_physics();
        let entity_type = self
            .entities
            .iter()
            .find(|entity| entity.id == id && entity.active)
            .ok_or(PossessedBodyBlock::BodyUnavailable)?
            .entity_type;
        if entity_type != 17 {
            return Err(PossessedBodyBlock::UnsupportedType(entity_type));
        }
        let metadata = self
            .type_runtime_metadata(entity_type)
            .cloned()
            .ok_or(PossessedBodyBlock::Components)?;
        let (
            RetailRuntimeValue::Known(Some(sub_a)),
            RetailRuntimeValue::Known(Some(sub_b)),
            RetailRuntimeValue::Known(Some(sub_c)),
            RetailRuntimeValue::Known(Some(sub_d)),
            RetailRuntimeValue::Known(Some(sub_h)),
        ) = (
            metadata.sub_a_propulsion_descriptor,
            metadata.sub_b_lateral_descriptor,
            metadata.sub_c_lift_descriptor,
            metadata.sub_d_steering_descriptor,
            metadata.sub_h_external_frame_descriptor.clone(),
        )
        else {
            return Err(PossessedBodyBlock::Components);
        };
        let config = HoverPhysicsConfig {
            drive_acceleration_raw: sub_a.acceleration_raw,
            overspeed_correction_raw: sub_a.overspeed_correction_raw,
            target_speed_base_raw: sub_a.target_speed_base_raw,
            lateral_dead_zone_raw: sub_b.projection_threshold_rate_raw,
            lateral_correction_raw: sub_b.correction_rate_raw,
            lift: HoverLiftConfig::from(sub_c),
            steering_divisor_raw: sub_d.steering_divisor_raw,
            steering_constructor_axis_flag: sub_d.couple_yaw_into_roll_raw != 0,
            steering_tail_raw: [0; 7],
            environment_flags_at_type_record_0xc0: 0,
        };
        let entity = self
            .entities
            .iter_mut()
            .find(|entity| entity.id == id)
            .expect("found above");
        let RetailRuntimeValue::Known(type_flags) = entity.collision.default_state_flags_at_0xc8
        else {
            return Err(PossessedBodyBlock::StateFlags);
        };
        let effective_flags = (type_flags | HOVER_BEHAVIOR_ENVIRONMENT_OR_MASK)
            & !HOVER_BEHAVIOR_ENVIRONMENT_CLEAR_MASK;
        let RetailRuntimeValue::Known(previous) = entity.physical_body_basis_q31 else {
            return Err(PossessedBodyBlock::BodyBasis);
        };
        let RetailRuntimeValue::Known(Some(propulsion)) = entity.sub_a_propulsion_runtime else {
            return Err(PossessedBodyBlock::SubARuntime);
        };
        let RetailRuntimeValue::Known(propulsion_target_raw) = propulsion.target_speed_raw() else {
            return Err(PossessedBodyBlock::SubARuntime);
        };
        if !matches!(&entity.sub_j_attachment_runtime,
            RetailRuntimeValue::Known(Some(runtime)) if runtime.is_empty())
        {
            return Err(PossessedBodyBlock::Cargo);
        }
        let attached_cargo_mass = 0;
        let RetailRuntimeValue::Known(state_flags) =
            entity.collision.state_flags_at_0x08.masked(u32::MAX)
        else {
            return Err(PossessedBodyBlock::StateFlags);
        };
        let active_model_extent_raw = entity
            .model_index
            .and_then(|model| frame.model_pool.global_model(model))
            .map(|model| model.radius)
            .ok_or(PossessedBodyBlock::ActiveModel)?;

        // FUN_00444CF0's Sub-D: the rate and step it publishes, then the yaw.
        let [heading_before, pitch_before, roll_before] = entity.rotation_heading_pitch_roll_raw();
        let step = steering_step_raw(frame.turn_channel, elapsed, sub_d.steering_divisor_raw);
        let heading = heading_before.wrapping_sub(step);
        if let Some(runtime) = entity.type17_sub_d_runtime.as_mut() {
            runtime.yaw_rate_raw = i32::from(frame.turn_channel)
                * PLAYER_CONTROL_STEERING_NUMERATOR
                / sub_d.steering_divisor_raw;
            runtime.last_yaw_step_raw = step;
        }

        // The common FUN_0040E640 attitude, from the turned pose, as the
        // craft's Hovering style runs it.
        let turned = HoverBasis {
            lateral: previous.lateral,
            up: previous.up,
            forward: previous.forward,
        };
        let attitude = plan_type9_terrain_attitude_raw(Type9TerrainAttitudeInput {
            terrain: frame.terrain,
            position_raw: entity.position_raw(),
            pitch_raw: pitch_before,
            roll_raw: roll_before,
            lateral_basis_q31: turned.lateral,
            forward_basis_q31: turned.forward,
            state_flags,
            effective_flags,
            active_model_extent_raw,
            resolved_surface_mode_raw: i8::from(
                config.lift.use_wave_surface && attached_cargo_mass <= 0x96,
            ),
            water_enabled: frame.water_enabled,
            wave_tick_50hz: frame.retail_tick as i32,
            effective_elapsed_micros: elapsed,
        });

        // Sub-C, Sub-B, then Sub-A through the previous frame's matrix.
        let basis = HoverBasis {
            lateral: previous.lateral,
            up: previous.up,
            forward: previous.forward,
        };
        let mut position_raw = entity.position_raw();
        let mut velocity_raw = entity.velocity_raw();
        let [sample_x, sample_z] =
            hover_surface_sample_raw(position_raw, basis, config.lift.offset_sample);
        let terrain_y_raw = terrain_height_raw(frame.terrain, sample_x, sample_z);
        let sea = frame.terrain.water_enabled();
        let targets_wave = sea && sub_c_uses_wave_surface(config.lift, attached_cargo_mass);
        let sampled_surface_raw = if !targets_wave {
            terrain_y_raw
        } else if environment.waves_enabled {
            v2k_formats::terrain::wave_surface_raw(
                sample_x,
                sample_z,
                frame.retail_tick as i32,
                frame.terrain.sea_level_raw(),
                terrain_y_raw,
            )
        } else {
            terrain_y_raw.max(frame.terrain.sea_level_raw())
        };
        apply_hover_lift_raw(
            &mut position_raw,
            &mut velocity_raw,
            elapsed,
            terrain_y_raw,
            sampled_surface_raw,
            targets_wave,
            config.lift,
            basis,
        );
        let direction = frame.throttle_q16.signum();
        let mut propulsion = propulsion;
        if direction != 0 {
            // The integrator writes the throttle's sign into Sub-A's +4.
            propulsion = SubAPropulsionRuntime::from_retail_words(
                propulsion.target_speed_raw(),
                direction,
                propulsion.drive_scale_percent(),
            );
        }
        apply_horizontal_velocity_raw(
            config,
            propulsion_target_raw,
            propulsion.drive_scale_percent(),
            direction as f32,
            basis,
            &mut velocity_raw,
            elapsed,
        );
        entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(Some(propulsion));

        // Player Control's Sub-H step after the integrator.
        let RetailRuntimeValue::Known(Some(runtime)) = &mut entity.sub_h_external_frame_runtime
        else {
            return Err(PossessedBodyBlock::SubH);
        };
        let cues = runtime
            .update(&sub_h, elapsed as i32)
            .map_err(|_| PossessedBodyBlock::SubH)?;
        if !cues.is_empty() {
            return Err(PossessedBodyBlock::SubHCue);
        }

        // The common tail: FUN_00413F70, FUN_0040E100's gravity and drag
        // under the effective flags, then FUN_00412DA0's motion.
        let mut angles = [heading, attitude.pitch_raw, attitude.roll_raw];
        let rebuilt = Type9BodyBasis::from_angle_words(angles[0], angles[1], angles[2]);
        entity.physical_body_basis_q31 = RetailRuntimeValue::Known(rebuilt);
        entity
            .collision
            .state_flags_at_0x08
            .overwrite(BODY_BASIS_REBUILT_STATE_BIT, BODY_BASIS_REBUILT_STATE_BIT);
        apply_common_gravity_and_underwater_raw(
            &mut velocity_raw,
            elapsed,
            CommonUnderwaterFrame {
                effective_environment_flags: effective_flags,
                water_response_enabled: config.lift.use_wave_surface,
                position_y_raw: position_raw[1],
                solid_or_sea_y_raw: common_environment_surface_raw(
                    frame.terrain,
                    position_raw[0],
                    position_raw[2],
                ),
                self_mass_raw: entity.mass_raw,
                attached_cargo_mass,
            },
        );
        if effective_flags & ENVIRONMENT_WIND_DRAG_FLAG != 0 {
            if let Some(callback_mass_raw) = std::num::NonZeroU16::new(entity.mass_raw) {
                apply_common_wind_drag_raw(
                    &mut velocity_raw,
                    &mut angles,
                    CommonWindDrag::from_environment(environment),
                    CommonWindDragFrame {
                        terrain: frame.terrain,
                        position_raw,
                        basis: rebuilt,
                        callback_mass_raw,
                        elapsed_micros: elapsed,
                    },
                );
            }
        }
        entity.set_rotation_heading_pitch_roll_raw(angles);
        entity.set_motion_raw(position_raw, velocity_raw);
        commit_known_common_master_motion(entity, elapsed);

        let contact = possessed_static_contact(entity, frame)?;
        if let Some(model_records) = entity
            .model_index
            .and_then(|model| frame.model_pool.global_model(model))
            .map(|model| model.records.as_slice())
        {
            let axes = match entity.physical_body_basis_q31() {
                RetailRuntimeValue::Known(basis) => Some([basis.lateral, basis.up, basis.forward]),
                RetailRuntimeValue::Unresolved => None,
            };
            let origin_raw = entity.position_raw();
            if let RetailRuntimeValue::Known(Some(runtime)) =
                &mut entity.sub_h_external_frame_runtime
            {
                let _ = commit_sub_h_geometry(
                    runtime,
                    &sub_h,
                    model_records,
                    origin_raw,
                    axes,
                    |x, z| frame.terrain.bilinear_height_raw(x, z),
                );
            }
        }
        Ok(contact)
    }
}

/// `00411AD0 -> 00412CF0 -> 0040D920`: the deepest static-model contact gets
/// `FUN_00411760`'s response; the Hovering style has no `+20` task hook and
/// no crush bit, and its `+1C` hook (`FUN_00447C90`) and the deliveries are
/// left to the caller.
fn possessed_static_contact(
    entity: &mut Entity,
    frame: PossessedBodyFrame<'_>,
) -> Result<Option<PossessedStaticContact>, PossessedBodyBlock> {
    let model = entity
        .model_index
        .and_then(|model| frame.model_pool.global_model(model))
        .ok_or(PossessedBodyBlock::ActiveModel)?;
    match static_contact_subject_eligible(&entity.collision, model.collision_radius_raw) {
        RetailRuntimeValue::Known(true) => {}
        RetailRuntimeValue::Known(false) => return Ok(None),
        RetailRuntimeValue::Unresolved => return Err(PossessedBodyBlock::StateFlags),
    }
    let RetailRuntimeValue::Known(basis) = entity.physical_body_basis_q31() else {
        return Err(PossessedBodyBlock::BodyBasis);
    };
    let anim_vars = entity.presentation_anim_vars(frame.retail_tick);
    let Some(contact) = scan_deepest_static_contact(StaticContactQuery {
        terrain: frame.terrain,
        terrain_objects: frame.terrain_objects,
        model_pool: frame.model_pool,
        tick: frame.retail_tick,
        active_model: model,
        active_model_to_world_basis: basis
            .orientation_world_from_model()
            .map(|row| row.map(f64::from)),
        active_anim_vars: &anim_vars,
        position_raw: entity.position_raw(),
    })
    .map_err(|_| PossessedBodyBlock::StaticScan)?
    else {
        return Ok(None);
    };
    let velocity_before_raw = entity.velocity_raw();
    let mut position_raw = entity.position_raw();
    let mut velocity_raw = velocity_before_raw;
    apply_contact_response_raw(&mut position_raw, &mut velocity_raw, contact);
    entity.set_motion_raw(position_raw, velocity_raw);
    Ok(Some(PossessedStaticContact {
        impact_raw: velocity_delta_impact_raw(velocity_before_raw, velocity_raw, entity.mass_raw),
        cell: contact.cell,
    }))
}

/// Why a MindBall could not be born or flown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MindBallBlock {
    Metadata,
    Terrain,
    Selection,
    Allocation,
}

impl EntityManager {
    /// `FUN_00409950`'s birth (`FUN_00438080`, type 98, no velocity or
    /// angles) at the released body's position, then `FUN_0040AC60`'s
    /// install: the behavior's selector word (`FUN_00425680`) and, for a
    /// type with Sub-A, `FUN_00406070`'s target word. The ball is never drawn
    /// as its type's model: `FUN_00409950` swaps its draw for `FUN_004097A0`.
    pub fn construct_mind_ball(
        &mut self,
        position_raw: [i16; 3],
        resources: &ResourceCache,
        fx: &mut WorldFx,
        retail_tick: u32,
    ) -> Result<u32, MindBallBlock> {
        let entity_type = crate::possession::MIND_BALL_TYPE;
        let metadata = resources
            .global_entity_type(entity_type as usize)
            .map(EntityTypeRuntimeMetadata::from_section12)
            .ok_or(MindBallBlock::Metadata)?;
        if self.type_metadata.get(entity_type as usize) != Some(&metadata) {
            return Err(MindBallBlock::Metadata);
        }
        let terrain = resources.level_terrain().ok_or(MindBallBlock::Terrain)?;
        let surface_bits = crate::entity_initializer::constructor_surface_bits_at_tick(
            position_raw,
            terrain,
            retail_tick,
            self.common_environment_physics().waves_enabled,
        )
        .ok_or(MindBallBlock::Terrain)?;
        let id = self.next_entity_id.max(1);
        if self.entities.iter().any(|actor| actor.id == id) {
            return Err(MindBallBlock::Allocation);
        }
        let choices = metadata
            .initializer
            .as_ref()
            .map(|spec| spec.behavior_choices.clone())
            .ok_or(MindBallBlock::Metadata)?;
        let selected = crate::entity_behavior::select_initial_behavior(
            &choices,
            |rule| i32::from(rule == crate::entity_behavior::BehaviorWeightRule::Always),
            || u32::from(fx.next_shared_retail_random_u16()),
        )
        .ok()
        .flatten()
        .ok_or(MindBallBlock::Selection)?;
        let stamp = self.begin_common_body_attempt();
        let mut entity =
            Entity::unresolved_port_entity(id, EntityKind::from_type(entity_type), entity_type);
        entity.construction_stamp_at_0xb4 = stamp;
        entity.set_motion_raw(position_raw, [0; 3]);
        entity.set_rotation_heading_pitch_roll_raw([0; 3]);
        entity.physical_body_basis_q31 =
            RetailRuntimeValue::Known(Type9BodyBasis::from_angle_words(0, 0, 0));
        entity.mass_raw = metadata.mass_raw;
        entity.capability_flags = metadata.capability_flags;
        let resolution =
            crate::entity_initializer::resolve_entity_initializer_with_selected_behavior(
                EntityInitializerRequest {
                    metadata: Some(&metadata),
                    spawn_param: 0,
                    authored_position_raw: position_raw,
                    terrain: Some(terrain),
                    resource_domain: ResourceDomainRelation::Current,
                },
                Some(selected),
            );
        let mut state_flags = resolution.state_flags;
        state_flags.overwrite(
            crate::entity_collision_state::SURFACE_STATE_MASK,
            surface_bits,
        );
        entity.collision =
            EntityCollisionRuntimeState::from_constructor(Some(&metadata), 0, state_flags);
        entity.initial_behavior = RetailRuntimeValue::Known(Some(selected));
        entity.sub_a_propulsion_runtime = RetailRuntimeValue::Known(None);
        if let RetailRuntimeValue::Known(Some(sub_a)) = metadata.sub_a_propulsion_descriptor {
            entity.sub_a_propulsion_runtime =
                RetailRuntimeValue::Known(Some(SubAPropulsionRuntime::from_20450_constructor(
                    sub_a,
                    fx.next_shared_retail_random_u16(),
                )));
        }
        assert_eq!(self.allocate_live_entity_id(), id);
        self.append_live_entity(entity);
        Ok(id)
    }

    /// `FUN_00409460` on the ball `id`, then `FUN_00410C10`'s removal when
    /// the flight ends.
    pub fn fly_mind_ball(
        &mut self,
        id: u32,
        flight: &mut crate::possession::MindBallFlight,
        elapsed_us: i32,
        terrain: &TerrainGrid,
    ) -> crate::possession::MindBallStep {
        let live = |manager: &Self, id: u32| {
            manager
                .entities
                .iter()
                .find(|entity| entity.id == id && entity.active)
                .map(|entity| entity.position_raw())
        };
        let target = live(self, flight.target);
        let craft = live(self, flight.craft);
        let Some(ball) = self.entity_mut(id) else {
            return crate::possession::MindBallStep::Die;
        };
        let mut position_raw = ball.position_raw();
        let step = flight.advance(&mut position_raw, target, craft, elapsed_us, |x, z| {
            terrain.bilinear_height_raw(x, z)
        });
        let velocity_raw = ball.velocity_raw();
        ball.set_motion_raw(position_raw, velocity_raw);
        if step == crate::possession::MindBallStep::Die {
            // FUN_00410C10: the ball's death leaves it to the deferred sweep.
            ball.collision
                .state_flags_at_0x08
                .apply_deferred_destroy_pending_write();
            self.queue_actor_deferred_destroy(id);
        }
        step
    }
}

/// Why a possessed Type17 could not take its own behavior back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PossessionReleaseBlock {
    BodyUnavailable,
    Metadata,
    LastHitTick,
    Context,
    Plan(crate::type17_impact_reselection::Type17ImpactReselectionError),
    FollowBeacons(crate::type17_follow_beacons_live::Type17FollowBeaconsLiveError),
}

impl EntityManager {
    /// The Hovering style's initializer (`FUN_004464F0`) on the target:
    /// Tertiary, then Secondary cleared, then Player Control in Primary,
    /// which the possession state holds.
    pub fn possess_body_tasks(&mut self, id: u32) -> bool {
        let Some(entity) = self.entity_mut(id) else {
            return false;
        };
        for slot in [
            crate::actor_task_owner::ActorTaskSlot::Tertiary,
            crate::actor_task_owner::ActorTaskSlot::Secondary,
            crate::actor_task_owner::ActorTaskSlot::Primary,
        ] {
            entity.actor_tasks.clear_slot(slot);
        }
        true
    }

    /// `FUN_00446D70`'s `FUN_0040AC60` on a released Type17: its own
    /// behavior source again, chosen afresh among its choices with the one
    /// selector word, and the chosen class's initializer.
    pub fn release_possessed_type17(
        &mut self,
        id: u32,
        current_tick: u32,
        fx: &mut WorldFx,
    ) -> Result<crate::type17_impact_reselection::Type17ImpactInstallDecision, PossessionReleaseBlock>
    {
        use crate::type17_impact_reselection::{
            plan_type17_weighted_selection, Type17ImpactEntityRef, Type17ImpactInstallDecision,
            Type17WeightedSelectionRequest, TYPE17_IMPACT_MODEL_ID,
        };
        let metadata = self
            .type_runtime_metadata(17)
            .cloned()
            .ok_or(PossessionReleaseBlock::Metadata)?;
        let candidates: Vec<Type17ImpactEntityRef> = self
            .iter()
            .map(|entity| Type17ImpactEntityRef {
                id: entity.id,
                entity_type: entity.entity_type,
                position_raw: entity.position_raw(),
                state_flags_raw: entity.collision.state_flags_at_0x08,
                capability_flags: RetailRuntimeValue::Known(entity.capability_flags),
                attached_entity_handle: RetailRuntimeValue::Known(None),
            })
            .collect();
        let entity = self
            .entities
            .iter_mut()
            .find(|entity| entity.id == id && entity.active)
            .ok_or(PossessionReleaseBlock::BodyUnavailable)?;
        let RetailRuntimeValue::Known(last_hit_tick) =
            entity.collision.last_hit_presentation_tick_at_0x34
        else {
            return Err(PossessionReleaseBlock::LastHitTick);
        };
        let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
            return Err(PossessionReleaseBlock::Context);
        };
        let owner = Type17ImpactEntityRef {
            id: entity.id,
            entity_type: entity.entity_type,
            position_raw: entity.position_raw(),
            state_flags_raw: entity.collision.state_flags_at_0x08,
            capability_flags: RetailRuntimeValue::Known(entity.capability_flags),
            attached_entity_handle: entity.collision.recent_relation_id_at_0x60,
        };
        let weighted = plan_type17_weighted_selection(
            Type17WeightedSelectionRequest {
                active_model_id: TYPE17_IMPACT_MODEL_ID,
                current_tick,
                last_hit_tick,
                metadata: &metadata,
                owner,
                candidates_in_intrusive_order: &candidates,
            },
            || u32::from(fx.next_shared_retail_random_u16()),
        )
        .map_err(PossessionReleaseBlock::Plan)?;
        let program = weighted.selection.program;
        let context = context
            .reselect_named_type_default(
                program,
                program.initial_style_table_index_raw,
                program.initial_style,
            )
            .ok_or(PossessionReleaseBlock::Context)?;
        match weighted.install {
            Type17ImpactInstallDecision::FollowBeaconsAcquiring => {
                crate::type17_follow_beacons_live::publish_type17_follow_beacons_acquiring(
                    entity, context, &metadata, fx,
                )
                .map_err(PossessionReleaseBlock::FollowBeacons)?;
            }
            Type17ImpactInstallDecision::SharedAcquiring => {
                crate::type17_impact_live::publish_type17_shared_acquiring(
                    entity, program, context, &metadata, fx,
                );
            }
        }
        Ok(weighted.install)
    }
}
