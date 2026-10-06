//! Synchronous class-19 task emissions and class-1 40950 explosion prefix.

use super::*;

impl WorldFx {
    /// BAF0 detects Type10's allocated Sub-G and selects ten class37
    /// attempts. Capability8 has no relation-owner bit40, so16F90 retains
    /// the dragon's own type/handle; the independent40950 sea/audio tail
    /// follows even when the direction scatter stops at allocator failure.
    pub(crate) fn emit_intro2_type10_terminal_raw(
        &mut self,
        position_raw: [i16; 3],
        source_handle: u32,
        extent_raw: u16,
        sea_level_raw: Option<i16>,
    ) {
        self.emit_direction_table_scatter_raw(
            position_raw,
            DirectionTableScatterEmission {
                base_count: 10,
                extent_raw,
                source_classes: [37; 2],
                velocity_scale_raw: 8,
                owner_id: Some(source_handle),
                source_entity_type_at_birth: Some(10),
                suppresses_impact_damage: false,
            },
        );
        self.emit_scatter_surface_sound_tail_raw(
            position_raw,
            sea_level_raw,
            source_handle,
            Some(10),
        );
    }

    /// BAF0 detects Type57's allocated Sub-G and selects ten class37
    /// attempts with provenance Type57 and handle.
    pub(crate) fn emit_intro2_type57_terminal_raw(
        &mut self,
        position_raw: [i16; 3],
        source_handle: u32,
        extent_raw: u16,
        sea_level_raw: Option<i16>,
    ) {
        self.emit_direction_table_scatter_raw(
            position_raw,
            DirectionTableScatterEmission {
                base_count: 10,
                extent_raw,
                source_classes: [37; 2],
                velocity_scale_raw: 8,
                owner_id: Some(source_handle),
                source_entity_type_at_birth: Some(57),
                suppresses_impact_damage: false,
            },
        );
        self.emit_scatter_surface_sound_tail_raw(
            position_raw,
            sea_level_raw,
            source_handle,
            Some(57),
        );
    }

    /// One 4061A0 allocation. The task owns the three subsequent jitter draws,
    /// including after the final attempt and after allocation failure.
    pub(crate) fn emit_intro2_meteor_trail_raw(
        &mut self,
        position_raw: [i16; 3],
        source_handle: u32,
    ) {
        self.materialize_owned_descriptor_particle_raw(
            position_raw,
            crate::intro2_meteors::METEOR_TRAIL_CLASS,
            [0; 3],
            source_handle,
            Some(crate::intro2_meteors::INTRO2_METEOR_TYPE as u8),
        );
    }

    /// BAF0's component-free Type34 profile enters 40950 with ten class16
    /// scatter attempts. The process direction cursor owns their directions;
    /// the independent surface tail owns its one randomized sound62 request.
    pub(crate) fn emit_intro2_meteor_impact_raw(
        &mut self,
        position_raw: [i16; 3],
        source_handle: u32,
        extent_raw: u16,
        sea_level_raw: Option<i16>,
    ) {
        self.emit_direction_table_scatter_raw(
            position_raw,
            DirectionTableScatterEmission {
                base_count: 10,
                extent_raw,
                source_classes: [16; 2],
                velocity_scale_raw: 8,
                owner_id: Some(source_handle),
                source_entity_type_at_birth: Some(34),
                suppresses_impact_damage: false,
            },
        );
        self.emit_scatter_surface_sound_tail_raw(
            position_raw,
            sea_level_raw,
            source_handle,
            Some(34),
        );
    }

    /// 4141D0's distinct ground table at 004C9368. In particular its final
    /// selectors differ from both particle-contact tables at 004CD718/C0.
    pub(crate) fn emit_intro2_meteor_ground_raw(
        &mut self,
        position_raw: [i16; 3],
        source_handle: u32,
        response_selector: u8,
        scale_raw: u32,
    ) {
        const GROUND_CLASSES: [u8; 13] = [7, 8, 9, 10, 7, 11, 13, 12, 59, 10, 10, 10, 7];
        let particle_class = GROUND_CLASSES[usize::from(response_selector)];
        self.emit_fun_00440dc0_raw(
            position_raw,
            particle_class,
            scale_raw,
            source_handle,
            false,
        );
    }
}
