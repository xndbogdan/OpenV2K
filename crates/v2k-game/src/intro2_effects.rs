//! Intro2's live physical-particle traversal and synchronous impact delivery.
//!
//! The cinematic uses the same `FUN_00440120` callbacks as ordinary worlds.
//! Actor submission and the camera may not replace the live collision model,
//! and damage must run between the particle's visual callback and parent free.
//! The report retains unsupported actor policies instead of dropping a hit.

use crate::damage::EntityHitEntry;
use crate::entity::{
    CheckedProjectileDamageOutcome, CheckedProjectileDamageRequest, Entity, EntityKind,
    EntityManager, Fun00411250Applied, Fun00411250Type9DamageOutcome,
};
use crate::entity_collision_state::{particle_model_collision_eligible, RetailRuntimeValue};
use crate::gameplay_notifications::GameplayNotifications;
use crate::resource_cache::ResourceCache;
use crate::specialized_actor_task_production::SpecializedActorTaskScheduler;
use crate::static_damage::{StaticDamageOutcome, StaticDamageScheduler};
use crate::type47_checked_damage::{
    apply_type47_checked_damage_after_c690, Type47CheckedDamageOutcome, Type47CheckedDamageRequest,
};
use crate::type47_impact_live::{
    apply_type47_impact_c690_live, Type47ImpactLiveError, Type47ImpactLiveOutcome,
    Type47ImpactLiveRequest,
};
use crate::type47_impact_reaction::{
    apply_type47_impact_reaction_after_c690, Type47ImpactReactionOutcome,
};
use crate::world_fx::{
    particle_uses_fun_0043f780_entity_hit, particle_uses_static_route_entity_hit,
    CombatGroundProgramRequest, EntityCollisionModel, ParticleCallbackContext,
    ParticleCollisionContext, ParticleEntityImpact, ParticleEnvironment, ParticleOwnerMotion,
    ParticleStaticImpact, ParticleTerrainMutation, ParticleTerrainPublication,
    ParticleTraversalContext, ParticleTraversalHost, ParticleTraversalTiming,
    ParticleUpdateOutcome, TerrainCollisionContext, WorldFx,
};

#[cfg(test)]
mod attached_tests;

pub struct Intro2EffectsFrame<'a> {
    pub cache: &'a mut ResourceCache,
    pub entities: &'a mut EntityManager,
    pub world_fx: &'a mut WorldFx,
    pub static_damage: &'a mut StaticDamageScheduler,
    pub scheduler: &'a mut SpecializedActorTaskScheduler,
    pub notifications: &'a mut GameplayNotifications,
    pub elapsed_micros: u32,
    pub retail_tick: u32,
}

#[derive(Debug)]
pub enum Intro2EntityImpactStep {
    Suppressed,
    /// No retained owner proves the exact 11320 type/style callback chain.
    UnsupportedCuredTarget,
    BirthProvenanceUnavailable,
    InfectedModel(Fun00411250Applied),
    Type47Reselection(Result<Type47ImpactLiveOutcome, Type47ImpactLiveError>),
    Type47Impulse(Type47ImpactReactionOutcome),
    Type47Damage(Type47CheckedDamageOutcome),
    NativeType47Damage(crate::shared_actor_impact::type47::NativeType47ImpactOutcome),
    Type9Damage(Fun00411250Type9DamageOutcome),
    NativeType9Damage(crate::ordinary_type9_impact::NativeType9ImpactOutcome),
    Type26Damage(crate::intro2_type26_defecate_virus::Intro2Type26ImpactOutcome),
    Type53Damage(crate::intro2_type53::impact::Intro2Type53ImpactOutcome),
    Type122Damage(crate::native_type122::impact::Type122ImpactOutcome),
    Type16Damage(crate::intro2_type16::impact::Intro2Type16ImpactOutcome),
    Type8Damage(crate::intro2_type8::impact::Intro2Type8ImpactOutcome),
    GunTurretDamage(crate::intro2_gun_turret::impact::Intro2GunTurretImpactOutcome),
    FlyerDamage(crate::intro2_flyer_impact::NativeFlyerImpactOutcome),
    Type10Damage(crate::intro2_type10::impact::Intro2Type10ImpactOutcome),
    Type57Damage(crate::intro2_type57::impact::Intro2Type57ImpactOutcome),
    Type13Damage(crate::intro2_type13_live::impact::Intro2Type13ImpactOutcome),
    Type13StaticDamage(crate::intro2_type13_live::impact::Intro2Type13StaticOutcome),
    Type58Damage(crate::intro2_type58::impact::Intro2Type58ImpactOutcome),
    Type94Damage(crate::intro2_type94::impact::Intro2Type94ImpactOutcome),
    Type66Damage(crate::intro2_type66::impact::Intro2Type66ImpactOutcome),
    Type17Damage(crate::intro2_type17::impact::Intro2Type17ImpactOutcome),
    MeteorInfectedDamage(crate::intro2_meteors::impact::Intro2MeteorInfectedHitOutcome),
    FixedActorDamage(CheckedProjectileDamageOutcome),
    DeathRegistrationRejected(String),
}

#[derive(Debug)]
pub struct Intro2EntityImpactDelivery {
    pub impact: ParticleEntityImpact,
    pub steps: Vec<Intro2EntityImpactStep>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intro2StaticImpactDelivery {
    Suppressed,
    CurrentTargetUnavailable,
    Delivered(StaticDamageOutcome),
    BurnCallbackBlocked(crate::static_terrain_burn::StaticTerrainBurnFailure),
}

#[derive(Debug, Default)]
pub struct Intro2EffectsReport {
    pub particles: ParticleUpdateOutcome,
    /// Physical-slot order; these callbacks have already run.
    pub entity_deliveries: Vec<Intro2EntityImpactDelivery>,
    /// The owning world pass advances queued static programs separately.
    pub static_deliveries: Vec<(ParticleStaticImpact, Intro2StaticImpactDelivery)>,
    /// Excluded candidates whose consumed F980 bits are unresolved. Preserve
    /// first-observed live-list order across the initial and inline rebuilds.
    pub unresolved_collision_entity_ids: Vec<u32>,
}

/// Run in the simulation phase after actor/static updates and the eye callback,
/// before active-pair contact. Actor-owned shot queues drain in presentation.
/// Terrain writes commit between particle slots, before later callbacks can
/// change those cells. The returned terrain journal is history, not pending work.
pub fn update_intro2_effects(frame: Intro2EffectsFrame<'_>) -> Intro2EffectsReport {
    let Intro2EffectsFrame {
        cache,
        entities,
        world_fx,
        static_damage,
        scheduler,
        notifications,
        elapsed_micros,
        retail_tick,
    } = frame;
    let owner_motions = entities
        .iter_all()
        .map(|entity| ParticleOwnerMotion {
            owner_id: entity.id,
            velocity: entity.velocity,
        })
        .collect();
    let projection = intro2_particle_collision_models(cache, entities, retail_tick);
    let mut host = Intro2ParticleHost {
        cache,
        entities,
        static_damage,
        scheduler,
        notifications,
        retail_tick,
        owner_motions,
        collision_models: projection.models,
        unresolved_collision_entity_ids: projection.unresolved_entity_ids,
        entity_deliveries: Vec::new(),
        static_deliveries: Vec::new(),
    };
    let particles = world_fx.update_with_traversal_host(
        ParticleTraversalTiming {
            elapsed_micros,
            retail_tick,
        },
        &mut host,
    );
    Intro2EffectsReport {
        particles,
        entity_deliveries: host.entity_deliveries,
        static_deliveries: host.static_deliveries,
        unresolved_collision_entity_ids: host.unresolved_collision_entity_ids,
    }
}

/// Each physical slot borrows current geometry only until its hit boundary.
/// Reentrant death can mutate terrain, remove actors and allocate particles
/// before the next slot observes the world; no frame snapshot owns that state.
struct Intro2ParticleHost<'a> {
    cache: &'a mut ResourceCache,
    entities: &'a mut EntityManager,
    static_damage: &'a mut StaticDamageScheduler,
    scheduler: &'a mut SpecializedActorTaskScheduler,
    notifications: &'a mut GameplayNotifications,
    retail_tick: u32,
    owner_motions: Vec<ParticleOwnerMotion>,
    collision_models: Vec<EntityCollisionModel>,
    unresolved_collision_entity_ids: Vec<u32>,
    entity_deliveries: Vec<Intro2EntityImpactDelivery>,
    static_deliveries: Vec<(ParticleStaticImpact, Intro2StaticImpactDelivery)>,
}

impl ParticleTraversalHost for Intro2ParticleHost<'_> {
    fn publish_terrain_mutations(
        &mut self,
        mutations: &[ParticleTerrainMutation],
    ) -> ParticleTerrainPublication {
        self.cache
            .apply_particle_terrain_mutations(mutations)
            .map_or(ParticleTerrainPublication::Deferred, |_| {
                ParticleTerrainPublication::Committed
            })
    }

    fn context(&self) -> ParticleTraversalContext<'_> {
        ParticleTraversalContext {
            environment: TerrainCollisionContext::from_current_level_cache(self.cache)
                .map_or(ParticleEnvironment::Dry, ParticleEnvironment::Terrain),
            callbacks: ParticleCallbackContext {
                owner_motions: &self.owner_motions,
                collision: Some(ParticleCollisionContext {
                    entities: &self.collision_models,
                    model_pool: self.cache,
                }),
            },
            particle_emitter: Default::default(),
        }
    }
    fn entity_impact(&mut self, world_fx: &mut WorldFx, impact: ParticleEntityImpact) {
        let steps = deliver_entity_impact(
            self.cache,
            self.entities,
            world_fx,
            self.static_damage,
            self.scheduler,
            self.notifications,
            impact,
            self.retail_tick,
        );
        self.entity_deliveries
            .push(Intro2EntityImpactDelivery { impact, steps });
        if impact.damage.is_some() {
            let projection =
                intro2_particle_collision_models(self.cache, self.entities, self.retail_tick);
            self.collision_models = projection.models;
            for id in projection.unresolved_entity_ids {
                if !self.unresolved_collision_entity_ids.contains(&id) {
                    self.unresolved_collision_entity_ids.push(id);
                }
            }
            self.owner_motions = self
                .entities
                .iter_all()
                .map(|entity| ParticleOwnerMotion {
                    owner_id: entity.id,
                    velocity: entity.velocity,
                })
                .collect();
        }
    }

    fn attachment_owner(
        &self,
        id: u32,
    ) -> RetailRuntimeValue<Option<crate::world_fx::ParticleAttachmentOwner>> {
        RetailRuntimeValue::Known(
            self.entities
                .iter_all()
                .find(|entity| entity.id == id)
                .map(crate::world_fx::ParticleAttachmentOwner::from_entity),
        )
    }
    fn begin_attached_update(
        &mut self,
        _world_fx: &mut WorldFx,
        request: crate::world_fx::AttachedParticleOwnerRequest,
    ) -> crate::world_fx::AttachedParticleOwnerLookup {
        crate::world_fx::begin_attached_particle_owner_update(self.entities, request)
    }
    fn attached_damage(
        &mut self,
        world_fx: &mut WorldFx,
        request: crate::world_fx::AttachedParticleDamageRequest,
    ) -> Result<i32, crate::attached_particle_damage::AttachedParticleDamageBlock> {
        let result = crate::attached_particle_damage::apply_attached_particle_damage(
            crate::attached_particle_damage::AttachedParticleDamageFrame {
                resources: self.cache,
                entities: self.entities,
                world_fx,
                static_damage: self.static_damage,
                scheduler: self.scheduler,
                notifications: self.notifications,
                retail_tick: self.retail_tick,
                world: crate::attached_particle_damage::AttachedParticleDamageWorld::Cinematic,
            },
            request,
        );
        let projection =
            intro2_particle_collision_models(self.cache, self.entities, self.retail_tick);
        self.collision_models = projection.models;
        for id in projection.unresolved_entity_ids {
            if !self.unresolved_collision_entity_ids.contains(&id) {
                self.unresolved_collision_entity_ids.push(id);
            }
        }
        self.owner_motions = self
            .entities
            .iter_all()
            .map(|entity| ParticleOwnerMotion {
                owner_id: entity.id,
                velocity: entity.velocity,
            })
            .collect();
        result
    }
    fn attached_cascade_owner(
        &mut self,
        _: &mut WorldFx,
        request: crate::world_fx::AttachedParticleCascadeOwnerRequest,
    ) -> Result<
        crate::world_fx::AttachedParticleCascadeOwner,
        crate::world_fx::AttachedParticleUpdateBlock,
    > {
        crate::world_fx::sample_attached_particle_cached_owner(self.entities, request)
    }
    fn static_impact(
        &mut self,
        world_fx: &mut WorldFx,
        impact: ParticleStaticImpact,
    ) -> Option<crate::world_fx::ParticleTerrainMutation> {
        let mut delivery = deliver_static_impact(self.static_damage, world_fx, impact);
        if let Intro2StaticImpactDelivery::Delivered(StaticDamageOutcome::ImmediateBurn {
            cell,
            ..
        }) = delivery
        {
            if let Err(error) =
                crate::static_terrain_burn::apply_immediate_static_burn(cell, self.cache, world_fx)
            {
                delivery = Intro2StaticImpactDelivery::BurnCallbackBlocked(error);
            }
        }
        self.static_deliveries.push((impact, delivery));
        None
    }

    fn ground_program_request(
        &mut self,
        _world_fx: &mut WorldFx,
        request: CombatGroundProgramRequest,
    ) -> bool {
        // 441B70 submits 4C98F8 through 28720 at this physical-slot boundary.
        // The shared cell key deduplicates against other queued static programs;
        // 281A0 executes the accepted node on the next world scheduler pass.
        self.static_damage.submit_fireball_ground_program([
            (request.position_raw[0] as u16 >> 8) as u8,
            (request.position_raw[2] as u16 >> 8) as u8,
        ]);
        true
    }
}

/// Collision is always in the live entity's physical basis. The cinematic's
/// camera-facing artwork and presentation proxies do not enter this query.
#[derive(Debug, Default)]
pub struct Intro2ParticleCollisionProjection {
    pub models: Vec<EntityCollisionModel>,
    pub unresolved_entity_ids: Vec<u32>,
}

/// Project F980's admitted live candidates, retaining unresolved eligibility
/// separately from an authored miss. 43FA3D gates before model lookup.
pub fn intro2_particle_collision_models(
    cache: &ResourceCache,
    entities: &EntityManager,
    retail_tick: u32,
) -> Intro2ParticleCollisionProjection {
    let mut projection = Intro2ParticleCollisionProjection::default();
    for entity in entities.iter_collidable() {
        if !entity.active || entity.kind == EntityKind::Trigger {
            continue;
        }
        match particle_model_collision_eligible(&entity.collision) {
            RetailRuntimeValue::Known(true) => {}
            RetailRuntimeValue::Known(false) => continue,
            RetailRuntimeValue::Unresolved => {
                projection.unresolved_entity_ids.push(entity.id);
                continue;
            }
        }
        let Some((model_id, radius_raw)) = entity.model_index.and_then(|id| {
            cache
                .global_model(id)
                .map(|model| (id, model.collision_radius_raw))
        }) else {
            continue;
        };
        if radius_raw != 0 {
            projection.models.push(EntityCollisionModel {
                entity_id: entity.id,
                center_world: entity.position,
                radius_raw,
                model_id,
                orientation_world_from_model: physical_orientation(entity),
                // FCE0 supplies the type+78 callback words and live entity+5C
                // context to 6AEF0, just as the physical model submission.
                anim_vars: entity.presentation_anim_vars(retail_tick),
                state_flags_at_0x08: match entity.collision.state_flags_at_0x08.masked(0xFFFF_FFFF)
                {
                    RetailRuntimeValue::Known(bits) => Some(bits),
                    _ => None,
                },
                capability_flags_at_0x64: entity.capability_flags,
            });
        }
    }
    projection
}

fn physical_orientation(entity: &Entity) -> [[f32; 3]; 3] {
    match entity.physical_body_basis_q31() {
        RetailRuntimeValue::Known(basis) => basis.orientation_world_from_model(),
        RetailRuntimeValue::Unresolved => {
            v2k_render::orientation_from_ypr(std::f32::consts::FRAC_PI_2 - entity.heading, 0.0, 0.0)
        }
    }
}

fn deliver_entity_impact(
    cache: &mut ResourceCache,
    entities: &mut EntityManager,
    world_fx: &mut WorldFx,
    static_damage: &mut StaticDamageScheduler,
    scheduler: &mut SpecializedActorTaskScheduler,
    notifications: &mut GameplayNotifications,
    impact: ParticleEntityImpact,
    retail_tick: u32,
) -> Vec<Intro2EntityImpactStep> {
    let Some(damage) = impact.damage else {
        return vec![Intro2EntityImpactStep::Suppressed];
    };
    // `FUN_00442950` (particle classes 52/68/85) owns the static `FUN_00441180`
    // route: it must precede every target-dispatched F590-style owner, which
    // would otherwise apply the wrong order and force.
    if particle_uses_static_route_entity_hit(impact.source_particle_class) {
        let type9 = crate::ordinary_type9_impact::apply_native_type9_particle_hit(
            entities,
            world_fx,
            scheduler,
            notifications,
            impact,
            retail_tick,
        );
        if !matches!(
            type9,
            crate::ordinary_type9_impact::NativeType9ImpactOutcome::NotApplicable
        ) {
            return vec![Intro2EntityImpactStep::NativeType9Damage(type9)];
        }
        let type26 = crate::intro2_type26_defecate_virus::apply_intro2_type26_particle_hit(
            entities,
            cache,
            world_fx,
            scheduler,
            impact,
            retail_tick,
        );
        if !matches!(
            type26,
            crate::intro2_type26_defecate_virus::Intro2Type26ImpactOutcome::NotApplicable
        ) {
            return vec![Intro2EntityImpactStep::Type26Damage(type26)];
        }
        return vec![Intro2EntityImpactStep::Type13StaticDamage(
            crate::intro2_type13_live::impact::apply_intro2_type13_static_hit(
                crate::intro2_type13_live::impact::Intro2Type13ImpactFrame {
                    entities: entities,
                    resources: cache,
                    world_fx: world_fx,
                    scheduler: scheduler,
                    static_damage: static_damage,
                    notifications: notifications,
                    retail_tick: retail_tick,
                },
                impact,
            ),
        )];
    }
    let meteor = crate::intro2_meteors::impact::apply_intro2_meteor_infected_hit(
        entities, world_fx, scheduler, impact,
    );
    if !matches!(
        meteor,
        crate::intro2_meteors::impact::Intro2MeteorInfectedHitOutcome::NotApplicable
    ) {
        return vec![Intro2EntityImpactStep::MeteorInfectedDamage(meteor)];
    }
    let type9 = crate::ordinary_type9_impact::apply_native_type9_particle_hit(
        entities,
        world_fx,
        scheduler,
        notifications,
        impact,
        retail_tick,
    );
    if !matches!(
        type9,
        crate::ordinary_type9_impact::NativeType9ImpactOutcome::NotApplicable
    ) {
        return vec![Intro2EntityImpactStep::NativeType9Damage(type9)];
    }
    let worker = crate::intro2_type8::impact::apply_intro2_type8_particle_hit(
        entities,
        world_fx,
        scheduler,
        impact,
        retail_tick,
    );
    if !matches!(
        worker,
        crate::intro2_type8::impact::Intro2Type8ImpactOutcome::NotApplicable
    ) {
        return vec![Intro2EntityImpactStep::Type8Damage(worker)];
    }
    let turret = crate::intro2_gun_turret::impact::apply_intro2_gun_turret_particle_hit(
        crate::intro2_gun_turret::impact::Intro2GunTurretImpactFrame {
            world: crate::intro2_gun_turret::impact::GunTurretImpactWorld::Cinematic,
            entities,
            resources: cache,
            static_damage,
            notifications,
            world_fx,
            scheduler,
            retail_tick,
        },
        impact,
    );
    if !matches!(
        turret,
        crate::intro2_gun_turret::impact::Intro2GunTurretImpactOutcome::NotApplicable
    ) {
        return vec![Intro2EntityImpactStep::GunTurretDamage(turret)];
    }
    let flyer = crate::intro2_flyer_impact::apply_native_flyer_particle_hit(
        crate::shared_actor_impact::SharedActorImpactFrame {
            entities: entities,
            resources: cache,
            world_fx: world_fx,
            scheduler: scheduler,
            notifications: notifications,
            retail_tick: retail_tick,
        },
        impact,
    );
    if !matches!(
        flyer,
        crate::intro2_flyer_impact::NativeFlyerImpactOutcome::NotApplicable
    ) {
        return vec![Intro2EntityImpactStep::FlyerDamage(flyer)];
    }
    let type10 = crate::intro2_type10::impact::apply_intro2_type10_particle_hit(
        entities,
        cache,
        world_fx,
        scheduler,
        impact,
        retail_tick,
    );
    if !matches!(
        type10,
        crate::intro2_type10::impact::Intro2Type10ImpactOutcome::NotApplicable
    ) {
        return vec![Intro2EntityImpactStep::Type10Damage(type10)];
    }
    let type57 = crate::intro2_type57::impact::apply_intro2_type57_particle_hit(
        entities,
        cache,
        world_fx,
        scheduler,
        impact,
        retail_tick,
    );
    if !matches!(
        type57,
        crate::intro2_type57::impact::Intro2Type57ImpactOutcome::NotApplicable
    ) {
        return vec![Intro2EntityImpactStep::Type57Damage(type57)];
    }
    let type13 = crate::intro2_type13_live::impact::apply_intro2_type13_particle_hit(
        crate::intro2_type13_live::impact::Intro2Type13ImpactFrame {
            entities: entities,
            resources: cache,
            world_fx: world_fx,
            scheduler: scheduler,
            static_damage: static_damage,
            notifications: notifications,
            retail_tick: retail_tick,
        },
        impact,
    );
    if !matches!(
        type13,
        crate::intro2_type13_live::impact::Intro2Type13ImpactOutcome::NotApplicable
    ) {
        return vec![Intro2EntityImpactStep::Type13Damage(type13)];
    }
    let type26 = crate::intro2_type26_defecate_virus::apply_intro2_type26_particle_hit(
        entities,
        cache,
        world_fx,
        scheduler,
        impact,
        retail_tick,
    );
    if !matches!(
        type26,
        crate::intro2_type26_defecate_virus::Intro2Type26ImpactOutcome::NotApplicable
    ) {
        return vec![Intro2EntityImpactStep::Type26Damage(type26)];
    }
    let type16 = crate::intro2_type16::impact::apply_intro2_type16_particle_hit(
        entities,
        cache,
        world_fx,
        scheduler,
        impact,
        retail_tick,
    );
    if !matches!(
        type16,
        crate::intro2_type16::impact::Intro2Type16ImpactOutcome::NotApplicable
    ) {
        return vec![Intro2EntityImpactStep::Type16Damage(type16)];
    }
    let type58 = crate::intro2_type58::impact::apply_intro2_type58_particle_hit(
        crate::shared_actor_impact::SharedActorImpactFrame {
            entities,
            resources: cache,
            world_fx,
            scheduler,
            notifications,
            retail_tick,
        },
        impact,
    );
    if !matches!(
        type58,
        crate::intro2_type58::impact::Intro2Type58ImpactOutcome::NotApplicable
    ) {
        return vec![Intro2EntityImpactStep::Type58Damage(type58)];
    }
    let type66 = crate::intro2_type66::impact::apply_intro2_type66_particle_hit(
        entities,
        world_fx,
        scheduler,
        impact,
        retail_tick,
    );
    if !matches!(
        type66,
        crate::intro2_type66::impact::Intro2Type66ImpactOutcome::NotApplicable
    ) {
        return vec![Intro2EntityImpactStep::Type66Damage(type66)];
    }
    let type94 = crate::intro2_type94::impact::apply_intro2_type94_particle_hit(
        entities,
        cache,
        world_fx,
        scheduler,
        impact,
        retail_tick,
    );
    if !matches!(
        type94,
        crate::intro2_type94::impact::Intro2Type94ImpactOutcome::NotApplicable
    ) {
        return vec![Intro2EntityImpactStep::Type94Damage(type94)];
    }
    let type47 = crate::shared_actor_impact::apply_shared_type47_particle_hit(
        crate::shared_actor_impact::SharedActorImpactFrame {
            entities,
            resources: cache,
            world_fx,
            scheduler,
            notifications,
            retail_tick,
        },
        impact,
    );
    if !matches!(
        type47,
        crate::shared_actor_impact::type47::NativeType47ImpactOutcome::NotApplicable
    ) {
        return vec![Intro2EntityImpactStep::NativeType47Damage(type47)];
    }
    let type17 = crate::shared_actor_impact::apply_shared_type17_particle_hit(
        crate::shared_actor_impact::SharedActorImpactFrame {
            entities,
            resources: cache,
            world_fx,
            scheduler,
            notifications,
            retail_tick,
        },
        impact,
    );
    if !matches!(
        type17,
        crate::intro2_type17::impact::Intro2Type17ImpactOutcome::NotApplicable
    ) {
        return vec![Intro2EntityImpactStep::Type17Damage(type17)];
    }
    let type53 = crate::shared_actor_impact::apply_shared_type53_particle_hit(
        crate::shared_actor_impact::SharedActorImpactFrame {
            entities,
            resources: cache,
            world_fx,
            scheduler,
            notifications,
            retail_tick,
        },
        impact,
    );
    if !matches!(
        type53,
        crate::intro2_type53::impact::Intro2Type53ImpactOutcome::NotApplicable
    ) {
        return vec![Intro2EntityImpactStep::Type53Damage(type53)];
    }
    if entities.iter_all().any(|entity| {
        entity.id == impact.target_entity_id
            && entity.entity_type == 122
            && entity.native_type122_runtime.is_some()
    }) {
        return vec![Intro2EntityImpactStep::Type122Damage(
            crate::shared_actor_impact::apply_shared_type122_particle_hit(
                crate::shared_actor_impact::SharedActorImpactFrame {
                    entities,
                    resources: cache,
                    world_fx,
                    scheduler,
                    notifications,
                    retail_tick,
                },
                impact,
            ),
        )];
    }
    if crate::world_fx::particle_uses_fun_0043f7c0_entity_hit(impact.source_particle_class) {
        return vec![Intro2EntityImpactStep::UnsupportedCuredTarget];
    }
    let mut steps = Vec::new();
    let infected = particle_uses_fun_0043f780_entity_hit(impact.source_particle_class);
    if infected {
        if let Some(applied) =
            entities.apply_fun_00411250_infected_model_bit(impact.target_entity_id)
        {
            if let Some(sound_id) = applied.sound_id {
                world_fx.queue_fixed_positional_sound_raw(sound_id, applied.position_raw);
            }
            steps.push(Intro2EntityImpactStep::InfectedModel(applied));
        }
    } else if damage.delivery_record().is_none() {
        return vec![Intro2EntityImpactStep::BirthProvenanceUnavailable];
    }
    {
        let reselection = apply_type47_impact_c690_live(
            entities,
            impact.target_entity_id,
            world_fx,
            Type47ImpactLiveRequest {
                entry: if infected {
                    EntityHitEntry::Infected
                } else {
                    EntityHitEntry::PrimaryProjectile
                },
                retail_tick,
            },
        );
        if let Ok(Type47ImpactLiveOutcome::Class12Published { publication, .. }) = &reselection {
            if let Err(failure) =
                register_native_common_dying(entities, scheduler, publication.owner.entity_id())
            {
                steps.push(Intro2EntityImpactStep::DeathRegistrationRejected(format!(
                    "{failure:?}"
                )));
            }
        }
        if !matches!(reselection, Ok(Type47ImpactLiveOutcome::NotApplicable)) {
            let blocked = reselection.is_err();
            steps.push(Intro2EntityImpactStep::Type47Reselection(reselection));
            if blocked {
                return steps;
            }
        }
    }

    let impulse = apply_type47_impact_reaction_after_c690(
        entities,
        world_fx,
        impact.target_entity_id,
        damage.packet.impact_sum_raw(),
        impact.velocity_raw,
    );
    if !matches!(impulse, Type47ImpactReactionOutcome::NotApplicable) {
        let blocked = matches!(impulse, Type47ImpactReactionOutcome::Blocked { .. });
        steps.push(Intro2EntityImpactStep::Type47Impulse(impulse));
        if blocked {
            return steps;
        }
    }
    let Some(delivery) = impact.damage_delivery_record() else {
        steps.push(Intro2EntityImpactStep::BirthProvenanceUnavailable);
        return steps;
    };
    let type47_damage = apply_type47_checked_damage_after_c690(
        entities,
        world_fx,
        impact.target_entity_id,
        Type47CheckedDamageRequest {
            delivery,
            entry: if infected {
                EntityHitEntry::Infected
            } else {
                EntityHitEntry::PrimaryProjectile
            },
            retail_tick,
        },
    );
    if let Type47CheckedDamageOutcome::Lethal {
        publication: Some(publication),
        ..
    } = &type47_damage
    {
        if let Err(failure) =
            register_native_common_dying(entities, scheduler, publication.owner.entity_id())
        {
            steps.push(Intro2EntityImpactStep::DeathRegistrationRejected(format!(
                "{failure:?}"
            )));
        }
    }
    if !matches!(type47_damage, Type47CheckedDamageOutcome::NotApplicable) {
        steps.push(Intro2EntityImpactStep::Type47Damage(type47_damage));
        return steps;
    }

    let type9_damage = entities.apply_fun_00411250_type9_checked_damage(
        impact.target_entity_id,
        damage.packet,
        world_fx,
        retail_tick as i32,
        Some(notifications),
    );
    match type9_damage {
        Fun00411250Type9DamageOutcome::Survived {
            generic_hit_sound_id: Some(sound_id),
            position_raw,
        } => {
            world_fx.queue_fixed_positional_sound_raw(sound_id, position_raw);
        }
        Fun00411250Type9DamageOutcome::Lethal {
            task_lease: Some(task_lease),
            ..
        } => {
            let native_intro2 = entities
                .iter_all()
                .find(|entity| entity.id == impact.target_entity_id)
                .is_some_and(crate::intro2_type9::intro2_type9_allocation_authenticates);
            if native_intro2 {
                scheduler.register_intro2_type9_class14(
                    crate::intro2_type9_class14::Intro2Type9Class14Owner::adopt(task_lease),
                );
            } else if let Err(failure) =
                scheduler.adopt_ordinary_type9_class14_after_checked_death(task_lease)
            {
                steps.push(Intro2EntityImpactStep::DeathRegistrationRejected(format!(
                    "{:?}",
                    failure.conflict
                )));
            }
        }
        _ => {}
    }
    if type9_damage != Fun00411250Type9DamageOutcome::NotType9 {
        steps.push(Intro2EntityImpactStep::Type9Damage(type9_damage));
        return steps;
    }
    let fixed = entities.apply_audited_base_factory_projectile_damage(
        impact.target_entity_id,
        CheckedProjectileDamageRequest {
            delivery,
            entry: if infected {
                EntityHitEntry::Infected
            } else {
                EntityHitEntry::PrimaryProjectile
            },
            retail_tick,
        },
    );
    queue_fixed_actor_damage_audio(world_fx, &fixed);
    steps.push(Intro2EntityImpactStep::FixedActorDamage(fixed));
    steps
}

fn register_native_common_dying(
    entities: &EntityManager,
    scheduler: &mut SpecializedActorTaskScheduler,
    entity_id: u32,
) -> Result<(), crate::intro2_common_dying::Intro2CommonDyingBlock> {
    let owner = crate::intro2_common_dying::Intro2CommonDyingOwner::adopt(entities, entity_id)?;
    scheduler.register_intro2_common_dying(owner);
    Ok(())
}

fn deliver_static_impact(
    scheduler: &mut StaticDamageScheduler,
    world_fx: &mut WorldFx,
    impact: ParticleStaticImpact,
) -> Intro2StaticImpactDelivery {
    let Some(damage) = impact.damage else {
        return Intro2StaticImpactDelivery::Suppressed;
    };
    let Some(current) = impact.current else {
        return Intro2StaticImpactDelivery::CurrentTargetUnavailable;
    };
    let outcome = scheduler.submit_hit(current.target, damage.packet, &mut || {
        world_fx.next_shared_retail_random_u16()
    });
    debug_assert!(
        !matches!(outcome, StaticDamageOutcome::BurnedKind10Transition { .. }),
        "the admitted F800 packets cannot enter kind-10's immediate mutation"
    );
    Intro2StaticImpactDelivery::Delivered(outcome)
}

fn queue_fixed_actor_damage_audio(
    world_fx: &mut WorldFx,
    outcome: &CheckedProjectileDamageOutcome,
) {
    let (position_raw, sounds) = match outcome {
        CheckedProjectileDamageOutcome::Applied(applied) => (
            applied.target_position_raw,
            [
                applied.transition.generic_hit_sound_id,
                applied
                    .accepted_hit_presentation
                    .and_then(|presentation| presentation.sound_id),
                None,
            ],
        ),
        CheckedProjectileDamageOutcome::ProgressiveDeathStarted(started) => (
            started.target_position_raw,
            [
                started.generic_hit_sound_id,
                started.death_sound_id,
                started
                    .accepted_hit_presentation
                    .and_then(|presentation| presentation.sound_id),
            ],
        ),
        CheckedProjectileDamageOutcome::HiveDeathStarted(started) => (
            started.target_position_raw,
            [
                started.generic_hit_sound_id,
                started
                    .accepted_hit_presentation
                    .and_then(|presentation| presentation.sound_id),
                None,
            ],
        ),
        _ => return,
    };
    for sound in sounds.into_iter().flatten() {
        world_fx.queue_fixed_positional_sound_raw(sound, position_raw);
    }
}

#[cfg(test)]
#[path = "intro2_effects/type122_tests.rs"]
mod type122_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actor_task_dispatcher::ActorTaskRuntime;
    use crate::actor_task_owner::ActorTaskSlot;
    use crate::damage::PRIMARY_PROJECTILE_DAMAGE_PACKET;
    use crate::intro2_type47_live::world::native_intro2_fixture;
    use crate::world_fx::{
        BallisticDamageRequest, DescriptorParticleRequest, ParticleOwnerAtBirth,
    };

    #[v2k_test_support::retail_test]
    fn intro2_particle_models_consume_live_entity_callback_words() {
        let Some((mut session, mut entities, _)) = native_intro2_fixture() else {
            return;
        };
        let entity = entities
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(0))
            .unwrap();
        let id = entity.id;
        let runtime = entities
            .entity_mut(id)
            .unwrap()
            .intro2_type13_common_mover_runtime
            .as_mut()
            .unwrap();
        runtime.sub_k_output_raw = [123, -456];
        runtime.sub_l_output_raw = [789, -321];
        let projection = intro2_particle_collision_models(&mut session.cache, &entities, 37);
        assert!(projection.unresolved_entity_ids.is_empty());
        let model = projection
            .models
            .iter()
            .find(|model| model.entity_id == id)
            .unwrap();
        // Type+78 binds K to selectors11/10, then L to6/7. FCE0 uses those
        // same entity-owned words when constructing its collision walker.
        assert_eq!(
            [
                model.anim_vars.dynamic[11],
                model.anim_vars.dynamic[10],
                model.anim_vars.dynamic[6],
                model.anim_vars.dynamic[7]
            ],
            [123, -456, 789, -321]
        );
    }

    #[v2k_test_support::retail_test]
    fn mutable_intro2_static_host_commits_null_program_burn_before_next_context() {
        use crate::static_damage::{
            StaticDamageTarget, StaticDamageTargetState, KIND_4_STATIC_OBJECT,
        };
        let Some((mut session, mut entities, _)) = native_intro2_fixture() else {
            return;
        };
        let cell = [3, 4];
        // Isolate the host's already-resolved F800 boundary. Static geometry and
        // post-F610 target selection have independent physical-traversal tests.
        let attribute = session
            .cache
            .terrain_objects()
            .unwrap()
            .records
            .iter()
            .position(|record| record.kind_index == KIND_4_STATIC_OBJECT)
            .unwrap() as u8;
        let terrain_cell = &mut session.cache.level_terrain_mut().unwrap().cells[3 * 256 + 4];
        terrain_cell.terrain_type = 0x10;
        terrain_cell.attribute = attribute;
        terrain_cell.height = 16;
        let target = StaticDamageTarget {
            cell,
            state: StaticDamageTargetState {
                kind_index: KIND_4_STATIC_OBJECT,
                terrain_type: 0x10,
                terrain_height_byte: 16,
                collision_radius_raw: 200,
                effect_extent_raw: 200,
            },
        };
        let static_impact = ParticleStaticImpact {
            source_particle_class: 1,
            source_owner_id: Some(7),
            position_world: [4.0, 2.0, 5.0],
            cell,
            attribute: 1,
            terrain_type: 0x10,
            model_id: 0,
            kind_index: KIND_4_STATIC_OBJECT,
            current: Some(crate::static_damage_live::CurrentStaticDamageSnapshot {
                target,
                attribute: 1,
                model_id: 0,
            }),
            damage: Some(BallisticDamageRequest {
                packet: crate::damage::PRIMARY_PROJECTILE_DAMAGE_PACKET,
                source_entity_type_at_birth: Some(46),
                source_owner_id: Some(7),
            }),
        };
        let mut static_damage = StaticDamageScheduler::new();
        let mut scheduler = SpecializedActorTaskScheduler::new();
        let mut notifications = GameplayNotifications::new();
        let mut fx = WorldFx::new();
        for _ in 0..8 {
            fx.advance_frame_pacing(20_000);
        }
        let mut host = Intro2ParticleHost {
            cache: &mut session.cache,
            entities: &mut entities,
            static_damage: &mut static_damage,
            scheduler: &mut scheduler,
            notifications: &mut notifications,
            retail_tick: 1,
            owner_motions: Vec::new(),
            collision_models: Vec::new(),
            unresolved_collision_entity_ids: Vec::new(),
            entity_deliveries: Vec::new(),
            static_deliveries: Vec::new(),
        };
        assert_eq!(
            host.static_impact(&mut fx, static_impact),
            None,
            "mutable hosts already committed the burn"
        );
        let ParticleEnvironment::Terrain(next) = host.context().environment else {
            panic!("current terrain")
        };
        assert_eq!(next.terrain.cell(3, 4).unwrap().terrain_type, 0x18);
        assert!(matches!(
            host.static_deliveries[0].1,
            Intro2StaticImpactDelivery::Delivered(StaticDamageOutcome::ImmediateBurn {
                severity_raw: 3000,
                sample: None,
                ..
            })
        ));
        assert!(host.cache.take_level_terrain_presentation_dirty());
        assert_eq!(host.static_damage.active_program_count(), 0);
        let particles = fx.test_particles_in_virgin_birth_order();
        assert_eq!(
            particles.len(),
            81,
            "null program still invokes the burn listener"
        );
        assert!(particles[..80]
            .iter()
            .all(|particle| particle.source_class == 93));
        assert_eq!(particles[80].source_class, 18);
        assert_eq!(fx.take_positional_sounds()[0].sound_id, 62);
    }

    fn impact(target_entity_id: u32) -> ParticleEntityImpact {
        ParticleEntityImpact {
            source_particle_class: 1,
            impact_position_argument_va: 0x004D_CF48,
            target_entity_id,
            position_world: [0.0; 3],
            velocity_raw: [0, 0, 8192],
            damage: Some(BallisticDamageRequest {
                packet: PRIMARY_PROJECTILE_DAMAGE_PACKET,
                source_entity_type_at_birth: Some(13),
                source_owner_id: Some(1),
            }),
        }
    }

    #[v2k_test_support::retail_test]
    fn dormant_type77_is_not_a_particle_candidate_and_enabled_policy_stays_explicit() {
        let Some((mut session, mut entities, _)) = native_intro2_fixture() else {
            return;
        };
        let id = entities
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(45))
            .unwrap()
            .id;
        entities.disable_authored_behavior_components();
        let before = entities.entity_mut(id).unwrap().collision.clone();
        let projection = intro2_particle_collision_models(&session.cache, &entities, 190);
        assert!(!projection.models.iter().any(|model| model.entity_id == id));
        assert!(!projection.unresolved_entity_ids.contains(&id));
        assert_eq!(entities.entity_mut(id).unwrap().collision, before);
        entities.set_authored_behavior_components_enabled(45, true);
        let projection = intro2_particle_collision_models(&session.cache, &entities, 190);
        assert!(projection.models.iter().any(|model| model.entity_id == id));
        // A genuinely eligible hit still reaches the unimplemented native
        // wrapper boundary; the gather fix must not pretend its policy exists.
        let steps = deliver_entity_impact(
            &mut session.cache,
            &mut entities,
            &mut WorldFx::new(),
            &mut StaticDamageScheduler::new(),
            &mut SpecializedActorTaskScheduler::new(),
            &mut GameplayNotifications::new(),
            impact(id),
            190,
        );
        assert!(
            matches!(
                steps.as_slice(),
                [Intro2EntityImpactStep::FixedActorDamage(
                    CheckedProjectileDamageOutcome::Unresolved {
                        reason:
                            crate::entity::CheckedProjectileDamageUnresolved::TargetSurvivorPolicy,
                        ..
                    }
                )]
            ),
            "{steps:?}"
        );
    }

    #[v2k_test_support::retail_test]
    fn class68_entity_hit_routes_to_static_owner_not_f590_target_owners() {
        let Some((mut session, mut entities, _)) = native_intro2_fixture() else {
            return;
        };
        let id = entities
            .iter_all()
            .find(|entity| {
                entity.authored_spawn_index
                    == Some(crate::intro2_type13_live::INTRO2_TYPE13_SPAWN_INDEX)
            })
            .unwrap()
            .id;
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert_eq!(scheduler.adopt_intro2_type13_search_attack(&entities), 1);
        let steps = deliver_entity_impact(
            &mut session.cache,
            &mut entities,
            &mut WorldFx::new(),
            &mut StaticDamageScheduler::new(),
            &mut scheduler,
            &mut GameplayNotifications::new(),
            ParticleEntityImpact {
                source_particle_class: 68,
                impact_position_argument_va: 0,
                target_entity_id: id,
                position_world: [0.0, 1000.0, 0.0],
                velocity_raw: [254, -471, 508],
                damage: Some(BallisticDamageRequest {
                    packet: crate::damage::CLASS68_STATIC_ROUTE_DAMAGE_PACKET,
                    source_entity_type_at_birth: Some(57),
                    source_owner_id: Some(53),
                }),
            },
            1473,
        );
        assert!(
            matches!(
                steps.as_slice(),
                [Intro2EntityImpactStep::Type13StaticDamage(
                    crate::intro2_type13_live::impact::Intro2Type13StaticOutcome::Applied(_)
                )]
            ),
            "{steps:?}"
        );
    }

    #[v2k_test_support::retail_test]
    fn bat_class68_model_collision_kills_native_peasant_through_11180() {
        for suppressed in [false, true] {
            let Some((mut session, mut entities, _)) = native_intro2_fixture() else {
                return;
            };
            let target = entities
                .iter_all()
                .find(|entity| entity.entity_type == 9)
                .unwrap();
            let id = target.id;
            let position_raw = target.position_raw();
            let source_id = entities
                .iter_all()
                .find(|entity| entity.entity_type == 57)
                .unwrap()
                .id;
            let mut scheduler = SpecializedActorTaskScheduler::new();
            assert_eq!(scheduler.adopt_intro2_type9(&mut entities), 13);
            let mut fx = WorldFx::new();
            if !suppressed {
                // Reproduce the previous router's exclusive Type13 branch
                // before exercising the corrected production model sweep.
                let before = crate::intro2_type13_live::impact::apply_intro2_type13_static_hit(
                    crate::intro2_type13_live::impact::Intro2Type13ImpactFrame {
                        entities: &mut entities,
                        resources: &mut session.cache,
                        world_fx: &mut fx,
                        scheduler: &mut scheduler,
                        static_damage: &mut crate::static_damage::StaticDamageScheduler::new(),
                        notifications:
                            &mut crate::gameplay_notifications::GameplayNotifications::new(),
                        retail_tick: 25,
                    },
                    ParticleEntityImpact {
                        source_particle_class: 68,
                        impact_position_argument_va: 0,
                        target_entity_id: id,
                        position_world: position_raw.map(|word| f32::from(word) / 256.0),
                        velocity_raw: [700, -200, 900],
                        damage: Some(BallisticDamageRequest {
                            packet: crate::damage::CLASS68_STATIC_ROUTE_DAMAGE_PACKET,
                            source_entity_type_at_birth: Some(57),
                            source_owner_id: Some(source_id),
                        }),
                    },
                );
                assert!(matches!(before,
                    crate::intro2_type13_live::impact::Intro2Type13StaticOutcome::Blocked {
                        reason: crate::intro2_type13_live::impact::Intro2Type13StaticBlock::UnsupportedStaticRouteTarget {
                            entity_type: Some(9),
                        }, committed_prefix: false,
                    }));
                assert_eq!(
                    entities.entity_mut(id).unwrap().collision.health_raw,
                    RetailRuntimeValue::Known(1500)
                );
                assert_eq!(fx.pending_event_count(), 0);
            }
            let allocation = fx.materialize_descriptor_particle_request(
                DescriptorParticleRequest {
                    source_class: 68,
                    position_raw,
                    velocity_raw: [700, -200, 900],
                    owner: Some(ParticleOwnerAtBirth {
                        entity_id: source_id,
                        entity_type: 57,
                    }),
                    suppresses_impact_damage: suppressed,
                },
                ParticleEnvironment::Dry,
                0,
            );
            if suppressed {
                assert!(
                    allocation.is_none(),
                    "class68 rejects suppressed births at 40A60"
                );
                continue;
            }
            allocation.unwrap();
            let report = update_intro2_effects(Intro2EffectsFrame {
                cache: &mut session.cache,
                entities: &mut entities,
                world_fx: &mut fx,
                static_damage: &mut StaticDamageScheduler::new(),
                scheduler: &mut scheduler,
                notifications: &mut GameplayNotifications::new(),
                elapsed_micros: 0,
                retail_tick: 25,
            });
            let delivery = report
                .entity_deliveries
                .iter()
                .find(|delivery| delivery.impact.target_entity_id == id)
                .unwrap_or_else(|| {
                    panic!("native man2 geometry must receive bat fire: {report:?}")
                });
            let [Intro2EntityImpactStep::NativeType9Damage(
                crate::ordinary_type9_impact::NativeType9ImpactOutcome::Applied(applied),
            )] = delivery.steps.as_slice()
            else {
                panic!(
                    "the previous Type13-only route rejected this peasant: {:?}",
                    delivery.steps
                )
            };
            assert_eq!(applied.filtered_damage_raw, 5600);
            assert!(applied.death_publication.is_some());
            assert_eq!(
                entities.entity_mut(id).unwrap().collision.health_raw,
                RetailRuntimeValue::Known(0)
            );
            assert_eq!(scheduler.family_for(id), Some(crate::specialized_actor_task_production::SpecializedActorTaskFamily::Intro2Type9Class14));
            assert_eq!(
                entities
                    .entity_mut(id)
                    .unwrap()
                    .collision
                    .last_hit_presentation_tick_at_0x34,
                RetailRuntimeValue::Known(25)
            );
            let sounds = fx.take_positional_sounds();
            let hit = sounds
                .iter()
                .position(|sound| sound.sound_id == 95)
                .expect("11180 alive cue");
            let death = sounds
                .iter()
                .position(|sound| sound.sound_id == 35)
                .expect("class14 death cue");
            assert!(
                hit < death,
                "11180's accepted cue precedes death: {sounds:?}"
            );
            let attached = fx
                .test_particles_in_virgin_birth_order()
                .into_iter()
                .filter(|particle| particle.source_class == 84)
                .collect::<Vec<_>>();
            assert_eq!(attached.len(), usize::from(!suppressed));
            if let Some(attached) = attached.first() {
                assert_eq!(attached.attached_owner_handle, Some(id));
            }
        }
    }

    #[v2k_test_support::retail_test]
    fn particle_projection_retains_unknown_ids_once_across_inline_rebuilds() {
        let Some((mut session, mut entities, _)) = native_intro2_fixture() else {
            return;
        };
        entities.disable_authored_behavior_components();
        let ids = [21, 45].map(|spawn| {
            entities
                .iter_all()
                .find(|entity| entity.authored_spawn_index == Some(spawn))
                .unwrap()
                .id
        });
        for id in ids {
            entities
                .entity_mut(id)
                .unwrap()
                .collision
                .state_flags_at_0x08
                .invalidate(0x8000);
        }
        let projection = intro2_particle_collision_models(&session.cache, &entities, 190);
        assert_eq!(projection.unresolved_entity_ids, ids);
        assert!(projection
            .models
            .iter()
            .all(|model| !ids.contains(&model.entity_id)));
        let mut static_damage = StaticDamageScheduler::new();
        let mut scheduler = SpecializedActorTaskScheduler::new();
        let mut notifications = GameplayNotifications::new();
        let mut host = Intro2ParticleHost {
            cache: &mut session.cache,
            entities: &mut entities,
            static_damage: &mut static_damage,
            scheduler: &mut scheduler,
            notifications: &mut notifications,
            retail_tick: 190,
            owner_motions: Vec::new(),
            collision_models: projection.models,
            unresolved_collision_entity_ids: projection.unresolved_entity_ids,
            entity_deliveries: Vec::new(),
            static_deliveries: Vec::new(),
        };
        // Controlled direct callbacks exercise the production refresh seam;
        // the unresolved candidates themselves are absent from the sweep.
        let mut fx = WorldFx::new();
        host.entity_impact(&mut fx, impact(ids[1]));
        host.entity_impact(&mut fx, impact(ids[1]));
        assert_eq!(host.unresolved_collision_entity_ids, ids);
    }

    #[test]
    fn infected_descriptor_uses_its_static_source_words_not_particle_birth_owner() {
        let mut hit = impact(1);
        hit.source_particle_class = 5;
        hit.damage = Some(BallisticDamageRequest {
            packet: crate::damage::FUN_0043F780_DAMAGE_PACKET,
            source_entity_type_at_birth: Some(46),
            source_owner_id: Some(0x1234),
        });
        assert_eq!(
            hit.damage_delivery_record(),
            Some(crate::damage::FUN_0043F780_DAMAGE_DELIVERY)
        );
        hit.damage.as_mut().unwrap().source_entity_type_at_birth = None;
        hit.damage.as_mut().unwrap().source_owner_id = None;
        assert_eq!(
            hit.damage_delivery_record(),
            Some(crate::damage::FUN_0043F780_DAMAGE_DELIVERY)
        );
        let mut ordinary = impact(1);
        ordinary
            .damage
            .as_mut()
            .unwrap()
            .source_entity_type_at_birth = Some(34);
        assert_eq!(
            ordinary
                .damage_delivery_record()
                .unwrap()
                .source_entity_type_raw,
            34
        );
        ordinary
            .damage
            .as_mut()
            .unwrap()
            .source_entity_type_at_birth = None;
        assert_eq!(ordinary.damage_delivery_record(), None);
        hit.damage.as_mut().unwrap().packet = PRIMARY_PROJECTILE_DAMAGE_PACKET;
        assert_eq!(
            hit.damage_delivery_record(),
            None,
            "descriptor/template mismatch must fail closed"
        );
    }

    #[v2k_test_support::retail_test]
    fn native_meteor_fragment_completes_type47_checked_damage() {
        let Some((mut session, mut manager, id)) = native_intro2_fixture() else {
            return;
        };
        let health = manager.entity_mut(id).unwrap().collision.health_raw;
        let mut hit = impact(id);
        hit.source_particle_class = 16;
        hit.damage = Some(BallisticDamageRequest {
            packet: crate::damage::BALLISTIC_PARTICLE_DAMAGE_PACKET,
            source_entity_type_at_birth: Some(34),
            source_owner_id: Some(0x0476_0001),
        });
        let steps = deliver_entity_impact(
            &mut session.cache,
            &mut manager,
            &mut WorldFx::new(),
            &mut StaticDamageScheduler::new(),
            &mut SpecializedActorTaskScheduler::new(),
            &mut GameplayNotifications::new(),
            hit,
            25,
        );
        assert!(
            steps.iter().any(|step| matches!(
                step,
                Intro2EntityImpactStep::Type47Damage(Type47CheckedDamageOutcome::Survived {
                    accepted_damage_raw: 500,
                    ..
                })
            )),
            "{steps:?}"
        );
        let RetailRuntimeValue::Known(health_before) = health else {
            panic!("native health")
        };
        assert_eq!(
            manager.entity_mut(id).unwrap().collision.health_raw,
            RetailRuntimeValue::Known(health_before - 500)
        );
    }

    #[v2k_test_support::retail_test]
    fn native_infected_type47_reselects_before_impulse_and_preserves_primary_stamp() {
        let Some((mut session, mut manager, id)) = native_intro2_fixture() else {
            return;
        };
        let entity = manager.entity_mut(id).unwrap();
        entity.collision.health_raw = RetailRuntimeValue::Known(10_000);
        entity.collision.last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(17);
        let mut hit = impact(id);
        hit.source_particle_class = 5;
        hit.damage = Some(BallisticDamageRequest {
            packet: crate::damage::FUN_0043F780_DAMAGE_PACKET,
            source_entity_type_at_birth: None,
            source_owner_id: None,
        });
        let mut fx = WorldFx::new();
        let steps = deliver_entity_impact(
            &mut session.cache,
            &mut manager,
            &mut fx,
            &mut StaticDamageScheduler::new(),
            &mut SpecializedActorTaskScheduler::new(),
            &mut GameplayNotifications::new(),
            hit,
            25,
        );
        assert!(
            matches!(
                steps.as_slice(),
                [
                    Intro2EntityImpactStep::InfectedModel(_),
                    Intro2EntityImpactStep::Type47Reselection(Ok(
                        Type47ImpactLiveOutcome::GuardPublished { .. }
                            | Type47ImpactLiveOutcome::WanderPublished { .. }
                    )),
                    Intro2EntityImpactStep::Type47Impulse(Type47ImpactReactionOutcome::Applied(_)),
                    Intro2EntityImpactStep::Type47Damage(
                        Type47CheckedDamageOutcome::FilteredZero { .. }
                    ),
                ]
            ),
            "{steps:?}"
        );
        assert_eq!(
            manager
                .entity_mut(id)
                .unwrap()
                .collision
                .last_hit_presentation_tick_at_0x34,
            RetailRuntimeValue::Known(17)
        );
        assert_eq!(fx.pending_event_count(), 0);
        assert_eq!(
            manager.entity_mut(id).unwrap().collision.health_raw,
            RetailRuntimeValue::Known(10_000)
        );
    }

    #[v2k_test_support::retail_test]
    fn native_spawn24_hive_filters_f780_without_player_feedback_or_primary_stamp() {
        let Some((mut session, mut manager, _)) = native_intro2_fixture() else {
            return;
        };
        let hive = manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(24))
            .unwrap();
        assert_eq!(hive.entity_type, 67);
        let id = hive.id;
        manager
            .entity_mut(id)
            .unwrap()
            .collision
            .last_hit_presentation_tick_at_0x34 = RetailRuntimeValue::Known(17);
        let mut expected = manager.entity_mut(id).unwrap().collision.clone();
        expected.state_flags_at_0x08.overwrite(0x2000, 0x2000);
        let mut hit = impact(id);
        hit.source_particle_class = 5;
        hit.damage = Some(BallisticDamageRequest {
            packet: crate::damage::FUN_0043F780_DAMAGE_PACKET,
            source_entity_type_at_birth: None,
            source_owner_id: Some(id),
        });
        let mut fx = WorldFx::new();
        let mut control = WorldFx::new();
        let steps = deliver_entity_impact(
            &mut session.cache,
            &mut manager,
            &mut fx,
            &mut StaticDamageScheduler::new(),
            &mut SpecializedActorTaskScheduler::new(),
            &mut GameplayNotifications::new(),
            hit,
            25,
        );
        assert!(
            matches!(
                steps.as_slice(),
                [
                    Intro2EntityImpactStep::InfectedModel(_),
                    Intro2EntityImpactStep::FixedActorDamage(
                        CheckedProjectileDamageOutcome::FilteredOut
                    ),
                ]
            ),
            "{steps:?}"
        );
        assert_eq!(manager.entity_mut(id).unwrap().collision, expected);
        assert_eq!(fx.pending_event_count(), 0);
        assert!(fx.take_positional_sounds().is_empty());
        assert_eq!(fx.particle_count(), 0);
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            control.next_shared_retail_random_u16()
        );
    }
    #[v2k_test_support::retail_test]
    fn native_meteor_f780_routes_to_retained_class19_before_fixed_actor_fallback() {
        let Some((mut session, mut manager, _)) = native_intro2_fixture() else {
            return;
        };
        let id = manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(33))
            .unwrap()
            .id;
        manager
            .entity_mut(id)
            .unwrap()
            .collision
            .state_flags_at_0x08
            .overwrite(0x8c00_8000, 0x0400_8000);
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert_eq!(scheduler.adopt_intro2_meteors(&manager), 4);
        let owner = scheduler.intro2_meteor_owner(id).unwrap();
        let mut hit = impact(id);
        hit.source_particle_class = 5;
        hit.damage = Some(BallisticDamageRequest {
            packet: crate::damage::FUN_0043F780_DAMAGE_PACKET,
            source_entity_type_at_birth: None,
            source_owner_id: Some(6),
        });
        let mut fx = WorldFx::new();
        let mut oracle = WorldFx::new();
        for _ in 0..3 {
            oracle.next_shared_retail_random_u16();
        }
        let steps = deliver_entity_impact(
            &mut session.cache,
            &mut manager,
            &mut fx,
            &mut StaticDamageScheduler::new(),
            &mut scheduler,
            &mut GameplayNotifications::new(),
            hit,
            146,
        );
        assert!(
            matches!(steps.as_slice(), [Intro2EntityImpactStep::MeteorInfectedDamage(
            crate::intro2_meteors::impact::Intro2MeteorInfectedHitOutcome::Applied { damage, .. }
        )] if damage.filtered_damage_raw == 0),
            "{steps:?}"
        );
        assert_eq!(scheduler.intro2_meteor_owner(id), Some(owner));
        assert_eq!(manager.entity_mut(id).unwrap().model_slots, [Some(560); 4]);
        assert_eq!(fx.pending_event_count(), 0);
        assert_eq!(fx.particle_count(), 0);
        assert!(fx.take_positional_sounds().is_empty());
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            oracle.next_shared_retail_random_u16()
        );
    }
    #[v2k_test_support::retail_test]
    fn intro2_particle_suppression_preserves_live_target_and_rng() {
        let Some((mut session, mut manager, id)) = native_intro2_fixture() else {
            return;
        };
        let before = manager.entity_mut(id).unwrap().collision.clone();
        let mut fx = WorldFx::new();
        let mut control = WorldFx::new();
        let mut hit = impact(id);
        hit.damage = None;
        let steps = deliver_entity_impact(
            &mut session.cache,
            &mut manager,
            &mut fx,
            &mut StaticDamageScheduler::new(),
            &mut SpecializedActorTaskScheduler::new(),
            &mut GameplayNotifications::new(),
            hit,
            25,
        );
        assert!(matches!(
            steps.as_slice(),
            [Intro2EntityImpactStep::Suppressed]
        ));
        assert_eq!(manager.entity_mut(id).unwrap().collision, before);
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            control.next_shared_retail_random_u16()
        );
    }
    #[v2k_test_support::retail_test]
    fn intro2_particle_type47_lethal_hit_registers_class12_in_place() {
        let Some((mut session, mut manager, id)) = native_intro2_fixture() else {
            return;
        };
        let mut scheduler = SpecializedActorTaskScheduler::new();
        assert_eq!(scheduler.adopt_intro2_type47_guards(&manager), 3);
        manager.entity_mut(id).unwrap().collision.health_raw = RetailRuntimeValue::Known(1);
        let steps = deliver_entity_impact(
            &mut session.cache,
            &mut manager,
            &mut WorldFx::new(),
            &mut StaticDamageScheduler::new(),
            &mut scheduler,
            &mut GameplayNotifications::new(),
            impact(id),
            25,
        );
        assert!(
            steps.iter().any(|step| matches!(
                step,
                Intro2EntityImpactStep::Type47Damage(Type47CheckedDamageOutcome::Lethal {
                    publication: Some(_),
                    ..
                })
            )),
            "{steps:?}"
        );
        assert!(
            !steps
                .iter()
                .any(|step| matches!(step, Intro2EntityImpactStep::DeathRegistrationRejected(_))),
            "{steps:?}"
        );
        assert_eq!(
            scheduler.registered_len(),
            3,
            "class12 replaces the same allocation's live owner"
        );
        assert!(matches!(
            manager
                .entity_mut(id)
                .unwrap()
                .actor_task_state(ActorTaskSlot::Primary),
            Some(ActorTaskRuntime::CommonDying(_))
        ));
    }
    #[v2k_test_support::retail_test]
    fn intro2_particle_lookalike_without_native_receipt_blocks_before_stamp_and_rng() {
        let Some((mut session, mut manager, id)) = native_intro2_fixture() else {
            return;
        };
        let entity = manager.entity_mut(id).unwrap();
        entity.intro2_type47_sub_d_frame_owner = None;
        let before = entity.collision.clone();
        let mut fx = WorldFx::new();
        let mut control = WorldFx::new();
        let steps = deliver_entity_impact(
            &mut session.cache,
            &mut manager,
            &mut fx,
            &mut StaticDamageScheduler::new(),
            &mut SpecializedActorTaskScheduler::new(),
            &mut GameplayNotifications::new(),
            impact(id),
            25,
        );
        assert!(
            matches!(
                steps.as_slice(),
                [Intro2EntityImpactStep::Type47Reselection(Err(
                    Type47ImpactLiveError::AllocationCohortUnavailable
                ))]
            ),
            "{steps:?}"
        );
        assert_eq!(manager.entity_mut(id).unwrap().collision, before);
        assert_eq!(
            fx.next_shared_retail_random_u16(),
            control.next_shared_retail_random_u16()
        );
    }

    #[v2k_test_support::retail_test]
    fn dragon_projectile_reaches_native_factory_damage_from_live_model_sweep() {
        let Some((mut session, mut manager, _)) = native_intro2_fixture() else {
            return;
        };
        let id = manager
            .iter_all()
            .find(|entity| entity.authored_spawn_index == Some(51))
            .unwrap()
            .id;
        manager.set_authored_behavior_components_enabled(51, true);
        let factory = manager.entity_mut(id).unwrap();
        // Isolate the Type66 filter against constructor/repaired health.
        // Post-load current-health 1 is a separate native write.
        factory.collision.health_raw = RetailRuntimeValue::Known(99_999);
        factory.set_motion_raw([0, 10_000, 0], [0; 3]);
        let mut fx = WorldFx::new();
        fx.materialize_descriptor_particle_request(
            DescriptorParticleRequest {
                source_class: 38,
                position_raw: [0, 10_000, -300],
                velocity_raw: [0, 0, 16_000],
                owner: Some(ParticleOwnerAtBirth {
                    entity_id: 7,
                    entity_type: 10,
                }),
                suppresses_impact_damage: false,
            },
            ParticleEnvironment::Dry,
            0,
        )
        .unwrap();
        let mut scheduler = SpecializedActorTaskScheduler::new();
        scheduler.adopt_intro2_type66(&manager);
        let report = update_intro2_effects(Intro2EffectsFrame {
            cache: &mut session.cache,
            entities: &mut manager,
            world_fx: &mut fx,
            static_damage: &mut StaticDamageScheduler::new(),
            scheduler: &mut scheduler,
            notifications: &mut GameplayNotifications::new(),
            elapsed_micros: 20_000,
            retail_tick: 1,
        });
        let delivery = report
            .entity_deliveries
            .iter()
            .find(|delivery| delivery.impact.target_entity_id == id)
            .expect("class38 must hit the original factory model");
        assert_eq!(
            delivery.impact.damage_delivery_record().unwrap().packet,
            crate::damage::DRAGON_FIREBALL_DAMAGE_PACKET
        );
        assert_eq!(
            manager.entity_mut(id).unwrap().collision.health_raw,
            RetailRuntimeValue::Known(87_699),
            "retail's first factory hit removes 12300 health"
        );
        assert!(report.particles.unhandled_ground_programs.is_empty());
    }

    #[v2k_test_support::retail_test]
    fn intro2_live_particle_traversal_reaches_authored_type47_collision() {
        let Some((mut session, mut manager, id)) = native_intro2_fixture() else {
            return;
        };
        // Isolate the original model in empty air; retain its native identity,
        // model and physical matrix while the real narrow phase owns the hit.
        let target = manager.entity_mut(id).unwrap();
        target.set_motion_raw([0, 10_000, 0], [0; 3]);
        let mut fx = WorldFx::new();
        fx.materialize_descriptor_particle_request(
            DescriptorParticleRequest {
                source_class: 1,
                position_raw: [0, 10_000, -300],
                velocity_raw: [0, 0, 16_000],
                owner: Some(ParticleOwnerAtBirth {
                    entity_id: 1,
                    entity_type: 13,
                }),
                suppresses_impact_damage: false,
            },
            ParticleEnvironment::Dry,
            0,
        )
        .unwrap();
        let report = update_intro2_effects(Intro2EffectsFrame {
            cache: &mut session.cache,
            entities: &mut manager,
            world_fx: &mut fx,
            static_damage: &mut StaticDamageScheduler::new(),
            scheduler: &mut SpecializedActorTaskScheduler::new(),
            notifications: &mut GameplayNotifications::new(),
            elapsed_micros: 20_000,
            retail_tick: 1,
        });
        assert!(
            report
                .entity_deliveries
                .iter()
                .any(|delivery| delivery.impact.target_entity_id == id),
            "{:?}",
            report.particles
        );
    }
}
