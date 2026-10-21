//! FUN_004425D0's direct, scaled checked damage. No primary hit, style hit,
//! impact impulse or radial falloff runs at this entry.

use crate::{
    damage::{
        DamageDeliveryRecord, CLASS68_STATIC_ROUTE_DAMAGE_PACKET, DRAGON_FIREBALL_DAMAGE_PACKET,
        TYPE_47_PROJECTILE_DAMAGE_PACKET,
    },
    entity::{
        DynamicRadialLiveBlock, DynamicRadialLiveBlockReason, DynamicRadialLiveOutcome,
        DynamicRadialLivePhase,
    },
    entity_collision_state::{RetailRuntimeValue, CHECKED_DAMAGE_ENABLED_STATE_BIT},
    intro2_radial::{Intro2RadialCallbacks, Intro2RadialTaskCustody},
    live_actor_checked_damage::{
        apply_live_actor_checked_damage, LiveActorDamageEntry, LiveActorDamageError,
        LiveActorDamageFeedback, LiveActorDamagePhase, LiveActorDamageRequest,
    },
    native_checked_damage::{
        apply_native_actor_checked_damage, prepare_native_actor_damage_mutation,
        NativeCheckedDamageContext, NativeMutationCaller,
    },
    world_fx::AttachedParticleDamageRequest,
};

pub struct AttachedParticleDamageFrame<'a> {
    pub resources: &'a mut crate::resource_cache::ResourceCache,
    pub entities: &'a mut crate::entity::EntityManager,
    pub world_fx: &'a mut crate::world_fx::WorldFx,
    pub static_damage: &'a mut crate::static_damage::StaticDamageScheduler,
    pub scheduler: &'a mut crate::specialized_actor_task_production::SpecializedActorTaskScheduler,
    pub notifications: &'a mut crate::gameplay_notifications::GameplayNotifications,
    pub retail_tick: u32,
    pub world: AttachedParticleDamageWorld<'a>,
}

pub enum AttachedParticleDamageWorld<'a> {
    Cinematic,
    Playing {
        player_hull: &'a mut crate::player_hull::PlayerHull,
        extra_lives: RetailRuntimeValue<u8>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttachedParticleDamageBlock {
    MutableOwnerUnavailable,
    PacketAddress(u32),
    Native(Box<DynamicRadialLiveBlock>),
    Fish(Box<LiveActorDamageError<crate::shared_fish::death::SharedFishDeathBlock, ()>>),
    Player(crate::entity::PlayerCheckedDamageBlock),
    PlayerTargetMismatch,
    PlayerHullUnavailable,
    PlayerExtraLivesUnavailable,
}

impl AttachedParticleDamageRequest {
    pub fn delivery(self) -> Result<DamageDeliveryRecord, AttachedParticleDamageBlock> {
        let packet = match self.packet_va {
            0x004c_c0f0 => TYPE_47_PROJECTILE_DAMAGE_PACKET,
            0x004c_c108 => CLASS68_STATIC_ROUTE_DAMAGE_PACKET,
            0x004c_c048 => DRAGON_FIREBALL_DAMAGE_PACKET,
            address => return Err(AttachedParticleDamageBlock::PacketAddress(address)),
        };
        Ok(DamageDeliveryRecord {
            packet,
            source_entity_type_raw: self.source_entity_type_raw,
            owner_handle: self.current_emitter.map_or(0, |owner| owner.entity_id),
        })
    }
}

/// The mutable world includes nested class49 radial calls and their terrain
/// writes. This function enters15040 directly without a primary/style hit.
pub fn apply_attached_particle_damage(
    frame: AttachedParticleDamageFrame<'_>,
    request: AttachedParticleDamageRequest,
) -> Result<i32, AttachedParticleDamageBlock> {
    let id = request.target_handle;
    let delivery = request.delivery()?;
    if frame
        .entities
        .player()
        .is_some_and(|entity| entity.id == id)
    {
        let AttachedParticleDamageWorld::Playing {
            player_hull,
            extra_lives,
        } = frame.world
        else {
            return Err(AttachedParticleDamageBlock::PlayerHullUnavailable);
        };
        let RetailRuntimeValue::Known(extra_lives) = extra_lives else {
            return Err(AttachedParticleDamageBlock::PlayerExtraLivesUnavailable);
        };
        return match frame.entities.apply_player_checked_damage(
            crate::entity::PlayerCheckedDamageRequest {
                target_id: id,
                entry: crate::entity::PlayerDamageEntry::Checked,
                delivery,
                ratio_numerator: u32::from(request.ratio_numerator),
                ratio_denominator: request.ratio_denominator,
            },
            crate::entity::PlayerCheckedDamageFrame {
                hull: player_hull,
                resources: frame.resources,
                world_fx: frame.world_fx,
                retail_tick: frame.retail_tick,
                notifications: frame.notifications,
                extra_lives,
            },
        ) {
            crate::entity::PlayerCheckedDamageOutcome::Applied {
                filtered_damage_raw,
                ..
            } => Ok(filtered_damage_raw),
            crate::entity::PlayerCheckedDamageOutcome::Ineligible
            | crate::entity::PlayerCheckedDamageOutcome::FilteredOut => Ok(0),
            crate::entity::PlayerCheckedDamageOutcome::Blocked(reason) => {
                Err(AttachedParticleDamageBlock::Player(reason))
            }
            crate::entity::PlayerCheckedDamageOutcome::NotPlayer => {
                Err(AttachedParticleDamageBlock::PlayerTargetMismatch)
            }
        };
    }
    apply_actor_damage(
        frame,
        LiveActorDamageRequest {
            entity_id: id,
            delivery,
            entry: LiveActorDamageEntry::Checked,
            ratio_numerator: u32::from(request.ratio_numerator),
            ratio_denominator: request.ratio_denominator,
            // Attached's source-5 cannot reach either player feedback branch.
            feedback: None,
        },
    )
}

/// `FUN_00417E10`: a peer's kind 9 for a body this machine owns, other than
/// the player, enters `FUN_00414E90` with the sender's amount and attacker
/// type, as directly as `FUN_004425D0` enters `FUN_00415040`.
pub fn apply_network_damage(
    frame: AttachedParticleDamageFrame<'_>,
    entity_id: u32,
    amount: u16,
    attacker_type: u16,
) -> Result<i32, AttachedParticleDamageBlock> {
    apply_actor_damage(
        frame,
        LiveActorDamageRequest {
            entity_id,
            delivery: DamageDeliveryRecord {
                source_entity_type_raw: u32::from(attacker_type),
                ..DamageDeliveryRecord::default()
            },
            entry: LiveActorDamageEntry::Network { amount },
            ratio_numerator: 0,
            ratio_denominator: 0,
            feedback: None,
        },
    )
}

/// The custody, fish and native phases of a direct entry past the player.
fn apply_actor_damage(
    mut frame: AttachedParticleDamageFrame<'_>,
    live_request: LiveActorDamageRequest<'_>,
) -> Result<i32, AttachedParticleDamageBlock> {
    let id = live_request.entity_id;
    // Custody is required only on the nonzero path that can write or enter
    // callbacks. Admission-clear and filtered-zero are real15040 zero returns;
    // 14E90 itself always writes.
    let mutation_requested = matches!(live_request.entry, LiveActorDamageEntry::Network { .. })
        || frame
            .entities
            .iter_all()
            .find(|entity| entity.id == id)
            .is_some_and(|entity| {
                entity
                    .collision
                    .state_flags_at_0x08
                    .masked(CHECKED_DAMAGE_ENABLED_STATE_BIT)
                    == RetailRuntimeValue::Known(CHECKED_DAMAGE_ENABLED_STATE_BIT)
                    && matches!(entity.collision.damage_profile, RetailRuntimeValue::Known(profile)
                        if live_request.delivery.packet.filtered_raw_with_ratio(Some(&profile),
                            live_request.ratio_numerator, live_request.ratio_denominator) != 0)
            });
    if mutation_requested
        && !prepare_native_actor_damage_mutation(
            frame.entities,
            id,
            NativeMutationCaller::Attached,
            &mut |entities: &crate::entity::EntityManager, id| {
                frame.scheduler.prepare_native_actor_mutation(entities, id)
            },
        )
    {
        return Err(AttachedParticleDamageBlock::Native(Box::new(
            DynamicRadialLiveBlock {
                target_id: id,
                phase: DynamicRadialLivePhase::MutationCustody,
                target_prefix_committed: false,
                reason: DynamicRadialLiveBlockReason::NativeActorMutationCustody,
            },
        )));
    }
    let fish = frame
        .entities
        .iter_all()
        .find(|entity| entity.id == id)
        .is_some_and(|entity| entity.shared_fish_runtime.is_some());
    if fish {
        // The death reborrows this frame's notifications through the
        // feedback, which a network kill by a craft (type 46) also needs.
        let live_request = LiveActorDamageRequest {
            feedback: Some(LiveActorDamageFeedback {
                notifications: &mut *frame.notifications,
                retail_tick: frame.retail_tick,
            }),
            ..live_request
        };
        let auto_pilot = frame
            .entities
            .iter_all()
            .any(|entity| entity.id == id && entity.entity_type == 124);
        let result = apply_live_actor_checked_damage(
            frame.entities,
            frame.world_fx,
            live_request,
            |entities, world_fx, feedback| {
                if !auto_pilot {
                    return crate::shared_fish::death::begin_shared_fish_standard_death(
                        entities,
                        id,
                        world_fx,
                        frame.scheduler,
                    );
                }
                // Type124's class63 takes BAF0's radial with this frame's world.
                use crate::class49_terminal::Class49WorldContext;
                crate::shared_fish::death::run_type124_auto_pilot_death(
                    crate::class49_terminal::Class49TerminalFrame {
                        entities,
                        resources: &mut *frame.resources,
                        world_fx,
                        static_damage: &mut *frame.static_damage,
                        notifications: &mut *feedback
                            .expect("fish damage lends its feedback")
                            .notifications,
                        retail_tick: frame.retail_tick,
                        world: match &mut frame.world {
                            AttachedParticleDamageWorld::Cinematic => {
                                Class49WorldContext::Cinematic {
                                    actor_tasks: &mut *frame.scheduler,
                                    active_terminal_calls: Vec::new(),
                                }
                            }
                            AttachedParticleDamageWorld::Playing {
                                player_hull,
                                extra_lives,
                            } => Class49WorldContext::Playing {
                                scheduler: &mut *frame.scheduler,
                                player_hull,
                                extra_lives: *extra_lives,
                                active_terminal_calls: Vec::new(),
                            },
                        },
                    },
                    id,
                )
            },
        );
        if match &result {
            Ok(outcome) => outcome.death_publication.is_some(),
            Err(error) => error.death_publication.is_some(),
        } {
            frame.scheduler.retire_shared_fish(frame.entities, id);
        }
        return result
            .map(|outcome| outcome.filtered_damage_raw)
            .map_err(|error| {
                if error.committed_prefix {
                    if let Some(runtime) = frame
                        .entities
                        .entity_mut(id)
                        .and_then(|entity| entity.shared_fish_runtime.as_mut())
                    {
                        runtime.impact_prefix_pending = true;
                    }
                }
                AttachedParticleDamageBlock::Fish(Box::new(error))
            });
    }
    let result = match frame.world {
        AttachedParticleDamageWorld::Cinematic => apply_native_actor_checked_damage(
            frame.entities, live_request, NativeCheckedDamageContext {
                world_fx: frame.world_fx, retail_tick: frame.retail_tick,
                notifications: frame.notifications,
                callbacks: &mut Intro2RadialCallbacks {
                    active_terminal_calls: &[], resources: frame.resources,
                    static_damage: frame.static_damage, actor_tasks: frame.scheduler,
                },
            }),
        AttachedParticleDamageWorld::Playing { player_hull, extra_lives } => apply_native_actor_checked_damage(
            frame.entities, live_request, NativeCheckedDamageContext {
                world_fx: frame.world_fx, retail_tick: frame.retail_tick,
                notifications: frame.notifications,
                callbacks: &mut crate::specialized_actor_task_production::playing_radial::PlayingNativeCallbacks {
                    active_terminal_calls: &[], resources: frame.resources,
                    static_damage: frame.static_damage, scheduler: frame.scheduler, player_hull, extra_lives,
                },
            }),
    };
    result
        .map(|outcome| outcome.filtered_damage_raw)
        .map_err(|error| {
            let phase = match error.phase {
                LiveActorDamagePhase::Admission => DynamicRadialLivePhase::Eligibility,
                LiveActorDamagePhase::Filter => DynamicRadialLivePhase::Filter,
                LiveActorDamagePhase::Modifier => DynamicRadialLivePhase::Modifier,
                LiveActorDamagePhase::RemoteOwner | LiveActorDamagePhase::Buffer => {
                    DynamicRadialLivePhase::Buffer
                }
                LiveActorDamagePhase::Dying => DynamicRadialLivePhase::Dying,
                LiveActorDamagePhase::HitSound => DynamicRadialLivePhase::HitSound,
                LiveActorDamagePhase::HitCallback => DynamicRadialLivePhase::TypeHit,
                LiveActorDamagePhase::Health => DynamicRadialLivePhase::Health,
                LiveActorDamagePhase::Death | LiveActorDamagePhase::PlayerFeedback => {
                    DynamicRadialLivePhase::Death
                }
            };
            let block = DynamicRadialLiveBlock {
                target_id: id,
                phase,
                target_prefix_committed: error.committed_prefix,
                reason: DynamicRadialLiveBlockReason::Checked(Box::new(error)),
            };
            if block.target_prefix_committed {
                frame.scheduler.park_attached_damage_prefix(id);
            }
            frame
                .scheduler
                .retain_dynamic_result(&DynamicRadialLiveOutcome {
                    blocked: Some(block.clone()),
                    ..Default::default()
                });
            AttachedParticleDamageBlock::Native(Box::new(block))
        })
}
