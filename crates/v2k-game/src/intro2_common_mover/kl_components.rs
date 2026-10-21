//! Native four-word ABCDEHKL component image from FUN_00409A80.
//!
//! The bank is zero-filled independently of K's 24450 allocation (16 bytes)
//! and L's 1BB80 allocation (20 bytes). 09A80 then binds both callbacks through
//! 0A950: selector n points to bank word n-1. The selectors are the type's own
//! payload bytes (409EF2 reads K's from type +0x30), so each admitted model
//! keeps its binding. This is a component shape, not an actor/type admission
//! receipt; the actor owner must authenticate its lease.

use crate::common_mover::type9_attitude::Type9BodyBasis;
use crate::entity_collision_state::{
    CommonMoverGklPayloads, EntityTypeRuntimeMetadata, RetailRuntimeValue,
};
use crate::gkl_common_mover::component_outputs;
use v2k_formats::models::AnimVars;

/// Type30's model: K writes selectors 3/4 and L selectors 2/1.
const TYPE30_PAYLOADS: CommonMoverGklPayloads = CommonMoverGklPayloads {
    sub_g: None,
    sub_k: Some([3, 4]),
    sub_l: Some([2, 1, 64, 31, 160, 15]),
};
/// Model 1131 (Type38/Type129): the same limits with each pair swapped.
const MODEL1131_PAYLOADS: CommonMoverGklPayloads = CommonMoverGklPayloads {
    sub_g: None,
    sub_k: Some([4, 3]),
    sub_l: Some([1, 2, 64, 31, 160, 15]),
};
/// Model 1171 (Type33): model1131's K and L selectors, with L's second
/// limit pair (64, 31) like its first.
const MODEL1171_PAYLOADS: CommonMoverGklPayloads = CommonMoverGklPayloads {
    sub_g: None,
    sub_k: Some([4, 3]),
    sub_l: Some([1, 2, 64, 31, 64, 31]),
};
/// Model 684 (Type12/Type127): an 82-word bank. K writes its last two words,
/// past the 63 a model operand can address, and L words 2/1.
const MODEL684_PAYLOADS: CommonMoverGklPayloads = CommonMoverGklPayloads {
    sub_g: None,
    sub_k: Some([81, 82]),
    sub_l: Some([2, 1, 244, 1, 120, 5]),
};
/// Model operands address callback values through six bits.
const MODEL_ADDRESSABLE_VARIABLES: usize = 64;

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
    sub_k_selectors: [u8; 2],
    sub_l_descriptor: [u8; 6],
    /// The type's `+0x110` words, zeroed by 09A80.
    model_variables_raw: Vec<i16>,
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
        let (sub_k_selectors, sub_l_descriptor, count) = Self::authenticate_profile(metadata)?;
        Ok(Self {
            sub_k_selectors,
            sub_l_descriptor,
            model_variables_raw: vec![0; count],
            sub_k_smoothed_raw: 0,
            sub_l_target_raw: [0; 3],
            sub_l_exact_raw: 0,
        })
    }

    fn authenticate_profile(
        metadata: &EntityTypeRuntimeMetadata,
    ) -> Result<([u8; 2], [u8; 6], usize), Intro2KlConstructionError> {
        if metadata.common_mover_topology != RetailRuntimeValue::Known(super::ABCDEHKL_TOPOLOGY) {
            return Err(Intro2KlConstructionError::ComponentTopology);
        }
        let RetailRuntimeValue::Known(count) = metadata.model_variable_count_raw else {
            return Err(Intro2KlConstructionError::VariableCount);
        };
        let expected = match metadata.common_mover_gkl_payloads {
            RetailRuntimeValue::Known(payloads)
                if payloads == TYPE30_PAYLOADS
                    || payloads == MODEL1131_PAYLOADS
                    || payloads == MODEL1171_PAYLOADS =>
            {
                4
            }
            RetailRuntimeValue::Known(payloads) if payloads == MODEL684_PAYLOADS => 82,
            _ => return Err(Intro2KlConstructionError::ComponentDescriptors),
        };
        if count != expected {
            return Err(Intro2KlConstructionError::VariableCount);
        }
        match metadata.common_mover_gkl_payloads {
            RetailRuntimeValue::Known(CommonMoverGklPayloads {
                sub_g: None,
                sub_k: Some(sub_k),
                sub_l: Some(sub_l),
            }) => Ok((sub_k, sub_l, count as usize)),
            _ => Err(Intro2KlConstructionError::ComponentDescriptors),
        }
    }

    pub(crate) fn authenticates(&self, metadata: &EntityTypeRuntimeMetadata) -> bool {
        Self::authenticate_profile(metadata)
            == Ok((
                self.sub_k_selectors,
                self.sub_l_descriptor,
                self.model_variables_raw.len(),
            ))
    }

    #[cfg(test)]
    pub(crate) fn model_variables_raw(&self) -> &[i16] {
        &self.model_variables_raw
    }

    /// D320/D350 use one-based bank selectors; selector0 remains the world
    /// clock and is deliberately outside this allocation. A model reads only
    /// the selectors its six-bit operands reach.
    pub(crate) fn publish_model_variables(&self, vars: &mut AnimVars) {
        for (selector, &value) in (1..MODEL_ADDRESSABLE_VARIABLES).zip(&self.model_variables_raw) {
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
            self.load_outputs(self.sub_k_selectors),
            self.sub_k_smoothed_raw,
            velocity_y_raw,
        );
        self.store_outputs(self.sub_k_selectors, outputs);
    }

    pub(super) fn advance_sub_l(&mut self, position_raw: [i16; 3], basis: Type9BodyBasis) {
        let outputs = component_outputs::advance_sub_l(
            self.load_outputs([self.sub_l_descriptor[0], self.sub_l_descriptor[1]]),
            self.sub_l_exact_raw,
            position_raw,
            self.sub_l_target_raw,
            basis,
            self.sub_l_descriptor,
        );
        self.store_outputs(
            [self.sub_l_descriptor[0], self.sub_l_descriptor[1]],
            outputs,
        );
    }
}
