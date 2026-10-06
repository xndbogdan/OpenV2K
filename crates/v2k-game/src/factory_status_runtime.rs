//! Pure Working Factory status-word projection.
//!
//! `FUN_00419630` publishes six low words after the production owner has
//! completed its animation transition.  The loader-owned Sub-M descriptor
//! then routes those retained words to the Section-8 model callback. This
//! module closes only that deterministic state transform. Receipt, allocation,
//! and mutable-version authentication remain with the owning live adapter;
//! `factory_activation_live` supplies one only for its bounded fresh-Level-1
//! prefix. Accepting a freely constructed publication here must not bypass
//! that owner.

use crate::entity::BaseFactoryRuntimeState;
use crate::factory_production_live::FactoryStatusPublication;

pub const WORKING_FACTORY_ENTITY_TYPE: u32 = 66;

/// The exact before/after Sub-M word bank derived from one status publication.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FactoryStatusProjection {
    pub before: BaseFactoryRuntimeState,
    pub after: BaseFactoryRuntimeState,
}

/// Project one `FUN_00419630` publication without mutating a live entity.
///
/// The six assignments intentionally mirror retail's publication order:
/// current staff, capacity, zero-extended output-selector byte, production
/// ratio, delivery ratio, and understaffed ratio. The Sub-M descriptor and
/// independently owned production state and progressive-death clock survive
/// unchanged.
pub fn project_factory_status(
    before: BaseFactoryRuntimeState,
    publication: FactoryStatusPublication,
) -> FactoryStatusProjection {
    let after = BaseFactoryRuntimeState {
        status_descriptor: before.status_descriptor,
        control_value_raw: u16::from(publication.output_selector_raw as u8),
        required_scientists: publication.scientist_capacity_raw,
        current_scientists: publication.current_scientists_raw,
        lifter_progress_raw: publication.delivery_or_cooldown_ratio_raw,
        production_progress_raw: publication.production_or_cooldown_ratio_raw,
        recovery_progress_raw: publication.understaffed_ratio_raw,
        production: before.production,
        live_owner: before.live_owner,
        progressive_death: before.progressive_death,
    };

    FactoryStatusProjection { before, after }
}
