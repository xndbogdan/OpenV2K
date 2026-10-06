//! Native fish 10EB0/11180/11250/11320 hits: style C690,11030, then15040.
//!
//! Aimless7930, Wander79C0 and both Flocking7DF8/7E40 styles have C690 in
//! primary+28 and infected+20. Their selector draw and three reaction draws
//! precede damage filtering, including rejected packets. B/D/F task resets
//! add no RNG. Class2's finished quiet death has null impact callbacks.

use crate::{
    entity_behavior::ActiveBehaviorStyle,
    entity_collision_state::{RetailRuntimeValue, DYING_STATE_BIT},
    impact_reaction::{
        apply_impact_reaction, ImpactReactionBody, IMPACT_REACTION_ENABLED_STATE_BIT,
        IMPACT_REACTION_NETWORKED_STATE_BIT, IMPACT_REACTION_SUPPRESSED_STATE_BIT,
    },
    live_actor_checked_damage::{
        apply_live_actor_checked_damage, LiveActorDamageEntry, LiveActorDamageError,
        LiveActorDamageOutcome, LiveActorDamageRequest,
    },
    world_fx::{
        particle_uses_fun_0043f780_entity_hit, particle_uses_fun_0043f7c0_entity_hit,
        particle_uses_static_route_entity_hit, ParticleEntityImpact,
    },
};

pub use super::death::SharedFishDeathBlock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SharedFishImpactBlock {
    Runtime(&'static str),
    Damage(LiveActorDamageError<SharedFishDeathBlock, ()>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SharedFishImpactOutcome {
    NotApplicable,
    Applied(LiveActorDamageOutcome<()>),
    Blocked {
        reason: SharedFishImpactBlock,
        committed_prefix: bool,
    },
}

pub(crate) fn apply_shared_fish_particle_hit(
    frame: crate::shared_actor_impact::SharedActorImpactFrame<'_>,
    impact: ParticleEntityImpact,
) -> SharedFishImpactOutcome {
    let Some(entity) = frame
        .entities
        .iter_all()
        .find(|entity| entity.id == impact.target_entity_id)
    else {
        return SharedFishImpactOutcome::NotApplicable;
    };
    if !super::is_shared_fish_type(entity.entity_type) || entity.shared_fish_runtime.is_none() {
        return SharedFishImpactOutcome::NotApplicable;
    }
    if entity
        .shared_fish_runtime
        .as_ref()
        .unwrap()
        .impact_prefix_pending
    {
        return SharedFishImpactOutcome::Blocked {
            reason: SharedFishImpactBlock::Runtime("pending fish hit prefix"),
            committed_prefix: false,
        };
    }
    let mut committed = false;
    let result = run(
        crate::shared_actor_impact::SharedActorImpactFrame {
            resources: frame.resources,
            entities: &mut *frame.entities,
            world_fx: &mut *frame.world_fx,
            scheduler: &mut *frame.scheduler,
            notifications: &mut *frame.notifications,
            retail_tick: frame.retail_tick,
        },
        impact,
        &mut committed,
    );
    match result {
        Ok(result) => SharedFishImpactOutcome::Applied(result),
        Err(reason) => {
            if committed {
                // Class2 already retired its scheduler owner. Allocation custody
                // must still prevent replay of a failed repeat-hit/suffix prefix.
                frame
                    .entities
                    .entity_mut(impact.target_entity_id)
                    .unwrap()
                    .shared_fish_runtime
                    .as_mut()
                    .unwrap()
                    .impact_prefix_pending = true;
            }
            SharedFishImpactOutcome::Blocked {
                reason,
                committed_prefix: committed,
            }
        }
    }
}

fn run(
    frame: crate::shared_actor_impact::SharedActorImpactFrame<'_>,
    impact: ParticleEntityImpact,
    committed: &mut bool,
) -> Result<LiveActorDamageOutcome<()>, SharedFishImpactBlock> {
    use SharedFishImpactBlock as Block;
    let crate::shared_actor_impact::SharedActorImpactFrame {
        entities: manager,
        world_fx,
        scheduler,
        notifications,
        retail_tick,
        resources: _,
    } = frame;
    let id = impact.target_entity_id;
    let static_route = particle_uses_static_route_entity_hit(impact.source_particle_class);
    if scheduler.shared_fish_has_pending_prefix(id) {
        return Err(Block::Runtime("pending actor/contact prefix"));
    }
    let delivery = impact
        .damage_delivery_record()
        .ok_or(Block::Runtime("particle provenance"))?;
    if !crate::shared_fish::allocation_authenticates(manager, id) {
        return Err(Block::Runtime("native allocation"));
    }
    let completed_death = super::death::completed_shared_fish_death(manager, id);
    if !scheduler.shared_fish_completed_owner(manager, id) && !completed_death {
        return Err(Block::Runtime("completed fish task custody"));
    }
    let entity_type = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .ok_or(Block::Runtime("allocation"))?
        .entity_type;
    let metadata = manager
        .type_runtime_metadata(entity_type)
        .cloned()
        .ok_or(Block::Runtime("metadata"))?;
    let infected = particle_uses_fun_0043f780_entity_hit(impact.source_particle_class);
    let cured = particle_uses_fun_0043f7c0_entity_hit(impact.source_particle_class);
    if infected {
        // The infected model-bit prefix is generic entry behavior shared with
        // the ground-actor sequence; fish carry no infected sound.
        let applied = manager
            .apply_fun_00411250_infected_model_bit(id)
            .ok_or(Block::Runtime("infected model prefix"))?;
        *committed = true;
        if let Some(sound) = applied.sound_id {
            world_fx.queue_fixed_positional_sound_raw(sound, applied.position_raw);
        }
    } else if cured {
        // 11320 clears the infected model bit and requests type+84 even
        // for a dying target. It preserves10EB0's presentation timestamp.
        let applied = manager
            .apply_fun_00411320_cured_model_bit(id)
            .ok_or(Block::Runtime("cured model prefix"))?;
        *committed = true;
        if let Some(sound) = applied.sound_id {
            world_fx.queue_fixed_positional_sound_raw(sound, applied.position_raw);
        }
    } else {
        manager
            .entity_mut(id)
            .ok_or(Block::Runtime("allocation"))?
            .collision
            .last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(retail_tick);
        *committed = true;
    }
    // PE4111C1..411219 stamps +34, requests the living type+80 cue,
    // then invokes the same DAC0/style+28 callback as10EB0. Fish author
    // a zero cue, but the ordering and pre-filter gate remain explicit.
    if static_route {
        let entity = manager.entity_mut(id).unwrap();
        let RetailRuntimeValue::Known(dying) =
            entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT)
        else {
            return Err(Block::Runtime("static-hit dying state"));
        };
        if dying == 0 {
            let RetailRuntimeValue::Known(sound) = metadata.accepted_hit_presentation_sound_id
            else {
                return Err(Block::Runtime("static-hit sound"));
            };
            if let Some(sound) = sound {
                world_fx.queue_fixed_positional_sound_raw(sound, entity.position_raw());
            }
        }
    }
    let entity = manager.entity_mut(id).unwrap();
    let RetailRuntimeValue::Known(Some(context)) = entity.current_behavior_context else {
        return Err(Block::Runtime("hit context"));
    };
    let ActiveBehaviorStyle::Audited(style) = context.active_style() else {
        return Err(Block::Runtime("hit style"));
    };
    // PE tables4C7930/79C0/7DF8/7E40: +20,+24 and+28 are40C690.
    // This direct style call omits task-result dispatch's416410 suppression.
    match style.frame_address {
        0x004c_7930 | 0x004c_79c0 | 0x004c_7df8 | 0x004c_7e40 => {
            crate::shared_fish_tasks::reselect_behavior(entity, &metadata, &mut || {
                u32::from(world_fx.next_shared_retail_random_u16())
            })
            .map_err(|_| Block::Runtime("hit C690 selection"))?;
            let owner = super::SharedFishOwner::adopt(manager, id)
                .map_err(|_| Block::Runtime("reselected fish graph"))?;
            scheduler.register_shared_fish(owner);
        }
        address
            if completed_death
                && address == crate::entity_behavior::QUIET_DEATH_STYLE.frame_address => {}
        _ => return Err(Block::Runtime("unaudited fish hit style")),
    }
    let entity = manager.entity_mut(id).unwrap();
    let RetailRuntimeValue::Known(flags) = entity.collision.state_flags_at_0x08.masked(
        IMPACT_REACTION_ENABLED_STATE_BIT
            | IMPACT_REACTION_NETWORKED_STATE_BIT
            | IMPACT_REACTION_SUPPRESSED_STATE_BIT,
    ) else {
        return Err(Block::Runtime("impact reaction state"));
    };
    if flags & IMPACT_REACTION_NETWORKED_STATE_BIT != 0 {
        return Err(Block::Runtime("network impact reaction"));
    }
    let mut body = ImpactReactionBody {
        state_flags_at_0x08: flags,
        mass_raw_at_0xb0: entity.mass_raw,
        linear_velocity_xyz_raw: entity.velocity_raw(),
        angular_heading_pitch_roll_raw: entity.rotation_heading_pitch_roll_raw(),
    };
    apply_impact_reaction(
        &mut body,
        id,
        // 41121E multiplies only the11030 force by eight.15040 still
        // receives the original packet, including wrapping32-bit amounts.
        delivery
            .packet
            .impact_sum_raw()
            .wrapping_mul(if static_route { 8 } else { 1 }),
        impact.velocity_raw,
        || u32::from(world_fx.next_shared_retail_random_u16()),
    )
    .map_err(|_| Block::Runtime("impact reaction mass"))?;
    entity.set_velocity_raw(body.linear_velocity_xyz_raw);
    entity.set_rotation_heading_pitch_roll_raw(body.angular_heading_pitch_roll_raw);
    let checked = apply_live_actor_checked_damage(
        manager,
        world_fx,
        LiveActorDamageRequest {
            ratio_numerator: 0,
            ratio_denominator: 0,
            feedback: Some(crate::live_actor_checked_damage::LiveActorDamageFeedback {
                notifications,
                retail_tick,
            }),
            entity_id: id,
            delivery,
            entry: LiveActorDamageEntry::Checked,
        },
        |manager, world_fx, _| {
            super::death::begin_shared_fish_standard_death(manager, id, world_fx, scheduler)
        },
    )
    .map_err(|error| {
        *committed |= error.committed_prefix;
        if error.death_publication.is_some() {
            scheduler.retire_shared_fish(id);
        }
        Block::Damage(error)
    })?;
    if checked.death_publication.is_some() {
        scheduler.retire_shared_fish(id);
    }
    // 10EB0's accepted-hit suffix follows damage/death. The retail fish have
    // capability0 and sound+80 zero; keep those independent runtime checks.
    if !static_route && !infected && !cured && checked.filtered_damage_raw != 0 {
        let entity = manager.entity_mut(id).unwrap();
        let RetailRuntimeValue::Known(dying) =
            entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT)
        else {
            return Err(Block::Runtime("accepted-hit dying state"));
        };
        if dying == 0 {
            let RetailRuntimeValue::Known(sound) =
                entity.collision.accepted_hit_presentation_sound_id
            else {
                return Err(Block::Runtime("accepted-hit sound"));
            };
            if let Some(sound) = sound {
                world_fx.queue_fixed_positional_sound_raw(sound, entity.position_raw());
            }
        }
        if entity.capability_flags & 8 != 0 {
            return Err(Block::Runtime("capability8 accepted-hit suffix"));
        }
    }
    Ok(checked)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        actor_task_owner::ActorTaskSlot,
        damage::{DamagePacket, FUN_0043F780_DAMAGE_PACKET, FUN_0043F7C0_DAMAGE_PACKET},
        entity::{AuthoredWorldConstruction, EntityConstructionResources, EntityManager},
        entity_collision_state::{EntityTypeRuntimeMetadata, RetailRuntimeValue},
        gameplay_notifications::GameplayNotifications,
        session::GameSession,
        specialized_actor_task_production::SpecializedActorTaskScheduler,
        world_fx::{BallisticDamageRequest, ParticleEntityImpact, WorldFx},
    };

    struct World {
        session: GameSession,
        manager: EntityManager,
        scheduler: SpecializedActorTaskScheduler,
        fx: WorldFx,
        notifications: GameplayNotifications,
        id: u32,
    }

    impl World {
        fn new() -> Self {
            Self::with_type(22, 22)
        }

        fn with_type(level: u32, entity_type: u32) -> Self {
            let data = v2k_test_support::retail_dir();
            assert!(data.join("PRELOAD.DAT").is_file(), "retail corpus required");
            let mut session = GameSession::init(&data).unwrap();
            session.load_auxiliary_ovl(3, 1).unwrap();
            session.load_level_by_id(level, 1).unwrap();
            let rows: Vec<_> = session
                .cache
                .global_entity_model_table()
                .iter()
                .enumerate()
                .map(|(id, _)| {
                    EntityTypeRuntimeMetadata::from_section12(
                        session.cache.global_entity_type(id).unwrap(),
                    )
                })
                .collect();
            let mut fx = WorldFx::new();
            let manager = EntityManager::from_authored_world(
                AuthoredWorldConstruction {
                    logical_world_index: (level - 12) as i32,
                    level: session.cache.level_desc().unwrap(),
                    type_metadata: &rows,
                    resources: EntityConstructionResources {
                        terrain: session.cache.terrain(),
                        terrain_objects: session.cache.terrain_objects(),
                        model_extent_raw: Some(&|id| {
                            session.cache.global_model(id).map(|m| m.radius)
                        }),
                    },
                    player_arrival: None,
                    retail_tick: 1000,
                },
                &mut fx,
            )
            .unwrap();
            let id = manager
                .iter_all()
                .find(|entity| entity.entity_type == entity_type)
                .unwrap()
                .id;
            let mut scheduler = SpecializedActorTaskScheduler::new();
            assert!(scheduler.adopt_shared_fish(&manager) > 0);
            Self {
                session,
                manager,
                scheduler,
                fx,
                notifications: GameplayNotifications::new(),
                id,
            }
        }

        fn force_style(&mut self, class: u32, acquired: bool) {
            use crate::entity_behavior::{
                behavior_program, BehaviorContextRuntime, BehaviorSelection,
            };
            let kind = self.manager.entity_mut(self.id).unwrap().entity_type;
            let metadata = self.manager.type_runtime_metadata(kind).unwrap().clone();
            let selection = BehaviorSelection {
                choice_index: 0,
                program: behavior_program(class).unwrap(),
            };
            let context = BehaviorContextRuntime::from_fresh_weighted_selection(selection).unwrap();
            crate::shared_fish_tasks::publish_selection(
                self.manager.entity_mut(self.id).unwrap(),
                &metadata,
                selection,
                context,
            )
            .unwrap();
            if acquired {
                let others: Vec<_> = self
                    .manager
                    .iter_all()
                    .filter(|e| e.entity_type == kind && e.id != self.id)
                    .map(|e| e.id)
                    .collect();
                for &other in &others {
                    self.manager
                        .entity_mut(other)
                        .unwrap()
                        .set_position_raw([15000, 0, 15000]);
                }
                self.manager
                    .entity_mut(self.id)
                    .unwrap()
                    .set_position_raw([100, 0, 100]);
                self.manager
                    .entity_mut(others[0])
                    .unwrap()
                    .set_position_raw([101, 0, 100]);
                crate::shared_fish_tasks::acquire(&mut self.manager, self.id, &mut || 0).unwrap();
                let RetailRuntimeValue::Known(Some(context)) = self
                    .manager
                    .entity_mut(self.id)
                    .unwrap()
                    .current_behavior_context
                else {
                    panic!("context")
                };
                assert_eq!(context.active_style().style_address(), 0x004c_7e40);
            }
            self.scheduler.register_shared_fish(
                super::super::SharedFishOwner::adopt(&self.manager, self.id).unwrap(),
            );
        }

        fn expected_reaction(&self, impact_raw: i32) -> (WorldFx, ImpactReactionBody) {
            let entity = self.manager.iter_all().find(|e| e.id == self.id).unwrap();
            let RetailRuntimeValue::Known(flags) =
                entity.collision.state_flags_at_0x08.masked(u32::MAX)
            else {
                panic!("state")
            };
            let mut expected = self.fx.fork_for_main_base_abort_transaction();
            expected.next_shared_retail_random_u16(); // C690 selector before11030.
            let mut body = ImpactReactionBody {
                state_flags_at_0x08: flags,
                mass_raw_at_0xb0: entity.mass_raw,
                linear_velocity_xyz_raw: entity.velocity_raw(),
                angular_heading_pitch_roll_raw: entity.rotation_heading_pitch_roll_raw(),
            };
            apply_impact_reaction(&mut body, self.id, impact_raw, [0, 0, 8192], || {
                u32::from(expected.next_shared_retail_random_u16())
            })
            .unwrap();
            (expected, body)
        }

        fn hit(&mut self, class: u8, amount: i32) -> SharedFishImpactOutcome {
            let outcome = crate::shared_actor_impact::apply_shared_actor_particle_hit(
                crate::shared_actor_impact::SharedActorImpactFrame {
                    resources: &self.session.cache,
                    entities: &mut self.manager,
                    world_fx: &mut self.fx,
                    scheduler: &mut self.scheduler,
                    notifications: &mut self.notifications,
                    retail_tick: 701,
                },
                ParticleEntityImpact {
                    source_particle_class: class,
                    impact_position_argument_va: if class == 5 { 0x004D_CF48 } else { 0 },
                    target_entity_id: self.id,
                    position_world: [0.0; 3],
                    velocity_raw: [0, 0, 8192],
                    damage: Some(BallisticDamageRequest {
                        packet: match class {
                            5 => FUN_0043F780_DAMAGE_PACKET,
                            6 => FUN_0043F7C0_DAMAGE_PACKET,
                            _ => DamagePacket {
                                channels: [1, 0],
                                amounts_raw: [amount, 0],
                            },
                        },
                        source_entity_type_at_birth: Some(if class == 6 { 0 } else { 34 }),
                        source_owner_id: Some(if class == 6 { 0 } else { 35 }),
                    }),
                },
            );
            match outcome {
                Some(crate::shared_actor_impact::SharedActorImpactOutcome::Fish(outcome)) => {
                    outcome
                }
                other => panic!("fish dispatch: {other:?}"),
            }
        }
    }

    #[v2k_test_support::retail_test]
    fn nonlethal_primary_hit_reselects_then_reacts_before_checked_health() {
        let mut f = World::new();
        // Authored type22 health is 1000; channel-1 threshold 2000 leaves 500.
        let (mut expected, body) = f.expected_reaction(2_500);
        let task_before = f
            .manager
            .entity_mut(f.id)
            .unwrap()
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary);
        let result = f.hit(16, 2_500);
        let SharedFishImpactOutcome::Applied(applied) = result else {
            panic!("{result:?}")
        };
        assert_eq!(applied.filtered_damage_raw, 500);
        assert_eq!(applied.damage_after_buffer_raw, 500);
        assert!(applied.death_publication.is_none());
        let entity = f.manager.entity_mut(f.id).unwrap();
        assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(500));
        assert_eq!(entity.velocity_raw(), body.linear_velocity_xyz_raw);
        assert_eq!(
            entity.rotation_heading_pitch_roll_raw(),
            body.angular_heading_pitch_roll_raw
        );
        assert_ne!(
            entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
            task_before
        );
        assert_eq!(entity.capability_flags, 0);
        assert_eq!(
            entity.collision.accepted_hit_presentation_sound_id,
            RetailRuntimeValue::Known(None)
        );
        assert_eq!(
            entity.collision.last_hit_presentation_tick_at_0x34,
            RetailRuntimeValue::Known(701)
        );
        assert_eq!(
            f.fx.next_shared_retail_random_u16(),
            expected.next_shared_retail_random_u16()
        );
        assert!(f.fx.take_positional_sounds().is_empty());
        assert_eq!(
            f.scheduler.adopt_shared_fish(&f.manager),
            0,
            "hit must retain the adopted owner"
        );
    }

    #[v2k_test_support::retail_test]
    fn every_living_style_hit_runs_selector_before_the_entry_specific_reaction() {
        for (class, acquired) in [(5, false), (6, false), (13, false), (13, true)] {
            for particle in [16, 5, 6, 52, 68, 85] {
                let mut f = World::new();
                f.force_style(class, acquired);
                f.manager.entity_mut(f.id).unwrap().collision.health_raw =
                    RetailRuntimeValue::Known(1_000_000);
                let amount = match particle {
                    5 => FUN_0043F780_DAMAGE_PACKET.impact_sum_raw(),
                    6 => FUN_0043F7C0_DAMAGE_PACKET.impact_sum_raw(),
                    _ => 2500,
                };
                let (mut expected, body) =
                    f.expected_reaction(amount.wrapping_mul(if matches!(particle, 52 | 68 | 85) {
                        8
                    } else {
                        1
                    }));
                let entity = f.manager.entity_mut(f.id).unwrap();
                if particle == 6 {
                    entity.collision.state_flags_at_0x08.overwrite(
                        crate::entity_collision_state::ACTIVE_MODEL_SLOT_HIGH_STATE_BIT,
                        crate::entity_collision_state::ACTIVE_MODEL_SLOT_HIGH_STATE_BIT,
                    );
                }
                let old_primary = entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary);
                let incoming_basis = entity.physical_body_basis_q31;
                let tick_before = entity.collision.last_hit_presentation_tick_at_0x34;
                let result = f.hit(particle, 2500);
                assert!(
                    matches!(result, SharedFishImpactOutcome::Applied(_)),
                    "class{class}/{acquired}/particle{particle}: {result:?}"
                );
                let entity = f.manager.entity_mut(f.id).unwrap();
                assert_ne!(
                    entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
                    old_primary
                );
                assert_eq!(entity.velocity_raw(), body.linear_velocity_xyz_raw);
                assert_eq!(
                    entity.rotation_heading_pitch_roll_raw(),
                    body.angular_heading_pitch_roll_raw
                );
                assert_eq!(
                    entity.physical_body_basis_q31, incoming_basis,
                    "11030 does not rebuild the body basis"
                );
                assert_eq!(
                    entity.collision.last_hit_presentation_tick_at_0x34,
                    if matches!(particle, 5 | 6) {
                        tick_before
                    } else {
                        RetailRuntimeValue::Known(701)
                    }
                );
                if particle == 6 {
                    assert_eq!(
                        entity.collision.state_flags_at_0x08.masked(
                            crate::entity_collision_state::ACTIVE_MODEL_SLOT_HIGH_STATE_BIT
                        ),
                        RetailRuntimeValue::Known(0)
                    );
                }
                assert_eq!(
                    f.fx.next_shared_retail_random_u16(),
                    expected.next_shared_retail_random_u16()
                );
                assert!(f.scheduler.shared_fish_completed_owner(&f.manager, f.id));
                assert!(f.fx.take_positional_sounds().is_empty());
            }
        }
    }

    #[v2k_test_support::retail_test]
    fn filtered_zero_hit_still_reselects_and_reacts_without_task_result_suppression() {
        let mut f = World::new();
        f.manager
            .entity_mut(f.id)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .overwrite(0x1000, 0x1000);
        let task_before = f
            .manager
            .entity_mut(f.id)
            .unwrap()
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary);
        let (mut expected, body) = f.expected_reaction(0);
        let result = f.hit(16, 0);
        let SharedFishImpactOutcome::Applied(applied) = result else {
            panic!("{result:?}")
        };
        assert_eq!(applied.filtered_damage_raw, 0);
        let entity = f.manager.entity_mut(f.id).unwrap();
        assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(1000));
        assert_ne!(
            entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
            task_before
        );
        assert_eq!(
            entity.rotation_heading_pitch_roll_raw(),
            body.angular_heading_pitch_roll_raw
        );
        assert_eq!(
            f.fx.next_shared_retail_random_u16(),
            expected.next_shared_retail_random_u16()
        );
    }

    #[v2k_test_support::retail_test]
    fn quiet_death_retires_owner_and_a_repeat_hit_runs_only_reaction_before_sweep() {
        let mut f = World::new();
        f.manager.entity_mut(f.id).unwrap().collision.health_raw = RetailRuntimeValue::Known(1);
        let result = f.hit(16, 2500);
        let SharedFishImpactOutcome::Applied(applied) = result else {
            panic!("{result:?}")
        };
        assert_eq!(applied.death_publication, Some(()));
        assert!(super::super::death::completed_shared_fish_death(
            &f.manager, f.id
        ));
        assert!(!f.scheduler.shared_fish_completed_owner(&f.manager, f.id));
        let entity = f.manager.entity_mut(f.id).unwrap();
        assert!(ActorTaskSlot::IN_RETAIL_TICK_ORDER
            .into_iter()
            .all(|slot| entity.actor_tasks.task_in_slot(slot).is_none()));
        let mut expected = f.fx.fork_for_main_base_abort_transaction();
        for _ in 0..3 {
            expected.next_shared_retail_random_u16();
        }
        let result = f.hit(16, 2500);
        let SharedFishImpactOutcome::Applied(applied) = result else {
            panic!("{result:?}")
        };
        assert!(applied.death_publication.is_none());
        assert_eq!(
            f.manager.entity_mut(f.id).unwrap().collision.health_raw,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(
            f.fx.next_shared_retail_random_u16(),
            expected.next_shared_retail_random_u16()
        );
        assert!(f.fx.take_positional_sounds().is_empty());
    }

    #[v2k_test_support::retail_test]
    fn failed_reaction_parks_committed_selection_without_entering_health() {
        let mut f = World::new();
        f.manager.entity_mut(f.id).unwrap().mass_raw = 0;
        let task_before = f
            .manager
            .entity_mut(f.id)
            .unwrap()
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary);
        let mut expected = f.fx.fork_for_main_base_abort_transaction();
        expected.next_shared_retail_random_u16();
        let result = f.hit(16, 2500);
        assert!(
            matches!(
                result,
                SharedFishImpactOutcome::Blocked {
                    reason: SharedFishImpactBlock::Runtime("impact reaction mass"),
                    committed_prefix: true
                }
            ),
            "{result:?}"
        );
        let entity = f.manager.entity_mut(f.id).unwrap();
        assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(1000));
        assert_ne!(
            entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
            task_before
        );
        assert_eq!(
            f.fx.next_shared_retail_random_u16(),
            expected.next_shared_retail_random_u16()
        );
        assert!(f.scheduler.shared_fish_has_pending_prefix(f.id));
        assert!(matches!(
            f.hit(16, 2500),
            SharedFishImpactOutcome::Blocked {
                committed_prefix: false,
                ..
            }
        ));
    }

    #[v2k_test_support::retail_test]
    fn repeat_hit_failure_retains_prefix_after_death_retires_scheduler_owner() {
        let mut f = World::new();
        f.manager.entity_mut(f.id).unwrap().collision.health_raw = RetailRuntimeValue::Known(1);
        assert!(matches!(
            f.hit(16, 2500),
            SharedFishImpactOutcome::Applied(_)
        ));
        f.manager.entity_mut(f.id).unwrap().mass_raw = 0;
        assert!(matches!(
            f.hit(16, 2500),
            SharedFishImpactOutcome::Blocked {
                committed_prefix: true,
                ..
            }
        ));
        let before = f.manager.entity_mut(f.id).unwrap().collision.clone();
        let mut expected = f.fx.fork_for_main_base_abort_transaction();
        assert!(matches!(
            f.hit(5, 0),
            SharedFishImpactOutcome::Blocked {
                committed_prefix: false,
                ..
            }
        ));
        assert_eq!(f.manager.entity_mut(f.id).unwrap().collision, before);
        assert_eq!(
            f.fx.next_shared_retail_random_u16(),
            expected.next_shared_retail_random_u16()
        );
    }

    #[v2k_test_support::retail_test]
    fn type124_lethal_hit_retains_reselection_and_reaction_at_named_boundary() {
        let mut f = World::with_type(30, 124);
        f.manager.entity_mut(f.id).unwrap().collision.health_raw = RetailRuntimeValue::Known(1);
        let task_before = f
            .manager
            .entity_mut(f.id)
            .unwrap()
            .actor_tasks
            .task_in_slot(ActorTaskSlot::Primary);
        let amount = 1_000_000;
        let expected_health = 1 - DamagePacket {
            channels: [1, 0],
            amounts_raw: [amount, 0],
        }
        .filtered_raw(
            match &f.manager.entity_mut(f.id).unwrap().collision.damage_profile {
                RetailRuntimeValue::Known(profile) => Some(profile),
                _ => panic!("profile"),
            },
        );
        let (mut expected, body) = f.expected_reaction(amount);
        let result = f.hit(16, amount);
        let SharedFishImpactOutcome::Blocked {
            reason,
            committed_prefix,
        } = result
        else {
            panic!("{result:?}")
        };
        assert!(committed_prefix);
        assert!(
            matches!(
                &reason,
                SharedFishImpactBlock::Damage(error)
                if matches!(
                    &error.reason,
                    crate::live_actor_checked_damage::LiveActorDamageBlock::Death(
                        SharedFishDeathBlock::UnsupportedDeathProgram {
                            entity_type: 124,
                            alternate_behavior_class: 63,
                        }
                    )
                )
            ),
            "{reason:?}"
        );
        let entity = f.manager.entity_mut(f.id).unwrap();
        assert_eq!(
            entity.collision.health_raw,
            RetailRuntimeValue::Known(expected_health)
        );
        assert_eq!(entity.velocity_raw(), body.linear_velocity_xyz_raw);
        assert_eq!(
            entity.rotation_heading_pitch_roll_raw(),
            body.angular_heading_pitch_roll_raw
        );
        assert_ne!(
            entity.actor_tasks.task_in_slot(ActorTaskSlot::Primary),
            task_before,
            "the C690 hit prefix replaced Primary before the death boundary"
        );
        assert_eq!(
            f.fx.next_shared_retail_random_u16(),
            expected.next_shared_retail_random_u16()
        );
        assert!(f.scheduler.park_native_contact_prefix(&f.manager, f.id));
    }

    #[v2k_test_support::retail_test]
    fn foreign_manager_and_parked_prefix_reject_before_hit_prefix() {
        for parked in [false, true] {
            let mut f = World::new();
            if parked {
                assert!(f.scheduler.park_native_contact_prefix(&f.manager, f.id));
            } else {
                let mut other = World::new();
                std::mem::swap(
                    f.manager.entity_mut(f.id).unwrap(),
                    other.manager.entity_mut(other.id).unwrap(),
                );
            }
            let before = f.manager.entity_mut(f.id).unwrap().collision.clone();
            let mut expected = f.fx.fork_for_main_base_abort_transaction();
            let result = f.hit(16, 2_500);
            assert!(
                matches!(
                    result,
                    SharedFishImpactOutcome::Blocked {
                        committed_prefix: false,
                        ..
                    }
                ),
                "{result:?}"
            );
            assert_eq!(f.manager.entity_mut(f.id).unwrap().collision, before);
            assert_eq!(
                f.fx.next_shared_retail_random_u16(),
                expected.next_shared_retail_random_u16()
            );
        }
    }

    #[v2k_test_support::retail_test]
    fn static_route_wraps_only_reaction_force_and_keeps_unscaled_health_damage() {
        for class in [52, 68, 85] {
            // A filtered-zero control and a32-bit overflow distinguish11180
            // force from packet filtering and the ordinary10EB0 wrapper.
            // The large packet also wraps its independent Q8 filter product:
            // (0x20000010 -2000)*256 narrows to-507904, then>>8 gives-1984.
            for (amount, filtered) in [(0_i32, 0), (2_500, 500), (0x2000_0010, -1_984)] {
                let mut f = World::new();
                f.manager.entity_mut(f.id).unwrap().collision.health_raw =
                    RetailRuntimeValue::Known(1_000_000);
                let (mut expected, body) = f.expected_reaction(amount.wrapping_mul(8));
                let result = f.hit(class, amount);
                let SharedFishImpactOutcome::Applied(applied) = result else {
                    panic!("class{class}/amount{amount}: {result:?}")
                };
                assert_eq!(applied.filtered_damage_raw, filtered);
                assert!(applied.death_publication.is_none());
                let entity = f.manager.entity_mut(f.id).unwrap();
                assert_eq!(
                    entity.collision.health_raw,
                    RetailRuntimeValue::Known(1_000_000 - filtered)
                );
                assert_eq!(entity.velocity_raw(), body.linear_velocity_xyz_raw);
                assert_eq!(
                    entity.rotation_heading_pitch_roll_raw(),
                    body.angular_heading_pitch_roll_raw
                );
                assert_eq!(
                    entity.collision.last_hit_presentation_tick_at_0x34,
                    RetailRuntimeValue::Known(701)
                );
                assert_eq!(
                    f.fx.next_shared_retail_random_u16(),
                    expected.next_shared_retail_random_u16()
                );
                assert!(f.scheduler.shared_fish_completed_owner(&f.manager, f.id));
                assert!(f.fx.take_positional_sounds().is_empty());
            }
        }
    }

    #[v2k_test_support::retail_test]
    fn playing_static_route_packets_retire_all_three_authored_level_one_fish() {
        use crate::{
            damage::{
                CLASS68_STATIC_ROUTE_DAMAGE_PACKET, DRAGON_FIREBALL_DAMAGE_PACKET,
                TYPE_47_PROJECTILE_DAMAGE_PACKET,
            },
            gameplay_notifications::GameplayNotificationPhase,
            shared_actor_impact::{
                apply_playing_actor_particle_hit, PlayingActorImpactFrame, SharedActorImpactOutcome,
            },
            specialized_actor_task_production::SpecializedActorTaskProductionFrame,
            static_damage::StaticDamageScheduler,
        };

        for (class, packet) in [
            (52, TYPE_47_PROJECTILE_DAMAGE_PACKET),
            (68, CLASS68_STATIC_ROUTE_DAMAGE_PACKET),
            (85, DRAGON_FIREBALL_DAMAGE_PACKET),
        ] {
            let mut f = World::with_type(13, 62);
            f.manager.cleanup_pending_actor_deferred_destroys();
            let ids: Vec<_> = f
                .manager
                .iter_all()
                .filter(|entity| entity.shared_fish_runtime.is_some())
                .map(|entity| entity.id)
                .collect();
            assert_eq!(ids.len(), 3);
            let mut static_damage = StaticDamageScheduler::new();
            let mut hull = crate::player_hull::PlayerHull::default();
            for tick in 1001..=1010 {
                let pass = f.scheduler.tick(
                    &mut f.manager,
                    SpecializedActorTaskProductionFrame {
                        world: crate::specialized_actor_task_production::SpecializedActorTaskWorld::Cinematic,
                        hive_components: None,
                        notification_phase: GameplayNotificationPhase::Playing,
                        resources: &mut f.session.cache,
                        world_fx: &mut f.fx,
                        static_damage: &mut static_damage,
                        elapsed_micros: 20_000,
                        global_elapsed_micros: 20_000,
                        retail_tick: tick,
                        main_base_abort_active: false,
                    },
                    &mut f.notifications,
                );
                assert!(pass.block.is_none(), "{pass:?}");
            }
            for &id in &ids {
                f.id = id;
                let stamp = f.manager.entity_mut(id).unwrap().construction_stamp_at_0xb4;
                for hit in 0..10 {
                    let (mut expected, body) = f.expected_reaction(packet.impact_sum_raw() * 8);
                    let position_world = f.manager.entity_mut(id).unwrap().position;
                    let outcome = apply_playing_actor_particle_hit(
                        PlayingActorImpactFrame {
                            extra_lives:
                                crate::entity_collision_state::RetailRuntimeValue::Unresolved,
                            resources: &mut f.session.cache,
                            entities: &mut f.manager,
                            world_fx: &mut f.fx,
                            scheduler: &mut f.scheduler,
                            notifications: &mut f.notifications,
                            static_damage: &mut static_damage,
                            player_hull: &mut hull,
                            retail_tick: 1011 + hit,
                        },
                        ParticleEntityImpact {
                            source_particle_class: class,
                            impact_position_argument_va: 0,
                            target_entity_id: id,
                            position_world,
                            velocity_raw: [0, 0, 8192],
                            damage: Some(BallisticDamageRequest {
                                packet,
                                source_entity_type_at_birth: Some(46),
                                source_owner_id: Some(1),
                            }),
                        },
                    );
                    let Some(SharedActorImpactOutcome::Fish(SharedFishImpactOutcome::Applied(
                        applied,
                    ))) = outcome
                    else {
                        panic!("class{class}/id{id}/hit{hit}: {outcome:?}")
                    };
                    assert!(applied.filtered_damage_raw > 0);
                    let entity = f.manager.entity_mut(id).unwrap();
                    assert_eq!(entity.construction_stamp_at_0xb4, stamp);
                    assert_eq!(entity.velocity_raw(), body.linear_velocity_xyz_raw);
                    assert_eq!(
                        entity.rotation_heading_pitch_roll_raw(),
                        body.angular_heading_pitch_roll_raw
                    );
                    assert_eq!(
                        f.fx.next_shared_retail_random_u16(),
                        expected.next_shared_retail_random_u16()
                    );
                    if applied.death_publication.is_some() {
                        assert!(super::super::death::completed_shared_fish_death(
                            &f.manager, id
                        ));
                        assert!(!f.scheduler.shared_fish_completed_owner(&f.manager, id));
                        break;
                    }
                    assert!(hit < 9, "authored packet never reached quiet death");
                }
            }
            assert_eq!(f.manager.pending_actor_deferred_destroy_ids(), ids);
            assert_eq!(f.manager.cleanup_pending_actor_deferred_destroys(), ids);
            assert_eq!(f.scheduler.adopt_shared_fish(&f.manager), 0);
        }
    }

    #[v2k_test_support::retail_test]
    fn infected_hit_stamps_model_bit_without_presentation_tick() {
        let mut f = World::new();
        f.manager.entity_mut(f.id).unwrap().collision.health_raw =
            RetailRuntimeValue::Known(1_000_000);
        let tick_before = f
            .manager
            .entity_mut(f.id)
            .unwrap()
            .collision
            .last_hit_presentation_tick_at_0x34;
        let result = f.hit(5, 0);
        let SharedFishImpactOutcome::Applied(applied) = result else {
            panic!("{result:?}")
        };
        assert!(applied.death_publication.is_none());
        let entity = f.manager.entity_mut(f.id).unwrap();
        assert_eq!(
            entity.collision.last_hit_presentation_tick_at_0x34, tick_before,
            "11250 never stamps the primary presentation tick"
        );
        assert_eq!(
            entity
                .collision
                .state_flags_at_0x08
                .masked(crate::entity_collision_state::ACTIVE_MODEL_SLOT_HIGH_STATE_BIT),
            RetailRuntimeValue::Known(
                crate::entity_collision_state::ACTIVE_MODEL_SLOT_HIGH_STATE_BIT
            )
        );
    }

    #[v2k_test_support::retail_test]
    fn cured_hit_preserves_quiet_death_and_clears_only_the_infected_model_bit() {
        use crate::entity_collision_state::ACTIVE_MODEL_SLOT_HIGH_STATE_BIT;

        let mut f = World::new();
        f.manager.cleanup_pending_actor_deferred_destroys();
        f.manager.entity_mut(f.id).unwrap().collision.health_raw = RetailRuntimeValue::Known(1);
        assert!(matches!(
            f.hit(16, 2500),
            SharedFishImpactOutcome::Applied(ref applied) if applied.death_publication == Some(())
        ));
        let entity = f.manager.entity_mut(f.id).unwrap();
        let context = entity.current_behavior_context;
        entity.collision.state_flags_at_0x08.overwrite(
            ACTIVE_MODEL_SLOT_HIGH_STATE_BIT,
            ACTIVE_MODEL_SLOT_HIGH_STATE_BIT,
        );
        entity.collision.last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(999);
        let mut expected = f.fx.fork_for_main_base_abort_transaction();
        for _ in 0..3 {
            expected.next_shared_retail_random_u16();
        }
        let outcome = f.hit(6, 0);
        assert!(
            matches!(
            outcome,
            SharedFishImpactOutcome::Applied(ref applied)
                if applied.filtered_damage_raw == 800 && applied.death_publication.is_none()
            ),
            "{outcome:?}"
        );
        let entity = f.manager.entity_mut(f.id).unwrap();
        assert_eq!(entity.current_behavior_context, context);
        assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
        assert_eq!(
            entity
                .collision
                .state_flags_at_0x08
                .masked(ACTIVE_MODEL_SLOT_HIGH_STATE_BIT | DYING_STATE_BIT),
            RetailRuntimeValue::Known(DYING_STATE_BIT)
        );
        assert_eq!(
            entity.collision.last_hit_presentation_tick_at_0x34,
            RetailRuntimeValue::Known(999)
        );
        assert_eq!(
            f.fx.next_shared_retail_random_u16(),
            expected.next_shared_retail_random_u16()
        );
        assert!(f.fx.take_positional_sounds().is_empty());
        assert!(super::super::death::completed_shared_fish_death(
            &f.manager, f.id
        ));
        assert!(!f.scheduler.shared_fish_completed_owner(&f.manager, f.id));
        assert_eq!(f.manager.cleanup_pending_actor_deferred_destroys(), [f.id]);
    }
}
