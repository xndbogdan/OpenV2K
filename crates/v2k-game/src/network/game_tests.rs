use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::time::Duration;

use super::*;
use crate::entity::{Entity, EntityKind};
use crate::entity_collision_state::EntityTypeRuntimeMetadata;
use crate::network::scoring::LineTable;
use crate::network::transport::{SearchTarget, SessionSearch};

fn tables() -> LineTables {
    LineTables {
        kill: LineTable(vec!["@2 fragged @1".into()]),
        die: LineTable(vec!["@1 died".into()]),
        self_kill: LineTable(vec!["@1 blew up".into()]),
        names: LineTable(vec![String::new()]),
        host: LineTable(vec!["50".into(), "@1 wins".into()]),
    }
}

fn world(position: [f32; 3]) -> EntityManager {
    let mut metadata = vec![EntityTypeRuntimeMetadata::default(); 60];
    metadata[51].model_slots = [301; 4];
    metadata[51].capability_flags = 5;
    let mut player = Entity::unresolved_port_entity(1, EntityKind::Player, 46);
    player.capability_flags = 5;
    player.position = position;
    player.velocity = [0.5, 0.0, 0.0];
    EntityManager::from_entities_with_type_metadata_for_test(vec![player], metadata, false)
}

/// A host and a guest over loopback, each in world 13 with its player.
struct Pair {
    host: NetworkGame,
    guest: NetworkGame,
    host_world: EntityManager,
    guest_world: EntityManager,
    ticks: u32,
}

impl Pair {
    fn new() -> Self {
        let host = NetworkGame::host("HOSTBOX", "ann", 0, tables()).unwrap();
        let port = host.port().unwrap();
        let mut search = SessionSearch::new(SearchTarget {
            broadcast: false,
            addresses: vec![SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port))],
        })
        .unwrap();
        let mut host = host;
        let info = loop {
            host.poll(Instant::now());
            if let Some(info) = search.refresh(Instant::now()).pop() {
                break info;
            }
            std::thread::sleep(Duration::from_millis(2));
        };
        let joining = std::thread::spawn(move || NetworkGame::join(&info, "bob", tables()));
        while !joining.is_finished() {
            host.poll(Instant::now());
            std::thread::sleep(Duration::from_millis(1));
        }
        let guest = joining.join().unwrap().unwrap();
        Self {
            host,
            guest,
            host_world: world([10.0, 1.0, 10.0]),
            guest_world: world([20.0, 1.0, 20.0]),
            ticks: 0,
        }
    }

    /// One 20 ms frame on both machines, with time for loopback.
    fn frame(&mut self) -> (Vec<NetworkEvent>, Vec<NetworkEvent>) {
        self.ticks += 1;
        let mut out = Vec::new();
        for (game, entities, name) in [
            (&mut self.host, &mut self.host_world, "Ann"),
            (&mut self.guest, &mut self.guest_world, "Bob"),
        ] {
            game.poll(Instant::now());
            game.advance_timer(20_000);
            let mut random = || 0x1234;
            let mut frame = WorldFrame {
                entities,
                world: 13,
                world_active: true,
                elapsed_us: 20_000,
                ticks: self.ticks,
                random: &mut random,
                profile: (name, name),
                player: LocalBodyState {
                    model_variables: vec![1, 2, 3],
                    shield: 0,
                },
            };
            game.world_frame(&mut frame);
            out.push(std::mem::take(&mut game.events));
        }
        std::thread::sleep(Duration::from_millis(3));
        let guest = out.pop().unwrap();
        (out.pop().unwrap(), guest)
    }

    fn run_until(&mut self, mut done: impl FnMut(&Self) -> bool) {
        for _ in 0..400 {
            self.frame();
            if done(self) {
                return;
            }
        }
        panic!(
            "the session never settled: slots {} {} ids {:?} {:?} known {:?} {:?} peers {} {} expected {:?} {:?}",
            self.host.local_slot(),
            self.guest.local_slot(),
            self.host.slots.ids,
            self.guest.slots.ids,
            self.host.scoring.known,
            self.guest.scoring.known,
            peer_crafts(&self.host_world).len(),
            peer_crafts(&self.guest_world).len(),
            self.host.expected,
            self.guest.expected,
        );
    }
}

fn peer_crafts(world: &EntityManager) -> Vec<&Entity> {
    world.iter_all().filter(|e| e.is_network_remote()).collect()
}

/// The guest takes slot 2 from the host's table; each machine then holds
/// the other's craft as a Type51 at its owner's position, with its names.
#[test]
fn two_machines_see_each_others_craft_and_names() {
    let mut pair = Pair::new();
    pair.run_until(|p| p.guest.local_slot() == 2 && p.host.local_slot() == 1);
    pair.run_until(|p| {
        peer_crafts(&p.guest_world).len() == 1
            && peer_crafts(&p.host_world).len() == 1
            && p.host.scoring.known[2]
            && p.guest.scoring.known[1]
    });
    let seen = peer_crafts(&pair.guest_world)[0];
    assert_eq!(seen.entity_type, 51);
    assert!((seen.position[0] - 10.0).abs() < 0.5);
    assert_eq!(seen.network_body().unwrap().model_variables, vec![1, 2, 3]);
    assert_eq!(pair.host.scoring.names[2], "Bob");
    assert_eq!(pair.guest.scoring.names[1], "Ann");
    assert_eq!(pair.host.scoring.frag_limit, 50);
}

/// The guest's hit on the host's craft reaches the host as damage from slot
/// 2; the host's death to that hit names the guest, scores the kill and
/// tells the guest.
#[test]
fn damage_goes_to_the_owner_and_a_kill_is_scored_and_announced() {
    let mut pair = Pair::new();
    pair.run_until(|p| {
        peer_crafts(&p.guest_world).len() == 1
            && p.host.scoring.known[2]
            && p.guest.scoring.known[1]
    });
    let target = peer_crafts(&pair.guest_world)[0].id;
    pair.guest.forward_damage(target, 1800, 46);
    let mut damage = None;
    for _ in 0..100 {
        let (host_events, _) = pair.frame();
        damage = host_events.into_iter().find_map(|event| match event {
            NetworkEvent::Damage {
                entity_id,
                amount,
                attacker_type,
                attacker_slot,
            } => Some((entity_id, amount, attacker_type, attacker_slot)),
            _ => None,
        });
        if damage.is_some() {
            break;
        }
    }
    assert_eq!(damage, Some((1, 1800, 46, 2)));
    pair.host.player_damaged(2, true, 0);
    pair.run_until(|p| p.guest.scoring.table[2].kills == 1);
    assert_eq!(pair.guest.scoring.table[1].deaths, 1);
    assert_eq!(pair.guest.scoring.table[2].score, 20);
    pair.run_until(|p| {
        p.guest
            .scoring
            .lines
            .iter()
            .any(|line| line.1 == "Bob fragged Ann")
    });
}

/// Kills past the frag limit name the winner on both machines.
#[test]
fn the_frag_limit_names_the_winner_everywhere() {
    let mut pair = Pair::new();
    pair.run_until(|p| p.host.scoring.known[2] && p.guest.scoring.known[1]);
    for _ in 0..3 {
        pair.host.player_damaged(2, true, 0);
    }
    let mut flashed = false;
    for _ in 0..200 {
        let (_, guest) = pair.frame();
        flashed |= guest.contains(&NetworkEvent::WinnerFlash);
        if flashed {
            break;
        }
    }
    assert!(flashed);
    assert_eq!(pair.host.scoring.winner, 2);
    assert_eq!(pair.guest.scoring.winner, 2);
}
