//! Local, offline `415040 -> 14E90` execution with a caller-owned death hook.
//!
//! Radial and projectile callers reach this same phase after their different
//! admission, impulse and impact callbacks. Neither caller may replay a failed
//! delivery: buffer, sound and health writes retain their original order.

use crate::{
    damage::{absorb_pre_health_damage_buffer, DamageDeliveryRecord},
    entity::EntityManager,
    entity_collision_state::{
        RetailRuntimeValue, CHECKED_DAMAGE_ENABLED_STATE_BIT, DYING_STATE_BIT,
        REMOTE_OWNED_STATE_BIT,
    },
    gameplay_notifications::GameplayNotifications,
    world_fx::WorldFx,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveActorDamagePhase {
    Admission,
    Filter,
    Modifier,
    RemoteOwner,
    Buffer,
    Dying,
    HitSound,
    HitCallback,
    Health,
    Death,
    PlayerFeedback,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiveActorDamageBlock<E> {
    Unresolved(&'static str),
    Callback(u32),
    RemoteOwned,
    PlayerFeedback,
    Death(E),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveActorDamageError<E, D> {
    pub phase: LiveActorDamagePhase,
    pub committed_prefix: bool,
    pub reason: LiveActorDamageBlock<E>,
    /// A completed death callback survives a later feedback boundary.
    pub death_publication: Option<D>,
}

pub struct LiveActorDamageRequest<'a> {
    pub entity_id: u32,
    pub delivery: DamageDeliveryRecord,
    pub entry: LiveActorDamageEntry,
    /// 4255E0 applies this ratio after the authored channel filter. Only the
    /// low numerator word participates; a zero denominator skips scaling.
    pub ratio_numerator: u32,
    pub ratio_denominator: u16,
    /// The caller owns the current local gameplay notification context. Its
    /// absence preserves the explicit boundary if 14E90 reaches selector4.
    pub feedback: Option<LiveActorDamageFeedback<'a>>,
}

pub struct LiveActorDamageFeedback<'a> {
    pub notifications: &'a mut GameplayNotifications,
    pub retail_tick: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveActorDamageEntry {
    /// 415040: admission bit and filtered-zero player feedback.
    Checked,
    /// 414E10: radial falloff has already passed its outer admission.
    Unchecked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveActorDamageOutcome<D> {
    /// 415040's return value, before the generic buffer consumes any damage.
    pub filtered_damage_raw: i32,
    pub damage_after_buffer_raw: i32,
    pub death_publication: Option<D>,
}

#[derive(Debug)]
pub struct LiveActorDeathResult<D> {
    /// Original 10C10 return, independent of whether a task remains to adopt.
    pub returned_nonzero: bool,
    pub publication: Option<D>,
}

/// Explicit post-filter ratio and no multiplayer session. A present modifier or
/// generic hit callback requires its own live owner; neither is treated as
/// identity. The death closure is entered after the lethal health write and
/// reborrows the current feedback context for synchronous child callbacks.
pub(crate) fn apply_live_actor_checked_damage<D, E>(
    manager: &mut EntityManager,
    world_fx: &mut WorldFx,
    mut request: LiveActorDamageRequest<'_>,
    death: impl FnOnce(
        &mut EntityManager,
        &mut WorldFx,
        Option<&mut LiveActorDamageFeedback<'_>>,
    ) -> Result<LiveActorDeathResult<D>, E>,
) -> Result<LiveActorDamageOutcome<D>, LiveActorDamageError<E, D>> {
    use LiveActorDamagePhase as Phase;
    let error = |phase, committed_prefix, reason| LiveActorDamageError {
        phase,
        committed_prefix,
        reason,
        death_publication: None,
    };
    let unresolved =
        |phase, committed, field| error(phase, committed, LiveActorDamageBlock::Unresolved(field));
    let mut outcome = LiveActorDamageOutcome {
        filtered_damage_raw: 0,
        damage_after_buffer_raw: 0,
        death_publication: None,
    };
    let Some(entity) = manager.entity_mut(request.entity_id) else {
        return Ok(outcome);
    };
    if request.entry == LiveActorDamageEntry::Checked {
        let enabled = known(
            entity
                .collision
                .state_flags_at_0x08
                .masked(CHECKED_DAMAGE_ENABLED_STATE_BIT),
        )
        .ok_or_else(|| unresolved(Phase::Admission, false, "checked-damage bit"))?;
        if enabled == 0 {
            return Ok(outcome);
        }
    }
    let profile = known(entity.collision.damage_profile)
        .ok_or_else(|| unresolved(Phase::Filter, false, "damage profile"))?;
    let filtered = request.delivery.packet.filtered_raw_with_ratio(
        Some(&profile),
        request.ratio_numerator,
        request.ratio_denominator,
    );
    if filtered == 0 {
        if request.entry == LiveActorDamageEntry::Checked
            && request
                .delivery
                .filtered_zero_feedback_required(entity.capability_flags)
        {
            return Err(error(
                Phase::Filter,
                false,
                LiveActorDamageBlock::PlayerFeedback,
            ));
        }
        return Ok(outcome);
    }
    if let Some(address) = known(entity.collision.pair_callbacks.damage_modifier_address)
        .ok_or_else(|| unresolved(Phase::Modifier, false, "damage modifier"))?
    {
        return Err(error(
            Phase::Modifier,
            false,
            LiveActorDamageBlock::Callback(address),
        ));
    }
    if known(
        entity
            .collision
            .state_flags_at_0x08
            .masked(REMOTE_OWNED_STATE_BIT),
    )
    .ok_or_else(|| unresolved(Phase::RemoteOwner, false, "remote ownership"))?
        != 0
    {
        return Err(error(
            Phase::RemoteOwner,
            false,
            LiveActorDamageBlock::RemoteOwned,
        ));
    }
    outcome.filtered_damage_raw = filtered;
    let buffer = known(entity.collision.pre_health_damage_buffer_raw)
        .ok_or_else(|| unresolved(Phase::Buffer, false, "pre-health buffer"))?;
    let (after, remaining) = absorb_pre_health_damage_buffer(buffer, filtered);
    outcome.damage_after_buffer_raw = remaining;
    let mut committed = buffer > 0;
    if committed {
        entity.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(after);
    }
    if known(entity.collision.state_flags_at_0x08.masked(DYING_STATE_BIT))
        .ok_or_else(|| unresolved(Phase::Dying, committed, "dying bit"))?
        != 0
    {
        return Ok(outcome);
    }
    if let Some(sound) = known(entity.collision.generic_hit_sound_id)
        .ok_or_else(|| unresolved(Phase::HitSound, committed, "generic-hit sound"))?
    {
        world_fx.queue_fixed_positional_sound_raw(sound, entity.position_raw());
        committed = true;
    }
    if let Some(address) = known(entity.collision.pair_callbacks.type_hit_callback_address)
        .ok_or_else(|| unresolved(Phase::HitCallback, committed, "generic-hit callback"))?
    {
        return Err(error(
            Phase::HitCallback,
            committed,
            LiveActorDamageBlock::Callback(address),
        ));
    }
    let health = known(entity.collision.health_raw)
        .ok_or_else(|| unresolved(Phase::Health, committed, "health"))?;
    let after = health.wrapping_sub(remaining);
    entity.collision.health_raw = RetailRuntimeValue::Known(after);
    if after > 0 {
        return Ok(outcome);
    }
    let death_result = death(manager, world_fx, request.feedback.as_mut())
        .map_err(|reason| error(Phase::Death, true, LiveActorDamageBlock::Death(reason)))?;
    outcome.death_publication = death_result.publication;
    // 14E90 reads the target bit after successful10C10, then calls568B0(4)
    // for source46. This local resource hint is separate from the following
    // network-statistics branch and from415040's filtered-zero selector23.
    if request.delivery.source_entity_type_raw == 46 && death_result.returned_nonzero {
        let flags = manager
            .entity_mut(request.entity_id)
            .and_then(|entity| known(entity.collision.state_flags_at_0x08.masked(0x0100_0000)));
        let reason = match flags {
            Some(0) => None,
            Some(_) => {
                if let Some(feedback) = request.feedback {
                    feedback
                        .notifications
                        .queue_player_kill(feedback.retail_tick as i32);
                    None
                } else {
                    Some(LiveActorDamageBlock::PlayerFeedback)
                }
            }
            None => Some(LiveActorDamageBlock::Unresolved("player-kill bit")),
        };
        if let Some(reason) = reason {
            let mut failure = error(Phase::PlayerFeedback, true, reason);
            failure.death_publication = outcome.death_publication;
            return Err(failure);
        }
    }
    Ok(outcome)
}

fn known<T>(value: RetailRuntimeValue<T>) -> Option<T> {
    match value {
        RetailRuntimeValue::Known(value) => Some(value),
        RetailRuntimeValue::Unresolved => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        damage::{DamagePacket, DamageProfile},
        entity::{Entity, EntityKind},
        entity_collision_state::RetailStateWord,
    };

    fn fixture(health: i32, buffer: i32) -> EntityManager {
        let mut entity = Entity::unresolved_port_entity(1, EntityKind::Enemy, 53);
        entity.collision.health_raw = RetailRuntimeValue::Known(health);
        entity.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(buffer);
        entity.collision.damage_profile = RetailRuntimeValue::Known(DamageProfile {
            thresholds_raw: [0; 7],
            multipliers_q8: [256; 7],
        });
        entity.collision.state_flags_at_0x08 =
            RetailStateWord::exact(CHECKED_DAMAGE_ENABLED_STATE_BIT);
        entity.collision.generic_hit_sound_id = RetailRuntimeValue::Known(Some(7));
        entity.collision.pair_callbacks.damage_modifier_address = RetailRuntimeValue::Known(None);
        entity.collision.pair_callbacks.type_hit_callback_address = RetailRuntimeValue::Known(None);
        EntityManager::from_entities_for_test(vec![entity])
    }
    fn request<'a>(source: u32, amount: i32) -> LiveActorDamageRequest<'a> {
        LiveActorDamageRequest {
            ratio_numerator: 0,
            ratio_denominator: 0,
            entity_id: 1,
            entry: LiveActorDamageEntry::Checked,
            feedback: None,
            delivery: DamageDeliveryRecord {
                packet: DamagePacket {
                    channels: [1, 0],
                    amounts_raw: [amount, 0],
                },
                source_entity_type_raw: source,
                owner_handle: 99,
            },
        }
    }

    #[test]
    fn live_checked_ratio_follows_authored_filter_before_buffer_and_health() {
        let mut manager = fixture(2, 1);
        let profile = &mut manager.entity_mut(1).unwrap().collision.damage_profile;
        let RetailRuntimeValue::Known(profile) = profile else {
            unreachable!()
        };
        profile.thresholds_raw[2] = 400;
        let mut req = request(u32::MAX - 4, 1_000);
        req.delivery.packet.channels = [2, 0];
        req.ratio_numerator = 0x1234_0001;
        req.ratio_denominator = 255;
        let mut fx = WorldFx::new();
        let outcome =
            apply_live_actor_checked_damage::<(), ()>(&mut manager, &mut fx, req, |_, _, _| {
                panic!("600 filtered units become2, then1 after buffer")
            })
            .unwrap();
        assert_eq!(outcome.filtered_damage_raw, 2);
        assert_eq!(outcome.damage_after_buffer_raw, 1);
        let entity = manager.entity_mut(1).unwrap();
        assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(1));
        assert_eq!(
            entity.collision.pre_health_damage_buffer_raw,
            RetailRuntimeValue::Known(0)
        );
        assert_eq!(fx.pending_event_count(), 1);
    }

    #[test]
    fn live_checked_ratio_zero_stops_before_unowned_modifier_and_keeps_feedback_gate() {
        for source in [u32::MAX - 4, 46] {
            let mut manager = fixture(100, 30);
            let entity = manager.entity_mut(1).unwrap();
            entity.capability_flags = 8;
            entity.collision.pair_callbacks.damage_modifier_address =
                RetailRuntimeValue::Unresolved;
            let mut req = request(source, 1_000);
            req.delivery.packet.channels = [2, 0];
            req.ratio_numerator = 0;
            req.ratio_denominator = 255;
            let mut fx = WorldFx::new();
            let outcome =
                apply_live_actor_checked_damage::<(), ()>(&mut manager, &mut fx, req, |_, _, _| {
                    panic!("zero scaled filter cannot reach death")
                });
            if source == 46 {
                let error = outcome.unwrap_err();
                assert_eq!(error.phase, LiveActorDamagePhase::Filter);
                assert_eq!(error.reason, LiveActorDamageBlock::PlayerFeedback);
                assert!(!error.committed_prefix);
            } else {
                assert_eq!(outcome.unwrap().filtered_damage_raw, 0);
            }
            let entity = manager.entity_mut(1).unwrap();
            assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(100));
            assert_eq!(
                entity.collision.pre_health_damage_buffer_raw,
                RetailRuntimeValue::Known(30)
            );
            assert_eq!(fx.pending_event_count(), 0);
        }
    }

    #[test]
    fn live_checked_ratio_keeps_checked_admission_before_profile_but_unchecked_scales() {
        for entry in [
            LiveActorDamageEntry::Checked,
            LiveActorDamageEntry::Unchecked,
        ] {
            let mut manager = fixture(100, 0);
            let entity = manager.entity_mut(1).unwrap();
            entity.collision.state_flags_at_0x08 = RetailStateWord::exact(0);
            if entry == LiveActorDamageEntry::Checked {
                entity.collision.damage_profile = RetailRuntimeValue::Unresolved;
            }
            let mut req = request(34, 100);
            req.entry = entry;
            req.ratio_numerator = 1;
            req.ratio_denominator = 2;
            let mut fx = WorldFx::new();
            let outcome =
                apply_live_actor_checked_damage::<(), ()>(&mut manager, &mut fx, req, |_, _, _| {
                    panic!("nonlethal or rejected admission")
                })
                .unwrap();
            let expected = if entry == LiveActorDamageEntry::Checked {
                0
            } else {
                50
            };
            assert_eq!(outcome.filtered_damage_raw, expected);
            assert_eq!(
                manager.entity_mut(1).unwrap().collision.health_raw,
                RetailRuntimeValue::Known(100 - expected)
            );
            assert_eq!(fx.pending_event_count(), usize::from(expected != 0));
        }
    }

    #[test]
    fn live_checked_ratio_preserves_signed_wrap_and_dying_buffer_order() {
        let mut manager = fixture(100, 0);
        let mut req = request(u32::MAX - 4, 6_000_000);
        req.ratio_numerator = 500;
        req.ratio_denominator = 255;
        let outcome = apply_live_actor_checked_damage::<(), ()>(
            &mut manager,
            &mut WorldFx::new(),
            req,
            |_, _, _| panic!("negative wrapped damage raises signed health"),
        )
        .unwrap();
        assert_eq!(outcome.filtered_damage_raw, -5_078_303);
        assert_eq!(outcome.damage_after_buffer_raw, -5_078_303);
        assert_eq!(
            manager.entity_mut(1).unwrap().collision.health_raw,
            RetailRuntimeValue::Known(5_078_403)
        );

        let mut manager = fixture(100, 30);
        let mut req = request(u32::MAX - 4, 6_000_000);
        req.ratio_numerator = 500;
        req.ratio_denominator = 255;
        let mut fx = WorldFx::new();
        let outcome =
            apply_live_actor_checked_damage::<(), ()>(&mut manager, &mut fx, req, |_, _, _| {
                panic!("positive buffer absorbs negative damage before health")
            })
            .unwrap();
        assert_eq!(outcome.filtered_damage_raw, -5_078_303);
        assert_eq!(outcome.damage_after_buffer_raw, 0);
        assert_eq!(
            manager
                .entity_mut(1)
                .unwrap()
                .collision
                .pre_health_damage_buffer_raw,
            RetailRuntimeValue::Known(5_078_333)
        );
        assert_eq!(
            manager.entity_mut(1).unwrap().collision.health_raw,
            RetailRuntimeValue::Known(100)
        );

        let mut manager = fixture(100, 30);
        manager
            .entity_mut(1)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .overwrite(DYING_STATE_BIT, DYING_STATE_BIT);
        let mut req = request(u32::MAX - 4, 100);
        req.ratio_numerator = 1;
        req.ratio_denominator = 2;
        let mut fx = WorldFx::new();
        let outcome =
            apply_live_actor_checked_damage::<(), ()>(&mut manager, &mut fx, req, |_, _, _| {
                panic!("already dying")
            })
            .unwrap();
        assert_eq!(outcome.filtered_damage_raw, 50);
        assert_eq!(outcome.damage_after_buffer_raw, 20);
        assert_eq!(
            manager.entity_mut(1).unwrap().collision.health_raw,
            RetailRuntimeValue::Known(100)
        );
        assert_eq!(fx.pending_event_count(), 0);
    }

    #[test]
    fn live_checked_zero_denominator_preserves_unscaled_delivery_and_death_feedback() {
        let mut manager = fixture(1, 0);
        let mut req = request(46, 50);
        req.ratio_numerator = u32::MAX;
        req.ratio_denominator = 0;
        let mut notifications = GameplayNotifications::new();
        req.feedback = Some(LiveActorDamageFeedback {
            notifications: &mut notifications,
            retail_tick: 123,
        });
        let outcome = apply_live_actor_checked_damage::<u32, ()>(
            &mut manager,
            &mut WorldFx::new(),
            req,
            |manager, _, _| {
                let entity = manager.entity_mut(1).unwrap();
                assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(-49));
                entity
                    .collision
                    .state_flags_at_0x08
                    .overwrite(0x0100_0000, 0x0100_0000);
                Ok(LiveActorDeathResult {
                    returned_nonzero: true,
                    publication: Some(71),
                })
            },
        )
        .unwrap();
        assert_eq!(outcome.filtered_damage_raw, 50);
        assert_eq!(outcome.death_publication, Some(71));
        let mut expected = GameplayNotifications::new();
        expected.queue_player_kill(123);
        assert_eq!(notifications, expected);
    }

    #[test]
    fn live_checked_damage_keeps_buffer_and_sound_before_blocked_hit_callback() {
        let mut manager = fixture(100, 30);
        manager
            .entity_mut(1)
            .unwrap()
            .collision
            .pair_callbacks
            .type_hit_callback_address = RetailRuntimeValue::Known(Some(0x1234));
        let mut fx = WorldFx::new();
        let result = apply_live_actor_checked_damage::<(), ()>(
            &mut manager,
            &mut fx,
            request(34, 20),
            |_, _, _feedback| panic!("hit callback must complete before death"),
        );
        let error = result.unwrap_err();
        assert_eq!(error.phase, LiveActorDamagePhase::HitCallback);
        assert!(error.committed_prefix);
        let entity = manager.entity_mut(1).unwrap();
        assert_eq!(
            entity.collision.pre_health_damage_buffer_raw,
            RetailRuntimeValue::Known(10)
        );
        assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(100));
        assert_eq!(fx.pending_event_count(), 1);
    }

    #[test]
    fn live_checked_damage_dying_consumes_buffer_without_hit_sound_or_health() {
        let mut manager = fixture(100, 30);
        manager
            .entity_mut(1)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .overwrite(DYING_STATE_BIT, DYING_STATE_BIT);
        let mut fx = WorldFx::new();
        let result = apply_live_actor_checked_damage::<(), ()>(
            &mut manager,
            &mut fx,
            request(34, 50),
            |_, _, _feedback| panic!("already dying"),
        )
        .unwrap();
        assert_eq!(
            (result.filtered_damage_raw, result.damage_after_buffer_raw),
            (50, 20)
        );
        assert_eq!(
            manager.entity_mut(1).unwrap().collision.health_raw,
            RetailRuntimeValue::Known(100)
        );
        assert_eq!(fx.pending_event_count(), 0);
    }

    #[test]
    fn live_checked_damage_feedback_block_retains_completed_death_receipt() {
        for publication in [None, Some(71_u32)] {
            let mut manager = fixture(1, 0);
            manager
                .entity_mut(1)
                .unwrap()
                .collision
                .state_flags_at_0x08
                .overwrite(0x0100_0000, 0x0100_0000);
            let result = apply_live_actor_checked_damage::<u32, ()>(
                &mut manager,
                &mut WorldFx::new(),
                request(46, 50),
                |manager, _, _feedback| {
                    assert_eq!(
                        manager.entity_mut(1).unwrap().collision.health_raw,
                        RetailRuntimeValue::Known(-49)
                    );
                    Ok(LiveActorDeathResult {
                        returned_nonzero: true,
                        publication,
                    })
                },
            )
            .unwrap_err();
            assert_eq!(result.phase, LiveActorDamagePhase::PlayerFeedback);
            assert_eq!(result.death_publication, publication);
        }
    }

    #[test]
    fn live_checked_damage_unchecked_filter_zero_does_not_request_player_feedback() {
        for entry in [
            LiveActorDamageEntry::Checked,
            LiveActorDamageEntry::Unchecked,
        ] {
            let mut manager = fixture(100, 0);
            manager.entity_mut(1).unwrap().capability_flags = 8;
            let mut req = request(46, 0);
            req.delivery.packet.channels = [3, 0];
            req.entry = entry;
            let mut notifications = GameplayNotifications::new();
            req.feedback = Some(LiveActorDamageFeedback {
                notifications: &mut notifications,
                retail_tick: 123,
            });
            let result = apply_live_actor_checked_damage::<(), ()>(
                &mut manager,
                &mut WorldFx::new(),
                req,
                |_, _, _feedback| panic!("zero filtered damage"),
            );
            assert_eq!(result.is_ok(), entry == LiveActorDamageEntry::Unchecked);
            if let Err(error) = result {
                assert_eq!(error.phase, LiveActorDamagePhase::Filter);
                assert_eq!(error.reason, LiveActorDamageBlock::PlayerFeedback);
                assert!(!error.committed_prefix);
            }
            assert_eq!(notifications, GameplayNotifications::new());
        }
    }

    #[test]
    fn live_checked_damage_queues_player_kill_only_after_the_completed_death_callback() {
        let mut notifications = GameplayNotifications::new();
        let mut expected = GameplayNotifications::new();
        expected.queue_hive_destroyed_hint(123);
        expected.queue_player_kill(123);
        // Two distinct killed allocations share the session feedback mask;
        // the second success must not refresh the first event's timestamp.
        for (tick, publication) in [(123, Some(71_u32)), (456, None)] {
            let mut manager = fixture(1, 0);
            let mut fx = WorldFx::new();
            let mut oracle = fx.fork_for_main_base_abort_transaction();
            let mut req = request(46, 50);
            req.feedback = Some(LiveActorDamageFeedback {
                notifications: &mut notifications,
                retail_tick: tick,
            });
            let result = apply_live_actor_checked_damage::<u32, ()>(
                &mut manager,
                &mut fx,
                req,
                |manager, fx, feedback| {
                    // Nested lifecycle work shares the live sink before the
                    // outer kill hint overwrites its visible resource slot.
                    let feedback = feedback.expect("current callback feedback");
                    assert_eq!(feedback.retail_tick, tick);
                    feedback
                        .notifications
                        .queue_hive_destroyed_hint(tick as i32);
                    let entity = manager.entity_mut(1).unwrap();
                    assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(-49));
                    assert_eq!(
                        fx.pending_event_count(),
                        1,
                        "generic hit sound precedes death"
                    );
                    // The pre-death bit was clear: feedback must read this
                    // callback's completed state, independently of its owner.
                    entity
                        .collision
                        .state_flags_at_0x08
                        .overwrite(DYING_STATE_BIT | 0x0100_0000, DYING_STATE_BIT | 0x0100_0000);
                    fx.queue_fixed_positional_sound_raw(94, entity.position_raw());
                    Ok(LiveActorDeathResult {
                        returned_nonzero: true,
                        publication,
                    })
                },
            )
            .unwrap();
            assert_eq!(result.death_publication, publication);
            assert_eq!(result.filtered_damage_raw, 50);
            assert_eq!(result.damage_after_buffer_raw, 50);
            assert_eq!(notifications, expected);
            fx.process_pending();
            assert_eq!(
                fx.take_positional_sounds()
                    .iter()
                    .map(|event| event.sound_id)
                    .collect::<Vec<_>>(),
                [7, 94]
            );
            assert_eq!(
                fx.next_shared_retail_random_u16(),
                oracle.next_shared_retail_random_u16()
            );

            let mut req = request(46, 50);
            req.feedback = Some(LiveActorDamageFeedback {
                notifications: &mut notifications,
                retail_tick: tick + 1,
            });
            let repeated = apply_live_actor_checked_damage::<u32, ()>(
                &mut manager,
                &mut fx,
                req,
                |_, _, _feedback| panic!("already dying cannot replay death"),
            )
            .unwrap();
            assert_eq!(repeated.death_publication, None);
            assert_eq!(notifications, expected);
            assert_eq!(fx.pending_event_count(), 0);
        }
    }

    #[test]
    fn live_checked_damage_feedback_retains_source_return_and_post_death_state_gates() {
        for (source, returned_nonzero, post_death_bit) in [
            (34, true, RetailRuntimeValue::Known(0x0100_0000)),
            (46, false, RetailRuntimeValue::Unresolved),
            (46, true, RetailRuntimeValue::Known(0)),
            (46, true, RetailRuntimeValue::Unresolved),
        ] {
            let mut manager = fixture(1, 0);
            manager
                .entity_mut(1)
                .unwrap()
                .collision
                .state_flags_at_0x08
                .overwrite(0x0100_0000, 0x0100_0000);
            let mut fx = WorldFx::new();
            let mut notifications = GameplayNotifications::new();
            let mut req = request(source, 50);
            req.feedback = Some(LiveActorDamageFeedback {
                notifications: &mut notifications,
                retail_tick: 123,
            });
            let result = apply_live_actor_checked_damage::<u32, ()>(
                &mut manager,
                &mut fx,
                req,
                |manager, _, _feedback| {
                    let state = &mut manager.entity_mut(1).unwrap().collision.state_flags_at_0x08;
                    match post_death_bit {
                        RetailRuntimeValue::Known(value) => state.overwrite(0x0100_0000, value),
                        RetailRuntimeValue::Unresolved => state.invalidate(0x0100_0000),
                    }
                    Ok(LiveActorDeathResult {
                        returned_nonzero,
                        publication: Some(71),
                    })
                },
            );
            if source == 46 && returned_nonzero && post_death_bit == RetailRuntimeValue::Unresolved
            {
                let error = result.unwrap_err();
                assert_eq!(error.phase, LiveActorDamagePhase::PlayerFeedback);
                assert_eq!(
                    error.reason,
                    LiveActorDamageBlock::Unresolved("player-kill bit")
                );
                assert!(error.committed_prefix);
                assert_eq!(error.death_publication, Some(71));
            } else {
                assert_eq!(result.unwrap().death_publication, Some(71));
            }
            assert_eq!(notifications, GameplayNotifications::new());
            assert_eq!(
                manager.entity_mut(1).unwrap().collision.health_raw,
                RetailRuntimeValue::Known(-49)
            );
        }
    }
}
