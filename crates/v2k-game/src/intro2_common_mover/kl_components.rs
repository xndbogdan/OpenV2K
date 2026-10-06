//! Native four-word ABCDEHKL component image from FUN_00409A80.
//!
//! The bank is zero-filled independently of K's 24450 allocation (16 bytes)
//! and L's 1BB80 allocation (20 bytes). 09A80 then binds both callbacks through
//! 0A950: selector n points to bank word n-1. This is a component shape, not an
//! actor/type admission receipt; the actor owner must authenticate its lease.

use crate::common_mover::type9_attitude::Type9BodyBasis;
use crate::entity_collision_state::{
    CommonMoverGklPayloads, EntityTypeRuntimeMetadata, RetailRuntimeValue,
};
use crate::gkl_common_mover::component_outputs;
use v2k_formats::models::AnimVars;

const SUB_K: [u8; 2] = [3, 4];
const SUB_L: [u8; 6] = [2, 1, 64, 31, 160, 15];
const PAYLOADS: CommonMoverGklPayloads = CommonMoverGklPayloads {
    sub_g: None,
    sub_k: Some(SUB_K),
    sub_l: Some(SUB_L),
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2KlConstructionError {
    ComponentTopology,
    VariableCount,
    ComponentDescriptors,
}

/// All K/L mutable words, including their real shared four-word bank.
///
/// Output fields are not mirrored in separate arrays: callback pointer loads
/// and stores read/write the bank itself, preserving actual binding identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Intro2KlComponents {
    model_variables_raw: [i16; 4],
    sub_k_smoothed_raw: i32,
    sub_l_target_raw: [i16; 3],
    sub_l_exact_raw: i32,
}

impl Intro2KlComponents {
    /// Successful native 09A80 K/L component construction, before behavior
    /// initialization. None of these allocation/zero-fill/binding steps draw
    /// RNG. Native allocation failure and disposal remain with the actor host.
    pub(crate) fn from_native_constructor(
        metadata: &EntityTypeRuntimeMetadata,
    ) -> Result<Self, Intro2KlConstructionError> {
        Self::authenticate_profile(metadata)?;
        Ok(Self {
            model_variables_raw: [0; 4],
            sub_k_smoothed_raw: 0,
            sub_l_target_raw: [0; 3],
            sub_l_exact_raw: 0,
        })
    }

    fn authenticate_profile(
        metadata: &EntityTypeRuntimeMetadata,
    ) -> Result<(), Intro2KlConstructionError> {
        if metadata.common_mover_topology != RetailRuntimeValue::Known(super::ABCDEHKL_TOPOLOGY) {
            return Err(Intro2KlConstructionError::ComponentTopology);
        }
        if metadata.model_variable_count_raw != RetailRuntimeValue::Known(4) {
            return Err(Intro2KlConstructionError::VariableCount);
        }
        if metadata.common_mover_gkl_payloads != RetailRuntimeValue::Known(PAYLOADS) {
            return Err(Intro2KlConstructionError::ComponentDescriptors);
        }
        Ok(())
    }

    pub(crate) fn authenticates(&self, metadata: &EntityTypeRuntimeMetadata) -> bool {
        Self::authenticate_profile(metadata).is_ok()
    }

    #[cfg(test)]
    pub(crate) const fn model_variables_raw(&self) -> &[i16; 4] {
        &self.model_variables_raw
    }

    /// D320/D350 use one-based bank selectors; selector0 remains the world
    /// clock and is deliberately outside this allocation.
    pub(crate) fn publish_model_variables(&self, vars: &mut AnimVars) {
        for (selector, value) in (1..=4).zip(self.model_variables_raw) {
            vars.dynamic[selector] = i32::from(value);
        }
    }

    pub(crate) const fn sub_k_smoothed_raw(&self) -> i32 {
        self.sub_k_smoothed_raw
    }

    #[cfg(test)]
    pub(crate) const fn sub_l_target_raw(&self) -> [i16; 3] {
        self.sub_l_target_raw
    }

    #[cfg(test)]
    pub(crate) const fn sub_l_exact_raw(&self) -> i32 {
        self.sub_l_exact_raw
    }

    pub(super) fn commit_target(&mut self, target_raw: [i16; 3]) {
        self.sub_l_target_raw = target_raw;
    }

    pub(super) fn commit_sub_d_writes(&mut self, smoothed_raw: i32, exact_raw: i32) {
        self.sub_k_smoothed_raw = smoothed_raw;
        self.sub_l_exact_raw = exact_raw;
    }

    fn load_outputs(&self, selectors: [u8; 2]) -> [i16; 2] {
        selectors.map(|selector| self.model_variables_raw[usize::from(selector) - 1])
    }

    fn store_outputs(&mut self, selectors: [u8; 2], outputs: [i16; 2]) {
        for (selector, value) in selectors.into_iter().zip(outputs) {
            self.model_variables_raw[usize::from(selector) - 1] = value;
        }
    }

    pub(super) fn advance_sub_k(&mut self, velocity_y_raw: i16) {
        let outputs = component_outputs::advance_sub_k(
            self.load_outputs(SUB_K),
            self.sub_k_smoothed_raw,
            velocity_y_raw,
        );
        self.store_outputs(SUB_K, outputs);
    }

    pub(super) fn advance_sub_l(&mut self, position_raw: [i16; 3], basis: Type9BodyBasis) {
        let outputs = component_outputs::advance_sub_l(
            self.load_outputs([SUB_L[0], SUB_L[1]]),
            self.sub_l_exact_raw,
            position_raw,
            self.sub_l_target_raw,
            basis,
            SUB_L,
        );
        self.store_outputs([SUB_L[0], SUB_L[1]], outputs);
    }
}
