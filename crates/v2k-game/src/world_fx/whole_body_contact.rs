//! `141D0 -> 440DC0 -> 40A60` with the actor's actual birth provenance.

use super::*;

pub(crate) struct WholeBodyContactScatter<'a> {
    pub position_raw: [i16; 3],
    pub particle_class: u8,
    pub scale_raw: u32,
    pub owner_id: u32,
    pub owner_entity_type: u8,
    pub owner_state_sign: bool,
    pub environment: ParticleEnvironment<'a>,
    pub retail_tick: u32,
}

impl WorldFx {
    /// The direction cursor advances before every allocation, even a rejected
    /// one. This scatter consumes no random words. Unlike a detached effect,
    /// its 40A60 birth also owns the current terrain/water cache initialization.
    pub(crate) fn emit_whole_body_contact_scatter_raw(
        &mut self,
        request: WholeBodyContactScatter<'_>,
    ) {
        let attempts = self
            .frame_pacing
            .scaled_count_min_one((request.scale_raw >> 10) as i32)
            .max(0) as usize;
        for _ in 0..attempts {
            self.direction_cursor = (self.direction_cursor + 1) % RETAIL_DIRECTION_TABLE_RAW.len();
            let direction = RETAIL_DIRECTION_TABLE_RAW[self.direction_cursor];
            let velocity = [
                i32::from(direction[0]) >> 1,
                (i32::from(direction[1]) >> 2).abs(),
                i32::from(direction[2]) >> 1,
            ];
            let mut burst = single_descriptor_particle_burst(
                raw_position_to_world(request.position_raw),
                request.particle_class,
                velocity,
                Some(request.owner_id),
            );
            burst.particles[0].source_entity_type_at_birth = Some(request.owner_entity_type);
            burst.particles[0].suppresses_impact_damage = request.owner_state_sign;
            self.materialize_particle_burst_with_birth_context(
                burst,
                Some(ParticleBirthContext {
                    environment: request.environment,
                    retail_tick: request.retail_tick,
                }),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whole_body_scatter_initializes_owned_particle_water_cache_without_rng() {
        let mut fx = WorldFx::new();
        for _ in 0..PARTICLE_FRAME_SAMPLE_COUNT {
            fx.advance_frame_pacing(20_000);
        }
        let rng = fx.rng_state;
        let direction = fx.direction_cursor;
        fx.emit_whole_body_contact_scatter_raw(WholeBodyContactScatter {
            position_raw: [623, 0, 1920],
            particle_class: 13,
            scale_raw: 0x1000,
            owner_id: 47,
            owner_entity_type: 87,
            owner_state_sign: false,
            environment: ParticleEnvironment::FlatWater { sea_level: 0.0 },
            retail_tick: 1774,
        });
        assert_eq!(fx.rng_state, rng);
        assert_eq!(
            fx.direction_cursor,
            (direction + 4) % RETAIL_DIRECTION_TABLE_RAW.len()
        );
        assert_eq!(fx.particle_count(), 4);
        for entry in fx.particles.presentation() {
            let particle = entry.particle;
            assert!(particle.water_state_initialized);
            assert_eq!(particle.water_state, 1);
            assert_eq!(particle.step_start_water_state, 1);
            assert_eq!(particle.owner_id, Some(47));
            assert_eq!(particle.source_entity_type_at_birth, Some(87));
        }
    }
}
