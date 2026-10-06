//! FUN_00442420 projectile surface replacement and its same-visit E3D0 tail.
//!
//! Decision matrix: source52/53 -> 74, source68/69 -> 76, source87 -> 88;
//! selector6 -> 73, selector0..5/8..12 -> the source replacement, all other
//! selectors -> delete. A water replacement continues the originating mode3
//! visit's ground tail. Replacement descriptors are mode0, so future visits
//! skip contacts (FUN_00440120's jump table at 4407B4).

use super::*;
use crate::projectile_emitter::{
    projectile_replacement_ground_response, projectile_surface_response,
    ProjectileReplacementGroundResponse, ProjectileSurfaceResponse,
    PROJECTILE_REPLACEMENT_GROUND_CALLBACK_VA, PROJECTILE_SURFACE_CALLBACK_VA,
};

pub(super) fn uses_projectile_surface_callback(source_class: u8) -> bool {
    particle_descriptor(source_class).is_some_and(|descriptor| {
        descriptor.raw_byte(0x0e) == 3 && descriptor.raw_u32(0x18) == PROJECTILE_SURFACE_CALLBACK_VA
    })
}

impl WorldFx {
    /// Descriptor-owned mode-3 surface tail beginning with `FUN_00442420`.
    ///
    /// A water conversion relinks the same physical slot as class 73/74/76/88 and
    /// then continues directly into this traversal's ground tail using the
    /// replacement descriptor and `FUN_0043E3D0`. Update/entity/static stages
    /// are not rerun.
    pub(super) fn dispatch_projectile_surface_collision(
        &mut self,
        slot: usize,
        particle: &mut WorldParticle,
        context: TerrainCollisionContext<'_>,
        birth_context: ParticleBirthContext<'_>,
        emitter_context: ParticleEmitterContext,
    ) -> bool {
        if particle.step_start_water_state == 2 && particle.water_state < 2 {
            if let Some(surface_y) =
                displaced_water_surface(context, particle.position, birth_context.retail_tick)
            {
                let material_code = nearest_material_code(context.terrain, particle.position);
                let selector = context.water_response_selectors[usize::from(material_code)];
                if !self.apply_projectile_surface_response(slot, particle, selector, surface_y) {
                    return false;
                }
            }
        }

        let collision_surface_y = context
            .terrain
            .height_at(particle.position[0], particle.position[2]);
        let radius = f32::from(particle.collision_radius_raw) / 256.0;
        if particle.position[1] - radius <= collision_surface_y {
            let material_code = nearest_material_code(context.terrain, particle.position);
            let selector = context.ground_response_selectors[usize::from(material_code)];
            let response_surface_y =
                coarse_particle_ground_response_y(context.terrain, particle.position);
            return if uses_projectile_surface_callback(particle.source_class) {
                self.apply_projectile_surface_response(slot, particle, selector, response_surface_y)
            } else {
                debug_assert!(matches!(particle.source_class, 73 | 74 | 76 | 88));
                debug_assert_eq!(
                    particle_descriptor(particle.source_class)
                        .expect("projectile replacement descriptor")
                        .raw_u32(0x18),
                    PROJECTILE_REPLACEMENT_GROUND_CALLBACK_VA
                );
                self.apply_projectile_replacement_ground_response(
                    slot,
                    particle,
                    selector,
                    response_surface_y,
                    birth_context,
                    emitter_context,
                )
            };
        }
        true
    }

    pub(super) fn apply_projectile_surface_response(
        &mut self,
        slot: usize,
        particle: &mut WorldParticle,
        selector: u8,
        surface_y: f32,
    ) -> bool {
        match projectile_surface_response(particle.source_class, selector) {
            ProjectileSurfaceResponse::Delete => false,
            ProjectileSurfaceResponse::ConvertToParticleClass(source_class) => {
                particle.position[1] = surface_y;
                self.particles
                    .transition_class_in_place(slot, particle, source_class)
            }
        }
    }

    fn apply_projectile_replacement_ground_response(
        &mut self,
        slot: usize,
        particle: &mut WorldParticle,
        selector: u8,
        surface_y: f32,
        birth_context: ParticleBirthContext<'_>,
        emitter_context: ParticleEmitterContext,
    ) -> bool {
        match projectile_replacement_ground_response(selector) {
            ProjectileReplacementGroundResponse::ConvertToParticleClass73 => {
                particle.position[1] = surface_y;
                self.particles.transition_class_in_place(slot, particle, 73)
            }
            ProjectileReplacementGroundResponse::EmitSurfaceChildAndDelete => {
                let source_class = match birth_context.environment {
                    ParticleEnvironment::Dry => UPGRADED_PRIMARY_ABOVE_SURFACE_CLASS,
                    ParticleEnvironment::FlatWater { sea_level } => {
                        if surface_y <= sea_level {
                            UPGRADED_PRIMARY_SUBMERGED_SURFACE_CLASS
                        } else {
                            UPGRADED_PRIMARY_ABOVE_SURFACE_CLASS
                        }
                    }
                    ParticleEnvironment::Terrain(context) => {
                        if surface_y <= context.terrain.sea_level_world_y() {
                            UPGRADED_PRIMARY_SUBMERGED_SURFACE_CLASS
                        } else {
                            UPGRADED_PRIMARY_ABOVE_SURFACE_CLASS
                        }
                    }
                };
                let mut burst = single_descriptor_particle_burst(
                    [particle.position[0], surface_y, particle.position[2]],
                    source_class,
                    UPGRADED_PRIMARY_SURFACE_VELOCITY_RAW,
                    emitter_context.current_owner.map(|owner| owner.entity_id),
                );
                if let Some(child) = burst.particles.first_mut() {
                    child.source_entity_type_at_birth = Some(
                        emitter_context
                            .current_owner
                            .map_or(0, |owner| owner.entity_type),
                    );
                    // F3E3D0 copies only parent +0x1D bit zero into the F41770
                    // request. Owner provenance comes from DAT_004DCA00 above,
                    // not particle +0x18/+0x1C.
                    child.suppresses_impact_damage = particle.suppresses_impact_damage;
                }
                self.materialize_particle_burst_with_birth_context(burst, Some(birth_context));
                false
            }
            ProjectileReplacementGroundResponse::Delete => false,
        }
    }
}
