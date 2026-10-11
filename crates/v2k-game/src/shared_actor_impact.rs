//! Shared native actor particle entry, independent of scene identity.

pub mod type47;
pub use crate::intro2_type53::impact::{Intro2Type53ImpactBlock, Intro2Type53ImpactOutcome};
pub use crate::intro2_type58::impact::{Intro2Type58ImpactBlock, Intro2Type58ImpactOutcome};
use crate::intro2_type8::NativeWorkerProfile;
pub use crate::native_type122::impact::{Type122ImpactBlock, Type122ImpactOutcome};
pub use crate::native_type123::impact::{NativeType123ImpactBlock, NativeType123ImpactOutcome};
pub use crate::native_type30::impact::{Type30ImpactBlock, Type30ImpactOutcome};
pub use crate::native_type40::impact::{Type40ImpactBlock, Type40ImpactOutcome};
pub use crate::native_type56::impact::{Type56ImpactBlock, Type56ImpactOutcome};
pub use crate::native_type86::impact::{NativeType86ImpactBlock, NativeType86ImpactOutcome};
use crate::native_type86::NativeFourChoiceProfile;
pub use crate::shared_fish::impact::{SharedFishImpactBlock, SharedFishImpactOutcome};

use crate::{
    entity::EntityManager,
    gameplay_notifications::GameplayNotifications,
    resource_cache::ResourceCache,
    specialized_actor_task_production::SpecializedActorTaskScheduler,
    world_fx::{ParticleEntityImpact, WorldFx},
};

pub struct SharedActorImpactFrame<'a> {
    pub resources: &'a ResourceCache,
    pub entities: &'a mut EntityManager,
    pub world_fx: &'a mut WorldFx,
    pub scheduler: &'a mut SpecializedActorTaskScheduler,
    pub notifications: &'a mut GameplayNotifications,
    pub retail_tick: u32,
}

#[derive(Debug)]
pub enum SharedActorImpactOutcome {
    /// 11320 requires the target's exact vtable+1C and current style+24.
    /// A target without a retained native hit owner cannot use generic damage.
    UnsupportedCuredTarget {
        entity_type: u32,
    },
    /// 442950 needs the distinct 11180 wrapper. An unsupported target must
    /// never enter its ordinary 10EB0/11250 adapter by class coincidence.
    UnsupportedStaticRouteTarget {
        entity_type: u32,
    },
    GunTurret(crate::intro2_gun_turret::impact::Intro2GunTurretImpactOutcome),
    NativeWeapon(crate::native_entity_weapons::impact::NativeWeaponImpactOutcome),
    Peasant(crate::ordinary_type9_impact::NativeType9ImpactOutcome),
    MainBase(crate::main_base_runtime::impact::MainBaseImpactOutcome),
    Worker(crate::intro2_type8::impact::Intro2Type8ImpactOutcome),
    Spider(crate::intro2_type17::impact::Intro2Type17ImpactOutcome),
    Insect(type47::NativeType47ImpactOutcome),
    Type53(Intro2Type53ImpactOutcome),
    Type58(Intro2Type58ImpactOutcome),
    Type122(Type122ImpactOutcome),
    Type18(crate::native_type18::impact::Type18ImpactOutcome),
    Type30(Type30ImpactOutcome),
    Type40(Type40ImpactOutcome),
    Type56(crate::native_type56::impact::Type56ImpactOutcome),
    Type123(NativeType123ImpactOutcome),
    Type86(NativeType86ImpactOutcome),
    Fish(SharedFishImpactOutcome),
    Factory(crate::intro2_type66::impact::Intro2Type66ImpactOutcome),
    Type26(crate::intro2_type26_defecate_virus::Intro2Type26ImpactOutcome),
    Flyer(crate::intro2_flyer_impact::NativeFlyerImpactOutcome),
    Type16(crate::intro2_type16::impact::Intro2Type16ImpactOutcome),
    Type94(crate::intro2_type94::impact::Intro2Type94ImpactOutcome),
    Type13(crate::intro2_type13_live::impact::Intro2Type13ImpactOutcome),
    Type10(crate::intro2_type10::impact::Intro2Type10ImpactOutcome),
    Type43(crate::native_type43::impact::Type43ImpactOutcome),
    Type38Family(crate::native_type38::impact::Type38ImpactOutcome),
}

/// Playing's synchronous particle visit also owns terminal blast resources.
/// Other native families borrow the existing read-only actor impact frame;
/// class49 turrets need the actual player hull and mutable static world.
pub struct PlayingActorImpactFrame<'a> {
    pub resources: &'a mut ResourceCache,
    pub entities: &'a mut EntityManager,
    pub world_fx: &'a mut WorldFx,
    pub scheduler: &'a mut SpecializedActorTaskScheduler,
    pub notifications: &'a mut GameplayNotifications,
    pub static_damage: &'a mut crate::static_damage::StaticDamageScheduler,
    pub player_hull: &'a mut crate::player_hull::PlayerHull,
    pub extra_lives: crate::entity_collision_state::RetailRuntimeValue<u8>,
    pub retail_tick: u32,
}

pub fn apply_playing_actor_particle_hit(
    frame: PlayingActorImpactFrame<'_>,
    impact: ParticleEntityImpact,
) -> Option<SharedActorImpactOutcome> {
    let entity = frame
        .entities
        .iter_all()
        .find(|entity| entity.id == impact.target_entity_id)?;
    if entity.native_entity_weapon_runtime.is_some() {
        return Some(SharedActorImpactOutcome::NativeWeapon(
            crate::native_entity_weapons::impact::apply_native_weapon_particle_hit(frame, impact),
        ));
    }
    if crate::world_fx::particle_uses_static_route_entity_hit(impact.source_particle_class)
        && entity.entity_type != 9
        && !(entity.entity_type == 26 && entity.intro2_type26_sub_d_runtime.is_some())
        && !(crate::shared_fish::is_shared_fish_type(entity.entity_type)
            && entity.shared_fish_runtime.is_some())
    {
        return Some(SharedActorImpactOutcome::UnsupportedStaticRouteTarget {
            entity_type: entity.entity_type,
        });
    }
    // Type124's class63 terminal needs BAF0's mutable static world and the
    // lent player; the other fish keep the shared frame below.
    if entity.entity_type == 124 && entity.shared_fish_runtime.is_some() {
        return Some(SharedActorImpactOutcome::Fish(
            crate::shared_fish::impact::apply_playing_type124_particle_hit(frame, impact),
        ));
    }
    // Presence selects the owner before authentication, so a foreign receipt
    // cannot silently fall through to generic damage on the same public type.
    if entity.entity_type == crate::intro2_type13_live::TYPE13_ENTITY_TYPE
        && entity.native_type13_allocation.is_some()
    {
        return Some(SharedActorImpactOutcome::Type13(
            crate::intro2_type13_live::impact::apply_playing_type13_particle_hit(
                crate::intro2_type13_live::impact::Intro2Type13ImpactFrame {
                    entities: frame.entities,
                    resources: frame.resources,
                    world_fx: frame.world_fx,
                    scheduler: frame.scheduler,
                    static_damage: frame.static_damage,
                    notifications: frame.notifications,
                    retail_tick: frame.retail_tick,
                },
                crate::intro2_type13_live::impact::Type13PlayingDeathWorld {
                    player_hull: frame.player_hull,
                    extra_lives: frame.extra_lives,
                },
                impact,
            ),
        ));
    }
    // Type16-family rows: Type128's class63 blast needs the lent Playing world.
    if crate::intro2_type16::Type16Row::from_entity_type(entity.entity_type).is_some()
        && entity.intro2_type16_runtime.is_some()
    {
        return Some(SharedActorImpactOutcome::Type16(
            crate::intro2_type16::impact::apply_playing_type16_family_particle_hit(frame, impact),
        ));
    }
    // Ordinary Type10-family rows: a lethal hit publishes class11 Tumble; its
    // later C750 radial runs in the contact walk with the lent player.
    if crate::intro2_type10::Type10Profile::from_entity_type(entity.entity_type).is_some()
        && entity
            .intro2_type10_runtime
            .is_some_and(|runtime| runtime.ordinary_allocation.is_some())
    {
        return Some(SharedActorImpactOutcome::Type10(
            crate::intro2_type10::impact::apply_playing_type10_family_particle_hit(frame, impact),
        ));
    }
    // Type38/129's lethal class1/class63 blasts need the same Playing world.
    if crate::native_type38::Type38Row::from_entity_type(entity.entity_type).is_some()
        && entity.native_type38_runtime.is_some()
    {
        return Some(SharedActorImpactOutcome::Type38Family(
            crate::native_type38::impact::apply_playing_type38_family_particle_hit(frame, impact),
        ));
    }
    // Type43's lethal class1 blast needs Playing's static world and player.
    if entity.entity_type == crate::native_type43::ENTITY_TYPE
        && entity.native_type43_runtime.is_some()
    {
        return Some(SharedActorImpactOutcome::Type43(
            crate::native_type43::impact::apply_playing_type43_particle_hit(frame, impact),
        ));
    }
    if entity.intro2_gun_turret_runtime.is_some() {
        return Some(SharedActorImpactOutcome::GunTurret(
            crate::intro2_gun_turret::impact::apply_intro2_gun_turret_particle_hit(
                crate::intro2_gun_turret::impact::Intro2GunTurretImpactFrame {
                    entities: frame.entities,
                    resources: frame.resources,
                    static_damage: frame.static_damage,
                    notifications: frame.notifications,
                    world_fx: frame.world_fx,
                    scheduler: frame.scheduler,
                    retail_tick: frame.retail_tick,
                    world: crate::intro2_gun_turret::impact::GunTurretImpactWorld::Playing {
                        player_hull: frame.player_hull,
                        extra_lives: frame.extra_lives,
                    },
                },
                impact,
            ),
        ));
    }
    apply_shared_actor_particle_hit(
        SharedActorImpactFrame {
            resources: frame.resources,
            entities: frame.entities,
            world_fx: frame.world_fx,
            scheduler: frame.scheduler,
            notifications: frame.notifications,
            retail_tick: frame.retail_tick,
        },
        impact,
    )
}

/// A retained native receipt selects the owner; its adapter validates manager
/// custody and every reached damage stage before publishing a replacement task.
pub fn apply_shared_actor_particle_hit(
    frame: SharedActorImpactFrame<'_>,
    impact: ParticleEntityImpact,
) -> Option<SharedActorImpactOutcome> {
    let entity = frame
        .entities
        .iter_all()
        .find(|entity| entity.id == impact.target_entity_id)?;
    if crate::world_fx::particle_uses_static_route_entity_hit(impact.source_particle_class)
        && entity.entity_type != 9
        && !(entity.entity_type == 26 && entity.intro2_type26_sub_d_runtime.is_some())
        && !(crate::shared_fish::is_shared_fish_type(entity.entity_type)
            && entity.shared_fish_runtime.is_some())
    {
        return Some(SharedActorImpactOutcome::UnsupportedStaticRouteTarget {
            entity_type: entity.entity_type,
        });
    }
    if matches!(entity.entity_type, 15 | 87) && entity.intro2_flyer_frame_owner.is_some() {
        return Some(SharedActorImpactOutcome::Flyer(
            crate::intro2_flyer_impact::apply_native_flyer_particle_hit(frame, impact),
        ));
    }
    if crate::intro2_type16::Type16Row::from_entity_type(entity.entity_type).is_some()
        && entity.intro2_type16_runtime.is_some()
    {
        return Some(SharedActorImpactOutcome::Type16(
            crate::intro2_type16::impact::apply_intro2_type16_particle_hit(
                frame.entities,
                frame.resources,
                frame.world_fx,
                frame.scheduler,
                impact,
                frame.retail_tick,
            ),
        ));
    }
    if entity.entity_type == 94 && entity.intro2_type94_runtime.is_some() {
        return Some(SharedActorImpactOutcome::Type94(
            crate::intro2_type94::impact::apply_intro2_type94_particle_hit(
                frame.entities,
                frame.resources,
                frame.world_fx,
                frame.scheduler,
                impact,
                frame.retail_tick,
            ),
        ));
    }
    if entity.entity_type == 26 && entity.intro2_type26_sub_d_runtime.is_some() {
        return Some(SharedActorImpactOutcome::Type26(
            crate::intro2_type26_defecate_virus::apply_intro2_type26_particle_hit(
                frame.entities,
                frame.resources,
                frame.world_fx,
                frame.scheduler,
                impact,
                frame.retail_tick,
            ),
        ));
    }
    if crate::shared_fish::is_shared_fish_type(entity.entity_type)
        && entity.shared_fish_runtime.is_some()
    {
        return Some(SharedActorImpactOutcome::Fish(
            apply_shared_fish_particle_hit(frame, impact),
        ));
    }
    let family = match entity.entity_type {
        6 if entity.main_base_runtime.is_some() => 6,
        type_id
            if NativeWorkerProfile::from_entity_type(type_id).is_some()
                && entity.intro2_type8_runtime.is_some() =>
        {
            8
        }
        9 if entity.ordinary_type9_native_receipt.is_some()
            || entity.intro2_type9_runtime.is_some() =>
        {
            9
        }
        66 if entity.intro2_type66_runtime.is_some() => 66,
        17 if entity.intro2_type17_runtime.is_some() => 17,
        47 if entity.native_type47_construction.is_some() => 47,
        53 if entity.intro2_type53_runtime.is_some() => 53,
        58 if entity.intro2_type58_runtime.is_some() => 58,
        122 if entity.native_type122_runtime.is_some() => 122,
        18 if entity.native_type18_runtime.is_some() => 18,
        30 if entity.native_type30_runtime.is_some() => 30,
        40 if entity.native_type40_runtime.is_some() => 40,
        56 if entity.native_type56_runtime.is_some() => 56,
        123 if entity.native_type123_runtime.is_some() => 123,
        type_id
            if NativeFourChoiceProfile::from_entity_type(type_id).is_some()
                && entity.native_type86_runtime.is_some() =>
        {
            86
        }
        _ => {
            return crate::world_fx::particle_uses_fun_0043f7c0_entity_hit(
                impact.source_particle_class,
            )
            .then_some(SharedActorImpactOutcome::UnsupportedCuredTarget {
                entity_type: entity.entity_type,
            });
        }
    };
    if family == 47 {
        return Some(SharedActorImpactOutcome::Insect(
            apply_shared_type47_particle_hit(frame, impact),
        ));
    }
    if family == 17 {
        return Some(SharedActorImpactOutcome::Spider(
            apply_shared_type17_particle_hit(frame, impact),
        ));
    }
    if family == 30 {
        return Some(SharedActorImpactOutcome::Type30(
            apply_shared_type30_particle_hit(frame, impact),
        ));
    }
    if family == 40 {
        return Some(SharedActorImpactOutcome::Type40(
            apply_shared_type40_particle_hit(frame, impact),
        ));
    }
    if family == 56 {
        return Some(SharedActorImpactOutcome::Type56(
            apply_shared_type56_particle_hit(frame, impact),
        ));
    }
    if family == 53 {
        return Some(SharedActorImpactOutcome::Type53(
            apply_shared_type53_particle_hit(frame, impact),
        ));
    }
    if family == 58 {
        return Some(SharedActorImpactOutcome::Type58(
            apply_shared_type58_particle_hit(frame, impact),
        ));
    }
    if family == 122 {
        return Some(SharedActorImpactOutcome::Type122(
            apply_shared_type122_particle_hit(frame, impact),
        ));
    }
    if family == 18 {
        return Some(SharedActorImpactOutcome::Type18(
            apply_shared_type18_particle_hit(frame, impact),
        ));
    }
    if family == 123 {
        return Some(SharedActorImpactOutcome::Type123(
            crate::native_type123::impact::apply_native_type123_particle_hit(
                frame.entities,
                frame.world_fx,
                frame.scheduler,
                impact,
                frame.retail_tick,
            ),
        ));
    }
    if family == 86 {
        return Some(SharedActorImpactOutcome::Type86(
            crate::native_type86::impact::apply_native_type86_particle_hit(
                frame.entities,
                frame.world_fx,
                frame.scheduler,
                frame.notifications,
                impact,
                frame.retail_tick,
            ),
        ));
    }
    let SharedActorImpactFrame {
        resources: _,
        entities,
        world_fx,
        scheduler,
        notifications,
        retail_tick,
    } = frame;
    match family {
        9 => Some(SharedActorImpactOutcome::Peasant(
            crate::ordinary_type9_impact::apply_native_type9_particle_hit(
                entities,
                world_fx,
                scheduler,
                notifications,
                impact,
                retail_tick,
            ),
        )),
        6 => Some(SharedActorImpactOutcome::MainBase(
            crate::main_base_runtime::impact::apply_main_base_particle_hit(
                entities,
                world_fx,
                scheduler,
                impact,
                retail_tick,
            ),
        )),
        8 => Some(SharedActorImpactOutcome::Worker(
            crate::intro2_type8::impact::apply_intro2_type8_particle_hit(
                entities,
                world_fx,
                scheduler,
                impact,
                retail_tick,
            ),
        )),
        66 => Some(SharedActorImpactOutcome::Factory(
            crate::intro2_type66::impact::apply_intro2_type66_particle_hit(
                entities,
                world_fx,
                scheduler,
                impact,
                retail_tick,
            ),
        )),
        _ => None,
    }
}

#[cfg(test)]
pub(crate) mod type9_tests;

#[cfg(test)]
mod type53_tests;

#[cfg(test)]
mod type58_tests;
#[cfg(test)]
mod type7_tests;

pub(crate) fn apply_shared_type58_particle_hit(
    frame: SharedActorImpactFrame<'_>,
    impact: ParticleEntityImpact,
) -> Intro2Type58ImpactOutcome {
    use crate::intro2_radial::Intro2RadialTaskCustody;
    let id = impact.target_entity_id;
    if !crate::intro2_type58::type58_manager_allocation_authenticates(frame.entities, id)
        || !frame
            .scheduler
            .prepare_native_actor_mutation(frame.entities, id)
    {
        return Intro2Type58ImpactOutcome::Blocked {
            reason: Intro2Type58ImpactBlock::Runtime("completed native allocation/task custody"),
            committed_prefix: false,
        };
    }
    let result = crate::intro2_type58::impact::apply_intro2_type58_particle_hit(
        SharedActorImpactFrame {
            entities: frame.entities,
            resources: frame.resources,
            world_fx: frame.world_fx,
            scheduler: &mut *frame.scheduler,
            notifications: frame.notifications,
            retail_tick: frame.retail_tick,
        },
        impact,
    );
    if matches!(
        result,
        Intro2Type58ImpactOutcome::Blocked {
            committed_prefix: true,
            ..
        }
    ) {
        frame
            .scheduler
            .park_native_contact_prefix(frame.entities, id);
    }
    result
}

pub(crate) fn apply_shared_type18_particle_hit(
    frame: SharedActorImpactFrame<'_>,
    impact: ParticleEntityImpact,
) -> crate::native_type18::impact::Type18ImpactOutcome {
    use crate::intro2_radial::Intro2RadialTaskCustody;
    use crate::native_type18::impact::{Type18ImpactBlock, Type18ImpactOutcome};
    let id = impact.target_entity_id;
    if !crate::native_type18::manager_allocation_authenticates(frame.entities, id)
        || !frame
            .scheduler
            .prepare_native_actor_mutation(frame.entities, id)
    {
        return Type18ImpactOutcome::Blocked {
            reason: Type18ImpactBlock::Runtime("completed native allocation/task custody"),
            committed_prefix: false,
        };
    }
    let result = crate::native_type18::impact::apply_type18_particle_hit(
        SharedActorImpactFrame {
            entities: frame.entities,
            resources: frame.resources,
            world_fx: frame.world_fx,
            scheduler: &mut *frame.scheduler,
            notifications: frame.notifications,
            retail_tick: frame.retail_tick,
        },
        impact,
    );
    if matches!(
        result,
        Type18ImpactOutcome::Blocked {
            committed_prefix: true,
            ..
        }
    ) {
        frame
            .scheduler
            .park_native_contact_prefix(frame.entities, id);
    }
    result
}

pub(crate) fn apply_shared_type122_particle_hit(
    frame: SharedActorImpactFrame<'_>,
    impact: ParticleEntityImpact,
) -> Type122ImpactOutcome {
    use crate::intro2_radial::Intro2RadialTaskCustody;
    let id = impact.target_entity_id;
    if !crate::native_type122::type122_manager_allocation_authenticates(frame.entities, id)
        || !frame
            .scheduler
            .prepare_native_actor_mutation(frame.entities, id)
    {
        return Type122ImpactOutcome::Blocked {
            reason: Type122ImpactBlock::Runtime("completed native allocation/task custody"),
            committed_prefix: false,
        };
    }
    let result = crate::native_type122::impact::apply_type122_particle_hit(
        SharedActorImpactFrame {
            entities: frame.entities,
            resources: frame.resources,
            world_fx: frame.world_fx,
            scheduler: &mut *frame.scheduler,
            notifications: frame.notifications,
            retail_tick: frame.retail_tick,
        },
        impact,
    );
    if matches!(
        result,
        Type122ImpactOutcome::Blocked {
            committed_prefix: true,
            ..
        }
    ) {
        frame
            .scheduler
            .park_native_contact_prefix(frame.entities, id);
    }
    result
}

pub(crate) fn apply_shared_type30_particle_hit(
    frame: SharedActorImpactFrame<'_>,
    impact: ParticleEntityImpact,
) -> Type30ImpactOutcome {
    use crate::intro2_radial::Intro2RadialTaskCustody;
    let id = impact.target_entity_id;
    if !crate::native_type30::manager_allocation_authenticates(frame.entities, id)
        || !frame
            .scheduler
            .prepare_native_actor_mutation(frame.entities, id)
    {
        return Type30ImpactOutcome::Blocked {
            reason: Type30ImpactBlock::Runtime("completed native allocation/task custody"),
            committed_prefix: false,
        };
    }
    let result = crate::native_type30::impact::apply_type30_particle_hit(
        SharedActorImpactFrame {
            entities: frame.entities,
            resources: frame.resources,
            world_fx: frame.world_fx,
            scheduler: &mut *frame.scheduler,
            notifications: frame.notifications,
            retail_tick: frame.retail_tick,
        },
        impact,
    );
    if matches!(
        result,
        Type30ImpactOutcome::Blocked {
            committed_prefix: true,
            ..
        }
    ) {
        frame
            .scheduler
            .park_native_contact_prefix(frame.entities, id);
    }
    result
}

pub(crate) fn apply_shared_type40_particle_hit(
    frame: SharedActorImpactFrame<'_>,
    impact: ParticleEntityImpact,
) -> Type40ImpactOutcome {
    use crate::intro2_radial::Intro2RadialTaskCustody;
    let id = impact.target_entity_id;
    if !crate::native_type40::manager_allocation_authenticates(frame.entities, id)
        || (!crate::native_type40::death::finished_terminal_authenticates(frame.entities, id)
            && !frame
                .scheduler
                .prepare_native_actor_mutation(frame.entities, id))
    {
        return Type40ImpactOutcome::Blocked {
            reason: Type40ImpactBlock::Runtime("completed native allocation/task custody"),
            committed_prefix: false,
        };
    }
    let result = crate::native_type40::impact::apply_type40_particle_hit(
        SharedActorImpactFrame {
            entities: frame.entities,
            resources: frame.resources,
            world_fx: frame.world_fx,
            scheduler: &mut *frame.scheduler,
            notifications: frame.notifications,
            retail_tick: frame.retail_tick,
        },
        impact,
    );
    if matches!(
        result,
        Type40ImpactOutcome::Blocked {
            committed_prefix: true,
            ..
        }
    ) {
        frame
            .scheduler
            .park_native_contact_prefix(frame.entities, id);
    }
    result
}

pub(crate) fn apply_shared_type56_particle_hit(
    frame: SharedActorImpactFrame<'_>,
    impact: ParticleEntityImpact,
) -> Type56ImpactOutcome {
    use crate::intro2_radial::Intro2RadialTaskCustody;
    let id = impact.target_entity_id;
    if !crate::native_type56::manager_allocation_authenticates(frame.entities, id)
        || (!crate::native_type56::death::finished_terminal_authenticates(frame.entities, id)
            && !frame
                .scheduler
                .prepare_native_actor_mutation(frame.entities, id))
    {
        return Type56ImpactOutcome::Blocked {
            reason: Type56ImpactBlock::Runtime("completed native allocation/task custody"),
            committed_prefix: false,
        };
    }
    let result = crate::native_type56::impact::apply_type56_particle_hit(
        SharedActorImpactFrame {
            entities: frame.entities,
            resources: frame.resources,
            world_fx: frame.world_fx,
            scheduler: &mut *frame.scheduler,
            notifications: frame.notifications,
            retail_tick: frame.retail_tick,
        },
        impact,
    );
    if matches!(
        result,
        Type56ImpactOutcome::Blocked {
            committed_prefix: true,
            ..
        }
    ) {
        frame
            .scheduler
            .park_native_contact_prefix(frame.entities, id);
    }
    result
}

pub(crate) fn apply_shared_type53_particle_hit(
    frame: SharedActorImpactFrame<'_>,
    impact: ParticleEntityImpact,
) -> Intro2Type53ImpactOutcome {
    use crate::intro2_radial::Intro2RadialTaskCustody;
    let id = impact.target_entity_id;
    if !frame.entities.iter_all().any(|entity| {
        entity.id == id && entity.entity_type == 53 && entity.intro2_type53_runtime.is_some()
    }) {
        return Intro2Type53ImpactOutcome::NotApplicable;
    }
    if !crate::intro2_type53::type53_manager_allocation_authenticates(frame.entities, id)
        || !frame
            .scheduler
            .prepare_native_actor_mutation(frame.entities, id)
    {
        return Intro2Type53ImpactOutcome::Blocked {
            reason: Intro2Type53ImpactBlock::Runtime("completed native allocation/task custody"),
            committed_prefix: false,
        };
    }
    let result = crate::intro2_type53::impact::apply_intro2_type53_particle_hit(
        SharedActorImpactFrame {
            resources: frame.resources,
            entities: frame.entities,
            world_fx: frame.world_fx,
            scheduler: &mut *frame.scheduler,
            notifications: frame.notifications,
            retail_tick: frame.retail_tick,
        },
        impact,
    );
    if matches!(
        result,
        Intro2Type53ImpactOutcome::Blocked {
            committed_prefix: true,
            ..
        }
    ) {
        frame
            .scheduler
            .park_native_contact_prefix(frame.entities, id);
    }
    result
}

pub(crate) fn apply_shared_type47_particle_hit(
    frame: SharedActorImpactFrame<'_>,
    impact: ParticleEntityImpact,
) -> type47::NativeType47ImpactOutcome {
    use crate::intro2_radial::Intro2RadialTaskCustody;
    use type47::{NativeType47ImpactBlock, NativeType47ImpactOutcome};
    if !frame.entities.iter_all().any(|entity| {
        entity.id == impact.target_entity_id
            && entity.entity_type == 47
            && entity.native_type47_construction.is_some()
    }) {
        return NativeType47ImpactOutcome::NotApplicable;
    }
    if !crate::shared_type47::type47_manager_allocation_authenticates(
        frame.entities,
        impact.target_entity_id,
    ) || !frame
        .scheduler
        .prepare_native_actor_mutation(frame.entities, impact.target_entity_id)
    {
        return NativeType47ImpactOutcome::Blocked {
            reason: NativeType47ImpactBlock::Runtime("completed native allocation/task custody"),
            committed_prefix: false,
        };
    }
    let result = type47::apply_native_type47_particle_hit(
        SharedActorImpactFrame {
            resources: frame.resources,
            entities: frame.entities,
            world_fx: frame.world_fx,
            scheduler: &mut *frame.scheduler,
            notifications: frame.notifications,
            retail_tick: frame.retail_tick,
        },
        impact,
    );
    if matches!(
        result,
        NativeType47ImpactOutcome::Blocked {
            committed_prefix: true,
            ..
        }
    ) {
        frame
            .scheduler
            .park_native_type47_external_prefix(impact.target_entity_id);
    }
    result
}

/// Production task custody is identical in Playing and Intro2. The lower
/// adapter remains available to explicit source-style oracle fixtures.
pub(crate) fn apply_shared_fish_particle_hit(
    frame: SharedActorImpactFrame<'_>,
    impact: ParticleEntityImpact,
) -> SharedFishImpactOutcome {
    let id = impact.target_entity_id;
    if !frame.entities.iter_all().any(|entity| {
        entity.id == id
            && crate::shared_fish::is_shared_fish_type(entity.entity_type)
            && entity.shared_fish_runtime.is_some()
    }) {
        return SharedFishImpactOutcome::NotApplicable;
    }
    if !crate::shared_fish::allocation_authenticates(frame.entities, id)
        || frame.scheduler.shared_fish_has_pending_prefix(id)
    {
        return SharedFishImpactOutcome::Blocked {
            reason: SharedFishImpactBlock::Runtime("completed native allocation/task custody"),
            committed_prefix: false,
        };
    }
    let result = crate::shared_fish::impact::apply_shared_fish_particle_hit(
        SharedActorImpactFrame {
            resources: frame.resources,
            entities: frame.entities,
            world_fx: frame.world_fx,
            scheduler: &mut *frame.scheduler,
            notifications: frame.notifications,
            retail_tick: frame.retail_tick,
        },
        impact,
    );
    if matches!(
        result,
        SharedFishImpactOutcome::Blocked {
            committed_prefix: true,
            ..
        }
    ) {
        frame
            .scheduler
            .park_native_contact_prefix(frame.entities, id);
    }
    result
}

/// Production task custody is identical in Playing and Intro2. The lower
/// adapter remains available to explicit source-style oracle fixtures.
pub(crate) fn apply_shared_type17_particle_hit(
    frame: SharedActorImpactFrame<'_>,
    impact: ParticleEntityImpact,
) -> crate::intro2_type17::impact::Intro2Type17ImpactOutcome {
    use crate::intro2_radial::Intro2RadialTaskCustody;
    use crate::intro2_type17::impact::{Intro2Type17ImpactBlock, Intro2Type17ImpactOutcome};
    if !frame.entities.iter_all().any(|entity| {
        entity.id == impact.target_entity_id
            && entity.entity_type == 17
            && entity.intro2_type17_runtime.is_some()
    }) {
        return Intro2Type17ImpactOutcome::NotApplicable;
    }
    if !frame
        .scheduler
        .prepare_native_actor_mutation(frame.entities, impact.target_entity_id)
    {
        return Intro2Type17ImpactOutcome::Blocked {
            reason: Intro2Type17ImpactBlock::Runtime("completed actor custody"),
            committed_prefix: false,
        };
    }
    let result = crate::intro2_type17::impact::apply_intro2_type17_particle_hit(
        SharedActorImpactFrame {
            resources: frame.resources,
            entities: frame.entities,
            world_fx: frame.world_fx,
            scheduler: &mut *frame.scheduler,
            notifications: frame.notifications,
            retail_tick: frame.retail_tick,
        },
        impact,
    );
    if matches!(
        result,
        Intro2Type17ImpactOutcome::Blocked {
            committed_prefix: true,
            ..
        }
    ) {
        frame
            .scheduler
            .park_intro2_type17_external_prefix(impact.target_entity_id);
    }
    result
}
