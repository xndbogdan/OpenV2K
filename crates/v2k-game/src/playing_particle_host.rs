//! Playing's mutable world boundary for the physical particle-slot traversal.
//!
//! Collision borrows end before a synchronous entity callback. Class-49 death
//! may then change terrain and allocations before the next slot borrows them.

use crate::{
    resource_cache::ResourceCache,
    static_damage::{StaticDamageOutcome, StaticDamageScheduler},
    world_fx::{
        CombatGroundProgramRequest, EntityCollisionModel, ParticleCallbackContext,
        ParticleCollisionCacheRefresh, ParticleCollisionContext, ParticleEntityImpact,
        ParticleEnvironment, ParticleOwnerMotion, ParticleStaticImpact, ParticleTerrainEvent,
        ParticleTerrainMutation, ParticleTerrainPublication, ParticleTerrainResponse,
        ParticleTraversalContext, ParticleTraversalHost, TerrainCollisionContext, WorldFx,
    },
};

pub struct PlayingParticleHost<'a, EntityHandler, TerrainHandler> {
    pub cache: &'a mut ResourceCache,
    pub static_damage: &'a mut StaticDamageScheduler,
    pub owner_motions: Vec<ParticleOwnerMotion>,
    pub collision_models: Vec<EntityCollisionModel>,
    /// Full 442950 emission lookup after inline damage/F610. The later
    /// 425D0 mutation uses on_actor_event's fresh live entity custody.
    pub attachment_owners: Vec<crate::world_fx::ParticleAttachmentOwner>,
    pub on_actor_event: EntityHandler,
    pub on_terrain_event: TerrainHandler,
}

/// Both projections follow the completed callback: radial impulses can change
/// inherited motion even when the direct target's collision model is unchanged.
pub struct PlayingParticleRefresh {
    pub collision: ParticleCollisionCacheRefresh,
    pub owner_motions: Vec<ParticleOwnerMotion>,
    pub attachment_owners: Vec<crate::world_fx::ParticleAttachmentOwner>,
}

/// Both particle callback entries need the same mutable EntityManager custody.
/// A single event boundary keeps that manager in one closure and releases the
/// geometry borrow before either the impact or the attached B2 write.
pub enum PlayingParticleActorEvent {
    Impact(ParticleEntityImpact),
    AttachedUpdate(crate::world_fx::AttachedParticleOwnerRequest),
    AttachedDamage(crate::world_fx::AttachedParticleDamageRequest),
    AttachedCascadeOwner(crate::world_fx::AttachedParticleCascadeOwnerRequest),
}

pub enum PlayingParticleActorResponse {
    Impact(PlayingParticleRefresh),
    AttachedUpdate(crate::world_fx::AttachedParticleOwnerLookup),
    AttachedDamage {
        result: Result<i32, crate::attached_particle_damage::AttachedParticleDamageBlock>,
        refresh: PlayingParticleRefresh,
    },
    AttachedCascadeOwner(
        Result<
            crate::world_fx::AttachedParticleCascadeOwner,
            crate::world_fx::AttachedParticleUpdateBlock,
        >,
    ),
}

impl<EntityHandler, TerrainHandler> ParticleTraversalHost
    for PlayingParticleHost<'_, EntityHandler, TerrainHandler>
where
    EntityHandler: FnMut(
        &mut ResourceCache,
        &mut StaticDamageScheduler,
        &mut WorldFx,
        PlayingParticleActorEvent,
    ) -> PlayingParticleActorResponse,
    TerrainHandler: FnMut(
        &mut StaticDamageScheduler,
        &mut WorldFx,
        ParticleTerrainEvent,
    ) -> ParticleTerrainResponse,
{
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
        let PlayingParticleActorResponse::Impact(refresh) = (self.on_actor_event)(
            self.cache,
            self.static_damage,
            world_fx,
            PlayingParticleActorEvent::Impact(impact),
        ) else {
            panic!("Playing impact callback must return its matching response");
        };
        if let ParticleCollisionCacheRefresh::Replace(models) = refresh.collision {
            self.collision_models = models;
        }
        self.owner_motions = refresh.owner_motions;
        self.attachment_owners = refresh.attachment_owners;
    }

    fn attachment_owner(
        &self,
        id: u32,
    ) -> crate::entity_collision_state::RetailRuntimeValue<
        Option<crate::world_fx::ParticleAttachmentOwner>,
    > {
        crate::entity_collision_state::RetailRuntimeValue::Known(
            self.attachment_owners
                .iter()
                .find(|owner| owner.entity_id == id)
                .copied(),
        )
    }

    fn begin_attached_update(
        &mut self,
        world_fx: &mut WorldFx,
        request: crate::world_fx::AttachedParticleOwnerRequest,
    ) -> crate::world_fx::AttachedParticleOwnerLookup {
        let PlayingParticleActorResponse::AttachedUpdate(owner) = (self.on_actor_event)(
            self.cache,
            self.static_damage,
            world_fx,
            PlayingParticleActorEvent::AttachedUpdate(request),
        ) else {
            panic!("Playing attached callback must return its matching response");
        };
        owner
    }
    fn attached_damage(
        &mut self,
        world_fx: &mut WorldFx,
        request: crate::world_fx::AttachedParticleDamageRequest,
    ) -> Result<i32, crate::attached_particle_damage::AttachedParticleDamageBlock> {
        let PlayingParticleActorResponse::AttachedDamage { result, refresh } = (self
            .on_actor_event)(
            self.cache,
            self.static_damage,
            world_fx,
            PlayingParticleActorEvent::AttachedDamage(request),
        ) else {
            panic!("Playing attached damage must return its matching response");
        };
        if let ParticleCollisionCacheRefresh::Replace(models) = refresh.collision {
            self.collision_models = models;
        }
        self.owner_motions = refresh.owner_motions;
        self.attachment_owners = refresh.attachment_owners;
        result
    }
    fn attached_cascade_owner(
        &mut self,
        world_fx: &mut WorldFx,
        request: crate::world_fx::AttachedParticleCascadeOwnerRequest,
    ) -> Result<
        crate::world_fx::AttachedParticleCascadeOwner,
        crate::world_fx::AttachedParticleUpdateBlock,
    > {
        let PlayingParticleActorResponse::AttachedCascadeOwner(owner) = (self.on_actor_event)(
            self.cache,
            self.static_damage,
            world_fx,
            PlayingParticleActorEvent::AttachedCascadeOwner(request),
        ) else {
            panic!("Playing cached attached owner must return its matching response");
        };
        owner
    }

    fn static_impact(
        &mut self,
        world_fx: &mut WorldFx,
        impact: ParticleStaticImpact,
    ) -> Option<ParticleTerrainMutation> {
        match (self.on_terrain_event)(
            self.static_damage,
            world_fx,
            ParticleTerrainEvent::StaticImpact(impact),
        ) {
            ParticleTerrainResponse::StaticDamage(StaticDamageOutcome::ImmediateBurn {
                cell,
                ..
            }) => {
                if let Err(error) = crate::static_terrain_burn::apply_immediate_static_burn(
                    cell, self.cache, world_fx,
                ) {
                    eprintln!("Playing static burn callback blocked: {error:?}");
                }
            }
            ParticleTerrainResponse::StaticDamage(
                StaticDamageOutcome::BurnedKind10Transition { .. },
            ) => unreachable!("the exact F800 packets cannot reach kind-10's immediate mutation"),
            _ => {}
        }
        None
    }

    fn ground_program_request(
        &mut self,
        world_fx: &mut WorldFx,
        request: CombatGroundProgramRequest,
    ) -> bool {
        matches!(
            (self.on_terrain_event)(
                self.static_damage,
                world_fx,
                ParticleTerrainEvent::GroundProgram(request),
            ),
            ParticleTerrainResponse::GroundProgramAccepted
        )
    }
}
