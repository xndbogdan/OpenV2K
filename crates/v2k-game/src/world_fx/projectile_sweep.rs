//! Shared F590/F800 model-hit phase; per-descriptor surface programs run later.
//! The packet is retained with its proven callback pair, independently of motion.
//! The `FUN_00442950` static-route family (classes 52/68/85) shares this sweep:
//! its entity hits carry the descriptor-selected `+0x20` packet into the
//! `FUN_00441180` delivery, while static contact consumes 52/68 through the
//! `FUN_0042E8E0` no-op and delivers `FUN_0043F800` for 85.

use super::*;
use crate::damage::{
    CLASS49_TURRET_BOLT_DAMAGE_PACKET, CLASS56_TURRET_BOLT_DAMAGE_PACKET,
    CLASS68_STATIC_ROUTE_DAMAGE_PACKET, DRAGON_FIREBALL_DAMAGE_PACKET, TURRET_BOLT_DAMAGE_PACKET,
    TYPE_47_PROJECTILE_DAMAGE_PACKET,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ProjectileModelSweepProgram {
    BallisticScatter,
    DragonFireball,
    TurretBolt {
        packet: DamagePacket,
    },
    /// `FUN_00442950` entity hit with the descriptor-selected packet.
    /// Static contact consumes classes 52/68 via the `FUN_0042E8E0` no-op
    /// and delivers `FUN_0043F800` for class 85.
    StaticRoute {
        packet: DamagePacket,
        static_noop: bool,
    },
}

impl ProjectileModelSweepProgram {
    pub(super) fn for_class(class: u8) -> Option<Self> {
        if uses_ballistic_sweep_callbacks(class) {
            return Some(Self::BallisticScatter);
        }
        if particle_uses_static_route_entity_hit(class) {
            // The gate authenticates (class, `+0x20`); the `+0x24` word selects
            // the static-contact policy: `FUN_0042E8E0` no-op for 52/68,
            // `FUN_0043F800` for 85.
            let descriptor = particle_descriptor(class)?;
            let (packet, static_noop) = match (class, descriptor.raw_u32(0x24)) {
                (52, STATIC_ROUTE_NOOP_STATIC_CALLBACK_VA) => {
                    (TYPE_47_PROJECTILE_DAMAGE_PACKET, true)
                }
                (68, STATIC_ROUTE_NOOP_STATIC_CALLBACK_VA) => {
                    (CLASS68_STATIC_ROUTE_DAMAGE_PACKET, true)
                }
                (85, BALLISTIC_STATIC_CALLBACK_VA) => (DRAGON_FIREBALL_DAMAGE_PACKET, false),
                _ => return None,
            };
            return Some(Self::StaticRoute {
                packet,
                static_noop,
            });
        }
        let descriptor = particle_descriptor(class)?;
        if descriptor.raw_byte(0x0e) != 3
            || descriptor.raw_u32(0x1c) != FUN_0043F590_HIT_VA
            || descriptor.raw_u32(0x24) != BALLISTIC_STATIC_CALLBACK_VA
        {
            return None;
        }
        match (class, descriptor.raw_u32(0x18), descriptor.raw_u32(0x20)) {
            (38, 0x0044_1B70, 0x004C_C048) => Some(Self::DragonFireball),
            (49 | 80, 0x0044_1C90, 0x004C_C078) => Some(Self::TurretBolt {
                packet: CLASS49_TURRET_BOLT_DAMAGE_PACKET,
            }),
            (55 | 81, 0x0044_1C90, 0x004C_C0A8) => Some(Self::TurretBolt {
                packet: TURRET_BOLT_DAMAGE_PACKET,
            }),
            (56 | 82, 0x0044_1C90, 0x004C_C090) => Some(Self::TurretBolt {
                packet: CLASS56_TURRET_BOLT_DAMAGE_PACKET,
            }),
            _ => None,
        }
    }

    const fn packet(self) -> DamagePacket {
        match self {
            Self::BallisticScatter => BALLISTIC_PARTICLE_DAMAGE_PACKET,
            Self::DragonFireball => DRAGON_FIREBALL_DAMAGE_PACKET,
            Self::TurretBolt { packet } => packet,
            Self::StaticRoute { packet, .. } => packet,
        }
    }
}

pub(super) struct ProjectileSweepFrame<'a> {
    pub collision: ParticleCollisionContext<'a>,
    pub terrain_context: Option<TerrainCollisionContext<'a>>,
    pub retail_tick: u32,
    pub terrain_type_mutations: &'a [ParticleTerrainMutation],
}

/// F980 uses the integrated endpoint for the actor callback. FF10 publishes
/// its refined static probe. The caller runs F610, then the synchronous host
/// damage callback, and only then frees the current physical record.
pub(super) fn detect_damage_projectile_sweep(
    program: ProjectileModelSweepProgram,
    physical_slot: usize,
    particle: &mut WorldParticle,
    frame: ProjectileSweepFrame<'_>,
) -> Option<BallisticSweepImpact> {
    debug_assert_eq!(
        ProjectileModelSweepProgram::for_class(particle.source_class),
        Some(program)
    );
    let ProjectileSweepFrame {
        collision,
        terrain_context,
        retail_tick,
        terrain_type_mutations,
    } = frame;
    // 442950 calls 11180 unconditionally. Its suppression bit gates only
    // the later attached allocation; F590/F800 retain their separate gate.
    let delivery = BallisticDamageRequest {
        packet: program.packet(),
        source_entity_type_at_birth: particle.source_entity_type_at_birth,
        source_owner_id: particle.owner_id,
    };
    let damage = (!particle.suppresses_impact_damage).then_some(delivery);

    if let Some(target_entity_id) = retail_swept_model_hit(
        particle.step_start_position,
        particle.position,
        particle.collision_radius_raw,
        particle.owner_id,
        particle.age_ticks,
        collision.entities,
        collision.model_pool,
    ) {
        // FUN_0043F590 deliberately presents and delivers at the integrated
        // endpoint, even though the exact model test used refined probes.
        return Some(BallisticSweepImpact::Entity(ParticleEntityImpact {
            source_particle_class: particle.source_class,
            impact_position_argument_va: retail_particle_impact_position_argument_va(physical_slot),
            target_entity_id,
            position_world: particle.position,
            velocity_raw: particle
                .velocity
                .map(|component| world_velocity_component_to_raw(component) as i16),
            damage: if matches!(program, ProjectileModelSweepProgram::StaticRoute { .. }) {
                Some(delivery)
            } else {
                damage
            },
        }));
    }

    let context = terrain_context?;
    let terrain_objects = context.terrain_objects?;
    let hit = retail_swept_static_tile_hit(
        particle.step_start_position,
        particle.position,
        particle.collision_radius_raw,
        context.terrain,
        terrain_objects,
        retail_tick,
        collision.model_pool,
        terrain_type_mutations,
    )?;
    if matches!(
        program,
        ProjectileModelSweepProgram::StaticRoute {
            static_noop: true,
            ..
        }
    ) {
        // `FUN_0042E8E0` (static-route classes 52/68): consume the parent
        // with no visual, damage, or static program.
        return Some(BallisticSweepImpact::ConsumedStaticNoOp {
            position_world: particle.position,
        });
    }
    // FUN_0043FF10 publishes its refined probe before FUN_0043F800.
    particle.position = hit.position_world;
    Some(BallisticSweepImpact::StaticTile(ParticleStaticImpact {
        source_particle_class: particle.source_class,
        source_owner_id: particle.owner_id,
        position_world: hit.position_world,
        cell: hit.cell,
        attribute: hit.attribute,
        terrain_type: hit.terrain_type,
        model_id: hit.model_id,
        kind_index: hit.kind_index,
        current: None,
        damage,
    }))
}
