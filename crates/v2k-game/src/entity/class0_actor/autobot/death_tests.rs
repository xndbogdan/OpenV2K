//! The Autobot's class11 death: the hold walk's lethal hit, the tumble on
//! the class0 host and its water terminal, against the retail lab.

use super::*;
use crate::damage::{DamageDeliveryRecord, DamagePacket};
use crate::entity::{tick_class0_actor_owner, CampaignCargoRestoreContext, Class0ActorFrame};
use crate::entity_scheduler::{
    COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT, SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT,
};
use crate::intro2_type10::contact::{
    resolve_intro2_type10_tumble_contact_with_playing, Intro2Type10ContactOutcome,
};
use crate::intro2_type10::death::Intro2Type10TumbleContact;

/// Retail lab (Level 1, master code, F9, `wpn4-hold`): the Autobot's
/// position, velocity and Euler words before each visit, from its first
/// tumble visit (tick 1788) to the tick after its water terminal (1847).
const RETAIL_TUMBLE: [([i16; 3], [i16; 3], [i16; 3]); 60] = [
    ([19896, -511, 12316], [1561, 631, 402], [16384, 240, 9]),
    ([19925, -500, 12323], [1561, 602, 402], [16618, 942, 477]),
    ([19954, -490, 12330], [1561, 573, 402], [16852, 1644, 945]),
    ([19983, -480, 12337], [1561, 544, 402], [17086, 2346, 1413]),
    ([20012, -471, 12344], [1561, 515, 402], [17320, 3048, 1881]),
    ([20041, -462, 12351], [1561, 486, 402], [17554, 3750, 2349]),
    ([20070, -454, 12358], [1561, 457, 402], [17788, 4452, 2817]),
    ([20099, -446, 12365], [1561, 428, 402], [18022, 5154, 3285]),
    ([20128, -439, 12372], [1561, 399, 402], [18256, 5856, 3753]),
    ([20157, -432, 12379], [1561, 370, 402], [18490, 6558, 4221]),
    ([20186, -426, 12386], [1561, 341, 402], [18724, 7260, 4689]),
    ([20215, -421, 12393], [1561, 312, 402], [18958, 7962, 5157]),
    ([20244, -416, 12400], [1561, 283, 402], [19192, 8664, 5625]),
    ([20273, -412, 12407], [1561, 254, 402], [19426, 9366, 6093]),
    ([20302, -408, 12414], [1561, 225, 402], [19660, 10068, 6561]),
    ([20331, -405, 12421], [1561, 196, 402], [19894, 10770, 7029]),
    ([20360, -402, 12428], [1561, 167, 402], [20128, 11472, 7497]),
    ([20389, -400, 12435], [1561, 138, 402], [20362, 12174, 7965]),
    ([20418, -398, 12442], [1561, 109, 402], [20596, 12876, 8433]),
    ([20447, -397, 12449], [1561, 80, 402], [20830, 13578, 8901]),
    ([20476, -397, 12456], [1561, 51, 402], [21064, 14280, 9369]),
    ([20505, -397, 12463], [1561, 22, 402], [21298, 14982, 9837]),
    ([20534, -398, 12470], [1561, -7, 402], [21532, 15684, 10305]),
    (
        [20563, -399, 12477],
        [1561, -36, 402],
        [21766, 16386, 10773],
    ),
    (
        [20592, -401, 12484],
        [1561, -65, 402],
        [22000, 17088, 11241],
    ),
    (
        [20621, -403, 12491],
        [1561, -94, 402],
        [22234, 17790, 11709],
    ),
    (
        [20650, -406, 12498],
        [1561, -123, 402],
        [22468, 18492, 12177],
    ),
    (
        [20679, -409, 12505],
        [1561, -152, 402],
        [22702, 19194, 12645],
    ),
    (
        [20708, -413, 12512],
        [1561, -181, 402],
        [22936, 19896, 13113],
    ),
    (
        [20737, -418, 12519],
        [1561, -210, 402],
        [23170, 20598, 13581],
    ),
    (
        [20766, -423, 12526],
        [1561, -239, 402],
        [23404, 21300, 14049],
    ),
    (
        [20795, -429, 12533],
        [1561, -268, 402],
        [23638, 22002, 14517],
    ),
    (
        [20824, -435, 12540],
        [1561, -297, 402],
        [23872, 22704, 14985],
    ),
    (
        [20853, -442, 12547],
        [1561, -326, 402],
        [24106, 23406, 15453],
    ),
    (
        [20882, -449, 12554],
        [1561, -355, 402],
        [24340, 24108, 15921],
    ),
    (
        [20911, -457, 12561],
        [1561, -384, 402],
        [24574, 24810, 16389],
    ),
    (
        [20940, -465, 12568],
        [1561, -413, 402],
        [24808, 25512, 16857],
    ),
    (
        [20969, -474, 12575],
        [1561, -442, 402],
        [25042, 26214, 17325],
    ),
    (
        [20998, -483, 12582],
        [1561, -471, 402],
        [25276, 26916, 17793],
    ),
    (
        [21027, -493, 12589],
        [1561, -500, 402],
        [25510, 27618, 18261],
    ),
    (
        [21056, -504, 12596],
        [1561, -529, 402],
        [25744, 28320, 18729],
    ),
    (
        [21085, -515, 12603],
        [1561, -558, 402],
        [25978, 29022, 19197],
    ),
    (
        [21114, -527, 12610],
        [1561, -587, 402],
        [26212, 29724, 19665],
    ),
    (
        [21143, -539, 12617],
        [1561, -616, 402],
        [26446, 30426, 20133],
    ),
    (
        [21172, -552, 12624],
        [1561, -645, 402],
        [26680, 31128, 20601],
    ),
    (
        [21201, -565, 12631],
        [1561, -674, 402],
        [26914, 31830, 21069],
    ),
    (
        [21230, -579, 12638],
        [1561, -703, 402],
        [27148, 32532, 21537],
    ),
    (
        [21259, -593, 12645],
        [1561, -732, 402],
        [27382, -32302, 22005],
    ),
    (
        [21288, -608, 12652],
        [1561, -761, 402],
        [27616, -31600, 22473],
    ),
    (
        [21317, -624, 12659],
        [1561, -790, 402],
        [27850, -30898, 22941],
    ),
    (
        [21346, -640, 12666],
        [1561, -819, 402],
        [28006, -30430, 23253],
    ),
    (
        [21375, -657, 12673],
        [1561, -848, 402],
        [28162, -29962, 23565],
    ),
    (
        [21404, -674, 12680],
        [1561, -877, 402],
        [28318, -29494, 23877],
    ),
    (
        [21433, -692, 12687],
        [1561, -906, 402],
        [28474, -29026, 24189],
    ),
    (
        [21462, -710, 12694],
        [1561, -935, 402],
        [28630, -28558, 24501],
    ),
    (
        [21491, -729, 12701],
        [1561, -964, 402],
        [28786, -28090, 24813],
    ),
    (
        [21520, -748, 12708],
        [1561, -993, 402],
        [28942, -27622, 25125],
    ),
    (
        [21549, -768, 12715],
        [1561, -1022, 402],
        [29098, -27154, 25437],
    ),
    (
        [21578, -789, 12722],
        [1561, -1051, 402],
        [29254, -26686, 25749],
    ),
    (
        [21607, -810, 12729],
        [1561, -540, 402],
        [29410, -26218, 26061],
    ),
];

struct Fixture {
    session: crate::session::GameSession,
    manager: EntityManager,
    fx: WorldFx,
    tasks: crate::specialized_actor_task_production::SpecializedActorTaskScheduler,
    id: u32,
}

/// Level 1 with the Autobot born into the hold.
fn born() -> Fixture {
    let (session, mut manager, mut fx) =
        crate::native_type122::construction_tests::native_fixture_with_player(13);
    let mut tasks = super::super::queen_tests::adopt_world(&mut manager);
    manager
        .dispatch_controller_operation_0x38(
            ENTITY_TYPE,
            &mut CampaignCargoRestoreContext {
                level: session.cache.level_desc().unwrap(),
                resources: EntityConstructionResources {
                    terrain: session.cache.terrain(),
                    terrain_objects: session.cache.terrain_objects(),
                    model_extent_raw: Some(&|id| session.cache.global_model(id).map(|m| m.radius)),
                },
                retail_tick: 810,
                scheduler: &mut tasks,
                world_fx: &mut fx,
                notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
            },
        )
        .unwrap();
    let id = *manager.auxiliary_owned_entity_ids().last().unwrap();
    Fixture {
        session,
        manager,
        fx,
        tasks,
        id,
    }
}

/// At tick 1787 the walk hands the Autobot the scaled radial packet that
/// reached the craft.
const WALK_PACKET: DamagePacket = DamagePacket {
    channels: [2, 3],
    amounts_raw: [3025, 2593],
};

fn walk_hit(f: &mut Fixture) {
    let player = f.manager.player().unwrap().id;
    let mut notifications = crate::gameplay_notifications::GameplayNotifications::new();
    let nonzero = f
        .manager
        .apply_autobot_walk_damage(
            f.id,
            DamageDeliveryRecord {
                packet: WALK_PACKET,
                source_entity_type_raw: 46,
                owner_handle: player,
            },
            &mut f.fx,
            crate::live_actor_checked_damage::LiveActorDamageFeedback {
                notifications: &mut notifications,
                retail_tick: 1787,
            },
        )
        .unwrap();
    assert!(nonzero);
}

#[v2k_test_support::retail_test]
fn the_walk_kills_the_autobot_into_class11_without_a_random_word() {
    let mut f = born();
    let living = Class0ActorOwner::adopt(&f.manager, f.id).unwrap();
    let profile = f
        .manager
        .type_runtime_metadata(ENTITY_TYPE)
        .unwrap()
        .damage_profile
        .unwrap();
    // Retail: FUN_00415040 on the Autobot returned 7211 (its health is 3000).
    assert_eq!(
        WALK_PACKET.filtered_raw_with_ratio(Some(&profile), 0, 0),
        7211
    );
    let rng = f.fx.shared_retail_random_state();
    walk_hit(&mut f);
    assert_eq!(f.fx.shared_retail_random_state(), rng);
    let entity = f.manager.iter_all().find(|e| e.id == f.id).unwrap();
    assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
    // Dying, and class11's checked damage and terrain contact bits.
    assert_eq!(
        entity.collision.state_flags_at_0x08.masked(0x1_C000),
        RetailRuntimeValue::Known(0x1_C000)
    );
    assert!(matches!(
        entity.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::TumbleOutOfSky(_))
    ));
    assert!(entity.actor_task_state(ActorTaskSlot::Secondary).is_none());
    assert!(entity.actor_task_state(ActorTaskSlot::Tertiary).is_none());
    // The living owner the scheduler kept holds the class11 one.
    let tumbling = Class0ActorOwner::adopt(&f.manager, f.id).unwrap();
    assert!(living.holds(tumbling));
    // A second lethal entry on the dying body is a no-op.
    f.manager.apply_autobot_death(f.id, &mut f.fx).unwrap();
    assert_eq!(Class0ActorOwner::adopt(&f.manager, f.id), Ok(tumbling));
}

/// From retail's pose at its first tumble visit, the class0 host's visits
/// (spin, gravity, master motion) reproduce every tick to 1846, and that
/// tick's contact pass enters the sea: class11's terminal (sixteen class
/// 94/95 particles, the type's radial, the removal staged) and the hard
/// entry's vertical velocity, which retail's tick 1847 shows.
#[v2k_test_support::retail_test]
fn the_tumble_falls_and_spins_as_retail_and_ends_in_the_sea() {
    let mut f = born();
    walk_hit(&mut f);
    let (position, velocity, rotation) = RETAIL_TUMBLE[0];
    let entity = f.manager.entity_mut(f.id).unwrap();
    entity.set_motion_raw(position, velocity);
    entity.set_rotation_heading_pitch_roll_raw(rotation);
    let detailed =
        COMMON_SCHEDULER_CALLBACK_ENABLED_STATE_BIT | SCHEDULER_RANDOM_WAIT_DISABLED_STATE_BIT;
    entity
        .collision
        .state_flags_at_0x08
        .overwrite(detailed, detailed);
    let mut owner = Class0ActorOwner::adopt(&f.manager, f.id).unwrap();
    // The lab's tick 1788 is world clock (`004FED60`) 1436.
    for (tick, row) in (1788..1847).zip(RETAIL_TUMBLE.windows(2)) {
        let next = tick_class0_actor_owner(
            &mut f.manager,
            owner,
            Class0ActorFrame {
                resources: &mut f.session.cache,
                world_fx: &mut f.fx,
                elapsed_micros: 20_000,
                retail_tick: tick - 352,
                generic_death: None,
                terrain_flash: false,
                full_frame_flash: false,
            },
        );
        assert!(
            matches!(
                next.outcome,
                Class0ActorOutcome::Advanced { detailed: true, .. }
            ),
            "tick {tick}: {:?}",
            next.outcome
        );
        owner = next.retained_owner.unwrap();
        let (position, velocity, rotation) = row[1];
        let entity = f.manager.iter_all().find(|e| e.id == f.id).unwrap();
        assert_eq!(entity.position_raw(), position, "tick {tick}");
        assert_eq!(
            entity.rotation_heading_pitch_roll_raw(),
            rotation,
            "tick {tick}"
        );
        // Tick 1846's contact pass halves the fall; the next row shows it.
        if tick < 1846 {
            assert_eq!(entity.velocity_raw(), velocity, "tick {tick}");
        }
    }
    // A 20-ms frame rate, which scales the scatter's count.
    for _ in 0..8 {
        f.fx.advance_frame_pacing(20_000);
    }
    f.fx.process_pending();
    let particles = f.fx.particle_count();
    let mut static_damage = crate::static_damage::StaticDamageScheduler::default();
    let mut notifications = crate::gameplay_notifications::GameplayNotifications::new();
    let mut hull = crate::player_hull::PlayerHull::default();
    let outcome = resolve_intro2_type10_tumble_contact_with_playing(
        &mut crate::intro2_contacts::Intro2ContactFrame {
            entities: &mut f.manager,
            resources: &mut f.session.cache,
            world_fx: &mut f.fx,
            static_damage: &mut static_damage,
            notifications: &mut notifications,
            retail_tick: 1846 - 352,
            actor_tasks: &mut f.tasks,
        },
        f.id,
        Some(crate::native_actor_capture::pair::PlayingPlayerContact {
            hull: &mut hull,
            extra_lives: RetailRuntimeValue::Known(3),
        }),
    );
    let Intro2Type10ContactOutcome::Applied(report) = outcome else {
        panic!("tumble contact: {outcome:?}");
    };
    assert!(!report.terrain_contact);
    assert!(report.water_entry);
    let terminal = report.terminal.unwrap();
    assert_eq!(terminal.contact, Intro2Type10TumbleContact::Water);
    assert!(terminal.finalized);
    f.fx.process_pending();
    assert!(
        f.fx.particle_count() >= particles + 16,
        "{particles} -> {}",
        f.fx.particle_count()
    );
    // Retail's radial at the terminal (`00440BC85`'s caller, BAF0): inner
    // 0x200, outer 0x400, impulse 2000, `{[1, 3], [4000, 4000]}`; the type
    // record's `+50` template.
    let header = &f.session.cache.global_entity_type(64).unwrap().raw_header;
    assert_eq!(
        header[0x50..0x68],
        [
            0x00, 0x02, 0x00, 0x04, 0xd0, 0x07, 0, 0, 1, 0, 0, 0, 3, 0, 0, 0, 0xa0, 0x0f, 0, 0,
            0xa0, 0x0f, 0, 0
        ]
    );
    let entity = f.manager.iter_all().find(|e| e.id == f.id).unwrap();
    assert_eq!(entity.velocity_raw(), RETAIL_TUMBLE[59].1);
    assert_eq!(
        entity
            .collision
            .state_flags_at_0x08
            .masked(crate::entity_collision_state::DEFERRED_DESTROY_PENDING_STATE_BIT),
        RetailRuntimeValue::Known(
            crate::entity_collision_state::DEFERRED_DESTROY_PENDING_STATE_BIT
        )
    );
    assert!(f
        .manager
        .iter_all()
        .any(|e| e.entity_type == 60 && e.authored_spawn_index.is_none()));
    // The emptied Primary drops the class0 owner at its next visit.
    assert!(Class0ActorOwner::adopt(&f.manager, f.id).is_err());
}

/// The player's own hit at tick 1787 walks the hold first: the Autobot takes
/// the packet and dies into class11, then the craft takes its own, 8011 in
/// retail, in full.
#[v2k_test_support::retail_test]
fn the_craft_takes_its_own_hit_after_the_autobot_dies() {
    use crate::entity::{PlayerCheckedDamageFrame, PlayerCheckedDamageRequest, PlayerDamageEntry};
    let mut f = born();
    let player = f.manager.player().unwrap();
    let player_id = player.id;
    let mut hull = crate::player_hull::PlayerHull::default();
    let RetailRuntimeValue::Known(health) = player.collision.health_raw else {
        panic!("player health");
    };
    let RetailRuntimeValue::Known(buffer) = player.collision.pre_health_damage_buffer_raw else {
        panic!("player buffer");
    };
    hull.health_raw = health;
    hull.pre_health_damage_buffer_raw = buffer;
    let outcome = f.manager.apply_player_checked_damage(
        PlayerCheckedDamageRequest {
            entry: PlayerDamageEntry::Checked,
            target_id: player_id,
            delivery: DamageDeliveryRecord {
                packet: WALK_PACKET,
                source_entity_type_raw: 46,
                owner_handle: player_id,
            },
            ratio_numerator: 0,
            ratio_denominator: 0,
        },
        PlayerCheckedDamageFrame {
            hull: &mut hull,
            resources: &f.session.cache,
            world_fx: &mut f.fx,
            retail_tick: 1435,
            notifications: &mut crate::gameplay_notifications::GameplayNotifications::new(),
            extra_lives: 3,
        },
    );
    assert!(
        matches!(
            outcome,
            crate::entity::PlayerCheckedDamageOutcome::Applied {
                filtered_damage_raw: 8011,
                dying: false,
                ..
            }
        ),
        "{outcome:?}"
    );
    assert_eq!(hull.health_raw, health - 8011);
    let entity = f.manager.iter_all().find(|e| e.id == f.id).unwrap();
    assert_eq!(entity.collision.health_raw, RetailRuntimeValue::Known(0));
    assert!(matches!(
        entity.actor_task_state(ActorTaskSlot::Primary),
        Some(ActorTaskRuntime::TumbleOutOfSky(_))
    ));
    // The dying Autobot no longer counts as a live hold body.
    assert_eq!(f.manager.plan_player_hold_walk(), Ok(None));
}
