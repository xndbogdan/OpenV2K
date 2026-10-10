//! Native Type15/87 11AD0 surface admission and task custody.
//! Their null Search hooks run the same shared141D0 phases as other flyers.

use crate::{
    entity::Entity,
    entity_collision_state::{active_model_slot_from_state_flags, RetailRuntimeValue},
    flying_surface_contact::resolve_null_hook_flying_surface,
    intro2_contacts::Intro2ContactFrame,
    intro2_flyers_live::Intro2FlyerSchedulerOwner,
    native_ground_actor::contact::NativeGroundContactOutcome,
};
#[cfg(test)]
use crate::{
    type60_exploding_ring::Type60ConstructionOutcome, whole_body_surface::WholeBodySurfaceResponse,
};

pub type Intro2FlyerContactBlock =
    crate::native_flying_surface_contact::NativeFlyingSurfaceContactBlock;
pub type Intro2FlyerContactOutcome =
    crate::native_flying_surface_contact::NativeFlyingSurfaceContactOutcome;

/// The complete Type15/87 surface/static prefix of the source11AD0 visit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeFlyerContactOutcome {
    pub surface: Intro2FlyerContactOutcome,
    /// None means the surface phase blocked before static contact was visited.
    pub static_contact: Option<NativeGroundContactOutcome>,
}

impl NativeFlyerContactOutcome {
    /// A blocked source prefix cannot continue into the later active-pair scan.
    pub fn blocks_later_contacts(&self) -> bool {
        matches!(&self.surface, Intro2FlyerContactOutcome::Blocked { .. })
            || matches!(
                &self.static_contact,
                Some(NativeGroundContactOutcome::Blocked { .. })
            )
    }
}

/// Share the existing Intro2 decision matrix with ordinary native Hive children
/// and receipt-bearing ordinary Type13: retain the entry model before surface
/// callbacks; a blocked surface skips static/pairs, an ineligible surface uses
/// static's own admission, and an admitted surface uses its retained-model
/// suffix. A blocked static skips pairs. Type13 keeps its own 13/10/57 surface
/// kernel; Intro2's spawn0 keeps the cinematic walk's own sequence.
pub fn resolve_native_flyer_contacts(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
) -> NativeFlyerContactOutcome {
    let Some(entity) = frame.entities.iter_all().find(|entity| entity.id == id) else {
        return NativeFlyerContactOutcome {
            surface: Intro2FlyerContactOutcome::Ineligible,
            static_contact: Some(NativeGroundContactOutcome::Ineligible),
        };
    };
    let ordinary_type13 = entity.entity_type == 13 && entity.native_type13_allocation.is_some();
    if !matches!(entity.entity_type, 15 | 87) && !ordinary_type13 {
        return NativeFlyerContactOutcome {
            surface: Intro2FlyerContactOutcome::Ineligible,
            static_contact: Some(NativeGroundContactOutcome::Ineligible),
        };
    }
    let entry_model_id = match entity.collision.state_flags_at_0x08.masked(0x6000) {
        RetailRuntimeValue::Known(bits) => {
            entity.model_in_slot(active_model_slot_from_state_flags(bits))
        }
        RetailRuntimeValue::Unresolved => None,
    };
    let surface = if ordinary_type13 {
        crate::native_flying_surface_contact::resolve_native_flying_surface_contact(frame, id)
    } else {
        resolve_intro2_flyer_surface_contact(frame, id)
    };
    let static_contact = match &surface {
        Intro2FlyerContactOutcome::Blocked { .. } => None,
        Intro2FlyerContactOutcome::Ineligible => {
            Some(crate::native_ground_actor::contact::resolve_flying_static_contact(frame, id))
        }
        Intro2FlyerContactOutcome::Applied { .. } => Some(
            crate::native_ground_actor::contact::resolve_flying_static_contact_continuation(
                frame,
                id,
                entry_model_id.expect("admitted flyer entry model"),
            ),
        ),
    };
    NativeFlyerContactOutcome {
        surface,
        static_contact,
    }
}

pub fn resolve_intro2_flyer_surface_contact(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
) -> Intro2FlyerContactOutcome {
    let mut committed = false;
    match resolve(frame, id, &mut committed) {
        Ok(outcome) => outcome,
        Err(reason) => {
            if committed {
                if !frame
                    .actor_tasks
                    .park_native_contact_prefix(frame.entities, id)
                {
                    frame.actor_tasks.park_intro2_flyer_contact_prefix(id);
                }
            }
            Intro2FlyerContactOutcome::Blocked {
                reason,
                committed_prefix: committed,
            }
        }
    }
}

fn bits(entity: &Entity, mask: u32) -> Result<u32, Intro2FlyerContactBlock> {
    match entity.collision.state_flags_at_0x08.masked(mask) {
        RetailRuntimeValue::Known(value) => Ok(value),
        RetailRuntimeValue::Unresolved => Err(Intro2FlyerContactBlock::Runtime("contact state")),
    }
}

fn resolve(
    frame: &mut Intro2ContactFrame<'_>,
    id: u32,
    committed: &mut bool,
) -> Result<Intro2FlyerContactOutcome, Intro2FlyerContactBlock> {
    use Intro2FlyerContactBlock as Block;
    let entity = frame
        .entities
        .iter_all()
        .find(|e| e.id == id)
        .ok_or(Block::Runtime("allocation"))?;
    if !entity.active || !matches!(entity.entity_type, 15 | 87) {
        return Ok(Intro2FlyerContactOutcome::Ineligible);
    }
    let entity_type = entity.entity_type;
    // Source-ineligible actors never require callback/task custody or a basis.
    if bits(entity, 0x8000)? == 0 || bits(entity, 0x1000)? != 0 {
        return Ok(Intro2FlyerContactOutcome::Ineligible);
    }
    match entity.collision.subject_scan_gate_at_0x70 {
        RetailRuntimeValue::Known(0) => {}
        RetailRuntimeValue::Known(_) => return Ok(Intro2FlyerContactOutcome::Ineligible),
        RetailRuntimeValue::Unresolved => return Err(Block::Runtime("subject +70")),
    }
    let model_id = entity
        .model_in_slot(active_model_slot_from_state_flags(bits(entity, 0x6000)?))
        .ok_or(Block::Runtime("entry model"))?;
    let model = frame
        .resources
        .global_model(model_id)
        .ok_or(Block::Runtime("entry model"))?;
    let radius = model.collision_radius_raw;
    if radius == 0 || bits(entity, 0x8800_0000)? != 0 || bits(entity, 0x10000)? == 0 {
        return Ok(Intro2FlyerContactOutcome::Ineligible);
    }
    let RetailRuntimeValue::Known(Some(style)) = entity.current_behavior_context else {
        return Err(Block::Runtime("current style"));
    };
    if !matches!(
        style.active_style().style_address(),
        0x004c7a50 | 0x004c7a98 | 0x004c7420
    ) {
        return Err(Block::Runtime("native flyer solid/water style hooks"));
    }
    if style.active_style().style_address() == 0x004c7420 {
        if !crate::native_flying_surface_contact::native_flyer_quiet_death_authenticates(
            frame.entities,
            id,
        ) {
            return Err(Block::Runtime("native flyer quiet terminal custody"));
        }
    } else {
        Intro2FlyerSchedulerOwner::adopt_published(entity)
            .map_err(|_| Block::Runtime("native flyer graph"))?;
        if !frame
            .actor_tasks
            .intro2_flyer_completed_owner(frame.entities, id)
        {
            return Err(Block::Runtime("current completed contact owner"));
        }
    }
    let record = frame
        .resources
        .global_entity_type(entity_type as usize)
        .ok_or(Block::Runtime("type record"))?;
    if record.raw_header.get(0x86..0x8a) != Some(&[0, 0, 0, 0]) {
        return Err(Block::Runtime("native flyer contact cues"));
    }
    resolve_null_hook_flying_surface(
        frame,
        id,
        model_id,
        0,
        0,
        committed,
        |death_frame| {
            crate::native_flying_surface_contact::run_native_flying_standard_death(death_frame, id)
        },
        crate::native_flying_surface_contact::register_native_flying_death,
        |_, _, _| Ok(()),
    )
}

#[cfg(test)]
mod tests;
