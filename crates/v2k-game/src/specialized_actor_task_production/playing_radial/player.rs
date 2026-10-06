//! Player branch of14AE0, before its synchronous15040/14E10 damage entry.
//!
//! The old retained-player matrix is preserved: bit8000 then800/1000, strict
//! outer radius, mass>=2 and clear8000000 for optional impulse,25430's wrapped
//! damage/impulse return gate, and type35's impulse exemption. The helper owns
//! full-radius versus signed-Q15 falloff and carries both provenance words.
//! An accepted zero impulse still precedes damage. Checked15040 owns the extra
//! admission and zero-filter feedback; falloff14E10 still filters through255E0
//! but has neither gate. Dying players consume only the buffer and keep health
//! and cues. A new death queues hit/death cues then447280's model/burst/CA-or-D9
//! and constructor seed before14AE0 reads this target's next live-list link.
//!
//!14E10 forwards packet+10 but substitutes process DAT_004F7378 for14E90's
//! fourth argument. The complete packet+14 is retained here as packet data;
//! the admitted empty modifier/null hit/local death path never consumes that
//! argument. An owner-dependent continuation must authenticate the live
//! process emitter instead of treating this trailing word as that argument.

use crate::{
    damage::DamageDeliveryRecord,
    entity::{
        DynamicRadialLivePhase, DynamicRadialRuntimeField, DynamicRadialUnresolvedReason,
        EntityManager, PlayerCheckedDamageBlock, PlayerCheckedDamageFrame,
        PlayerCheckedDamageOutcome, PlayerCheckedDamageRequest, PlayerDamageEntry,
    },
    entity_collision_state::{
        RetailRuntimeValue, CHECKED_DAMAGE_ENABLED_STATE_BIT, PAIR_COLLISION_FIXED_STATE_BIT,
        PAIR_COLLISION_INELIGIBLE_STATE_BIT, RADIAL_DAMAGE_ALTERNATE_ELIGIBLE_STATE_BIT,
        REMOTE_OWNED_STATE_BIT,
    },
    gameplay_notifications::GameplayNotifications,
    player_hull::PlayerHull,
    radial_damage::{radial_distance_raw, scale_radial_damage, RadialDamageTemplate},
    resource_cache::ResourceCache,
    world_fx::WorldFx,
};

pub(super) struct PlayerRadialContext<'a> {
    pub player_hull: &'a mut PlayerHull,
    pub extra_lives: RetailRuntimeValue<u8>,
    pub origin_raw: [i16; 3],
    pub template: RadialDamageTemplate,
    pub world_fx: &'a mut WorldFx,
    pub notifications: &'a mut GameplayNotifications,
    pub retail_tick: u32,
    pub resources: &'a ResourceCache,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerRadialBlockReason {
    Runtime(DynamicRadialUnresolvedReason),
    ExtraLivesUnavailable,
    Damage(PlayerCheckedDamageBlock),
    PlayerTargetMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerRadialBlock {
    pub target_id: u32,
    pub phase: DynamicRadialLivePhase,
    ///25430 returned nonzero, even if a later phase blocks.
    pub accepted_target: bool,
    /// A radial impulse can precede a later checked-damage rejection.
    pub target_prefix_committed: bool,
    pub reason: PlayerRadialBlockReason,
}

pub(super) fn apply_player_radial_target(
    entities: &mut EntityManager,
    id: u32,
    context: PlayerRadialContext<'_>,
) -> Result<bool, PlayerRadialBlock> {
    use DynamicRadialLivePhase as Phase;
    let block = |phase, accepted_target, target_prefix_committed, reason| PlayerRadialBlock {
        target_id: id,
        phase,
        accepted_target,
        target_prefix_committed,
        reason,
    };
    let known = |value: RetailRuntimeValue<u32>, field, phase| match value {
        RetailRuntimeValue::Known(value) => Ok(value),
        RetailRuntimeValue::Unresolved => Err(block(
            phase,
            false,
            false,
            PlayerRadialBlockReason::Runtime(DynamicRadialUnresolvedReason::MissingRuntimeField(
                field,
            )),
        )),
    };
    let entity = entities
        .player()
        .filter(|entity| entity.id == id)
        .ok_or_else(|| {
            block(
                Phase::Eligibility,
                false,
                false,
                PlayerRadialBlockReason::PlayerTargetMismatch,
            )
        })?;
    let position = entity.position_raw();
    // These passive reads retain the old outer-miss policy: an unrelated
    // out-of-range allocation cannot block the blast on unknown state.
    if radial_distance_raw(context.origin_raw, position)
        >= i32::from(context.template.outer_radius_raw)
    {
        return Ok(false);
    }
    let enabled = known(
        entity
            .collision
            .state_flags_at_0x08
            .masked(CHECKED_DAMAGE_ENABLED_STATE_BIT),
        DynamicRadialRuntimeField::EligibilityState,
        Phase::Eligibility,
    )?;
    if enabled == 0 {
        return Ok(false);
    }
    let alternate = known(
        entity
            .collision
            .state_flags_at_0x08
            .masked(RADIAL_DAMAGE_ALTERNATE_ELIGIBLE_STATE_BIT),
        DynamicRadialRuntimeField::EligibilityState,
        Phase::Eligibility,
    )?;
    if alternate == 0
        && known(
            entity
                .collision
                .state_flags_at_0x08
                .masked(PAIR_COLLISION_INELIGIBLE_STATE_BIT),
            DynamicRadialRuntimeField::EligibilityState,
            Phase::Eligibility,
        )? != 0
    {
        return Ok(false);
    }
    let impulse_requested = entity.mass_raw >= 2
        && known(
            entity
                .collision
                .state_flags_at_0x08
                .masked(PAIR_COLLISION_FIXED_STATE_BIT),
            DynamicRadialRuntimeField::FixedState,
            Phase::Impulse,
        )? == 0;
    let Some(scaled) = scale_radial_damage(
        context.template,
        context.origin_raw,
        position,
        impulse_requested,
    ) else {
        return Ok(false);
    };
    let impulse = (entity.entity_type != 0x23)
        .then_some(scaled.impulse_vector_raw)
        .flatten();
    if impulse.is_some() {
        let remote = match entity
            .collision
            .state_flags_at_0x08
            .masked(REMOTE_OWNED_STATE_BIT)
        {
            RetailRuntimeValue::Known(value) => value,
            RetailRuntimeValue::Unresolved => {
                return Err(block(
                    Phase::Impulse,
                    true,
                    false,
                    PlayerRadialBlockReason::Runtime(
                        DynamicRadialUnresolvedReason::MissingRuntimeField(
                            DynamicRadialRuntimeField::RemoteOwnership,
                        ),
                    ),
                ));
            }
        };
        if remote != 0 {
            return Err(block(
                Phase::Impulse,
                true,
                false,
                PlayerRadialBlockReason::Runtime(
                    DynamicRadialUnresolvedReason::RemoteDamageDispatchRequired,
                ),
            ));
        }
    }
    let RetailRuntimeValue::Known(extra_lives) = context.extra_lives else {
        return Err(block(
            Phase::MutationCustody,
            true,
            false,
            PlayerRadialBlockReason::ExtraLivesUnavailable,
        ));
    };
    let committed = if let Some(impulse) = impulse {
        let entity = entities
            .entity_mut(id)
            .expect("retained player radial allocation");
        let velocity = entity.velocity_raw();
        entity.set_velocity_raw(std::array::from_fn(|axis| {
            velocity[axis].wrapping_add(impulse[axis])
        }));
        true
    } else {
        false
    };
    let outcome = entities.apply_player_checked_damage(
        PlayerCheckedDamageRequest {
            target_id: id,
            entry: if scaled.checked_damage {
                PlayerDamageEntry::Checked
            } else {
                PlayerDamageEntry::Unchecked
            },
            delivery: DamageDeliveryRecord {
                packet: scaled.packet,
                source_entity_type_raw: scaled.trailing_raw[0] as u32,
                owner_handle: scaled.trailing_raw[1] as u32,
            },
            ratio_numerator: 0,
            ratio_denominator: 0,
        },
        PlayerCheckedDamageFrame {
            hull: context.player_hull,
            resources: context.resources,
            world_fx: context.world_fx,
            retail_tick: context.retail_tick,
            notifications: context.notifications,
            extra_lives,
        },
    );
    match outcome {
        PlayerCheckedDamageOutcome::Applied { .. }
        | PlayerCheckedDamageOutcome::Ineligible
        | PlayerCheckedDamageOutcome::FilteredOut => Ok(true),
        PlayerCheckedDamageOutcome::Blocked(reason) => Err(block(
            Phase::Death,
            true,
            committed,
            PlayerRadialBlockReason::Damage(reason),
        )),
        PlayerCheckedDamageOutcome::NotPlayer => Err(block(
            Phase::Eligibility,
            true,
            committed,
            PlayerRadialBlockReason::PlayerTargetMismatch,
        )),
    }
}
