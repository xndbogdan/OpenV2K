//! `FUN_00416820`'s lethal branch for Type73 and Type109.
//!
//! The modifier's gates run in [`crate::live_actor_checked_damage`]; a lethal
//! hit on an aged, living, local body reaches this owner. Unless channel 3
//! carried the hit, it counts the Type73 and Type109 bodies in the actor list
//! and births two more of its own type (fewer at the 25-body cap, with hint
//! 24 when a player's shot meets it) through `438080`. Each birth draws nine
//! words: a ±256 offset, an X/Z velocity of ±512 with Y 0..511, the parent's
//! heading ±2048 and a pitch and roll of ±2048. A failed birth is disposed
//! and the loop goes on. It then emits particle 90 or 51 at the body, zeroes
//! the buffer and health, and dies through `10C10`.

use super::*;
use crate::{
    gameplay_notifications::GameplayNotifications,
    intro2_common_dying::Intro2CommonDyingBlock,
    live_actor_checked_damage::{LethalSplitRequest, LiveActorDeathResult},
    native_ground_actor::{NativeGroundDeathContext, NativeGroundTerminalPublication},
};

/// Bodies of either splitting type the list may hold before births stop.
pub const LETHAL_SPLIT_BODY_CAP: i32 = 25;

/// One birth record from nine shared words, in the order `416820` draws them.
pub(crate) fn lethal_split_child_request(
    entity_type: u32,
    parent_position_raw: [i16; 3],
    parent_heading_raw: i16,
    objective: bool,
    next_random: &mut impl FnMut() -> u16,
) -> SplitChildRequest {
    let mut offset = |base: i16| {
        (((next_random() >> 7) & 0x1ff) as i16)
            .wrapping_add(base)
            .wrapping_sub(0x100)
    };
    let position_raw = [
        offset(parent_position_raw[0]),
        offset(parent_position_raw[1]),
        offset(parent_position_raw[2]),
    ];
    let vx = ((next_random() >> 6) & 0x3ff) as i16 - 0x200;
    let vy = ((next_random() >> 7) & 0x1ff) as i16;
    let vz = ((next_random() >> 6) & 0x3ff) as i16 - 0x200;
    let mut angle = |base: i16| {
        (((next_random() >> 4) & 0xfff) as i16)
            .wrapping_add(base)
            .wrapping_sub(0x800) as u16
    };
    let heading = angle(parent_heading_raw);
    let pitch = angle(0);
    let roll = angle(0);
    SplitChildRequest {
        // DAT_004DCA00's null handle selects 104B0's fresh handle.
        requested_entity_handle_raw: 0,
        entity_type,
        position_raw,
        objective,
        velocity_raw: [vx, vy, vz],
        rotation_heading_pitch_roll_raw: [heading, pitch, roll],
    }
}

/// How many bodies `416820` births at this count, and whether it asks for
/// the cap hint (the hint also needs a player source).
pub(crate) const fn lethal_split_birth_count(bodies: i32) -> (i32, bool) {
    if bodies + 2 > LETHAL_SPLIT_BODY_CAP {
        (LETHAL_SPLIT_BODY_CAP - bodies, true)
    } else {
        (2, false)
    }
}

pub(crate) fn publish_lethal_split(
    row: Type31Row,
    manager: &mut EntityManager,
    id: u32,
    context: &mut NativeGroundDeathContext<'_>,
    split: LethalSplitRequest,
    notifications: Option<&mut GameplayNotifications>,
) -> Result<LiveActorDeathResult<NativeGroundTerminalPublication>, Intro2CommonDyingBlock> {
    use Intro2CommonDyingBlock as Block;
    if !row.splits_on_lethal_hit()
        || !manager_allocation_authenticates(manager, id)
        || manager
            .iter_all()
            .find(|entity| entity.id == id)
            .and_then(type31_row)
            != Some(row)
    {
        return Err(Block::Runtime("416820 allocation"));
    }
    let NativeGroundDeathContext::Split {
        resources,
        fx,
        tick,
        tasks,
    } = context
    else {
        return Err(Block::Runtime("416820 inline birth custody"));
    };
    if split.births() {
        let bodies = manager
            .iter_all()
            .filter(|entity| matches!(entity.entity_type, 73 | 109))
            .count() as i32;
        let (births, at_cap) = lethal_split_birth_count(bodies);
        if at_cap
            && manager
                .iter_all()
                .find(|entity| entity.id == split.owner_handle)
                .is_some_and(|entity| entity.capability_flags & 1 != 0)
        {
            notifications
                .ok_or(Block::Runtime("416820 cap hint sink"))?
                .queue_split_cap_hint(*tick as i32);
        }
        for _ in 0..births.max(0) {
            let parent = manager
                .iter_all()
                .find(|entity| entity.id == id)
                .ok_or(Block::Runtime("416820 parent"))?;
            let RetailRuntimeValue::Known(objective) =
                parent.collision.state_flags_at_0x08.masked(0x0100_0000)
            else {
                return Err(Block::Runtime("416820 objective bit"));
            };
            let request = lethal_split_child_request(
                row.entity_type(),
                parent.position_raw(),
                parent.rotation_heading_pitch_roll_raw()[0],
                objective != 0,
                &mut || fx.next_shared_retail_random_u16(),
            );
            let birth = manager
                .construct_native_type31_family_child(request, resources, fx, *tick)
                .map_err(|_| Block::Runtime("416820 birth"))?;
            tasks
                .register_split_type31_family_child(birth.owner)
                .map_err(Block::Runtime)?;
        }
    }
    let entity = manager
        .entity_mut(id)
        .ok_or(Block::Runtime("416820 parent"))?;
    let [vx, _, vz] = entity.velocity_raw();
    fx.emit_lethal_split_particle_raw(split.particle_class, entity.position_raw(), [vx, 0, vz], id);
    entity.collision.pre_health_damage_buffer_raw = RetailRuntimeValue::Known(0);
    entity.collision.health_raw = RetailRuntimeValue::Known(0);
    super::death::begin_quiet_standard_death(manager, id, row)
        .map_err(|_| Block::Runtime("416820 quiet death"))
}

/// The radial, pair and attached-particle entry (`414E10`/`415040` through
/// the native checked-damage frame).
#[allow(clippy::too_many_arguments)]
pub(crate) fn radial_lethal_split(
    manager: &mut EntityManager,
    id: u32,
    resources: &crate::resource_cache::ResourceCache,
    fx: &mut WorldFx,
    tick: u32,
    tasks: &mut dyn crate::native_ground_actor::NativeGroundTaskCustody,
    notifications: &mut GameplayNotifications,
    split: LethalSplitRequest,
) -> Result<
    LiveActorDeathResult<crate::entity::DynamicRadialDeathPublication>,
    crate::entity::DynamicRadialLiveBlockReason,
> {
    let row = manager
        .iter_all()
        .find(|entity| entity.id == id)
        .and_then(type31_row)
        .ok_or(crate::entity::DynamicRadialLiveBlockReason::NativeType31(
            death::Type31DeathBlock::Allocation,
        ))?;
    publish_lethal_split(
        row,
        manager,
        id,
        &mut NativeGroundDeathContext::Split {
            resources,
            fx,
            tick,
            tasks,
        },
        split,
        Some(notifications),
    )
    .map(|result| LiveActorDeathResult {
        returned_nonzero: result.returned_nonzero,
        publication: result.publication.map(|terminal| match terminal {
            NativeGroundTerminalPublication::CommonDying(owner) => {
                crate::entity::DynamicRadialDeathPublication::Intro2Class12(owner)
            }
            NativeGroundTerminalPublication::Deferred(receipt) => {
                crate::entity::DynamicRadialDeathPublication::NativeGroundDeferred(receipt)
            }
        }),
    })
    .map_err(|error| {
        crate::entity::DynamicRadialLiveBlockReason::NativeType31(match error {
            Intro2CommonDyingBlock::Runtime(reason) => death::Type31DeathBlock::Runtime(reason),
            _ => death::Type31DeathBlock::Graph,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_birth_record_takes_nine_words_in_retail_order() {
        let words = [
            0x0000u16, 0xffff, 0x8000, 0x0000, 0xffff, 0x8000, 0x0000, 0xffff, 0x8010,
        ];
        let mut draws = words.into_iter();
        let request = lethal_split_child_request(73, [1000, 200, -3000], 0x0400, true, &mut || {
            draws.next().unwrap()
        });
        assert!(draws.next().is_none());
        assert_eq!(request.position_raw, [1000 - 256, 200 + 255, -3000]);
        assert_eq!(request.velocity_raw, [-512, 511, 0]);
        assert_eq!(
            request.rotation_heading_pitch_roll_raw,
            [(0x0400 - 0x800) as u16, 0x7ff, 1]
        );
        assert_eq!(request.entity_type, 73);
        assert!(request.objective);
        assert_eq!(request.requested_entity_handle_raw, 0);
    }

    #[test]
    fn births_stop_at_twenty_five_bodies() {
        assert_eq!(lethal_split_birth_count(1), (2, false));
        assert_eq!(lethal_split_birth_count(23), (2, false));
        assert_eq!(lethal_split_birth_count(24), (1, true));
        assert_eq!(lethal_split_birth_count(25), (0, true));
        assert_eq!(lethal_split_birth_count(30), (-5, true));
    }
}
