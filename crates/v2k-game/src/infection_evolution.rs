//! Natural hive-owned terrain infection evolution.
//!
//! This is the literal terrain-state part of retail `FUN_0041BEB0`,
//! `FUN_00436960`, `FUN_00436BA0`, and `FUN_00436750`. It is deliberately
//! separate from class-5 impact infection (`FUN_0043E180`) and from the
//! player-facing "Cleansing Landscape" selector whose input ownership is not
//! yet recovered.
//!
//! The accepted Level-1 capture
//! `20260722-022643-hive-virus.jsonl` proves an independent evolution path
//! because it contains three clears while class-5 ground contact can only set
//! infection. Ten additional sets lack a same-window class-5 correlation, but
//! the sampled particle pool is non-atomic and does not identify their source.
//! The decompiled callbacks close the exact interval, neighborhood, wrapping,
//! random-walk, queued-tail, and positional-sound contracts implemented here.
//! Callers must supply the process-wide shared RNG stream.

use std::collections::VecDeque;

use v2k_formats::terrain::{TerrainGrid, GRID_SIZE};

/// Terrain-type bit owned by the infection callbacks.
pub const INFECTION_TERRAIN_TYPE_BIT: u8 = 0x10;
/// Delay installed by `FUN_00436BA0` for every queued propagation step.
pub const INFECTION_TAIL_STEP_US: i32 = 350_000;
/// Global positional sound selected by every queued propagation step.
pub const INFECTION_TAIL_SOUND_ID: u16 = 0x40;

/// One exact set/clear of Section-10 terrain-type bit `0x10`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InfectionCellWrite {
    pub cell: [u8; 2],
    pub infected: bool,
}

/// One fixed-rate, full-gain positional sound requested immediately before a
/// queued cell is infected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InfectionTailSound {
    pub global_sound_id: u16,
    pub position_raw: [i16; 3],
}

/// Shared side effects consumed synchronously by the evolution callbacks.
///
/// Retail uses the same process-global RNG stream as the component's radial
/// particle emitter. Keeping both operations on one adapter prevents a caller
/// from accidentally reordering independent closures around the sound/write
/// boundary.
pub trait InfectionEvolutionEffects {
    fn next_shared_random_u16(&mut self) -> u16;
    fn queue_infection_tail_sound(&mut self, sound: InfectionTailSound);
}

/// Per-frame terrain view shared by every authored infection controller.
///
/// Writes immediately change this view, so later crossings/controllers in the
/// same retail list order observe earlier mutations. The owning cache can
/// commit [`writes`](Self::writes) after all callbacks release their immutable
/// terrain borrow.
#[derive(Debug, Clone)]
pub struct InfectionTerrainSnapshot {
    heights: Vec<u8>,
    infected: Vec<bool>,
    sea_level_raw: i16,
    writes: Vec<InfectionCellWrite>,
}

impl InfectionTerrainSnapshot {
    pub fn from_terrain(terrain: &TerrainGrid) -> Self {
        Self {
            heights: terrain.cells.iter().map(|cell| cell.height).collect(),
            infected: terrain
                .cells
                .iter()
                .map(|cell| cell.terrain_type & INFECTION_TERRAIN_TYPE_BIT != 0)
                .collect(),
            sea_level_raw: terrain.sea_level_raw(),
            writes: Vec::new(),
        }
    }

    pub fn writes(&self) -> &[InfectionCellWrite] {
        &self.writes
    }

    pub fn into_writes(self) -> Vec<InfectionCellWrite> {
        self.writes
    }

    pub fn is_infected(&self, cell: [u8; 2]) -> bool {
        self.infected[cell_index(cell)]
    }

    fn set_infected(&mut self, cell: [u8; 2], infected: bool) {
        let index = cell_index(cell);
        if self.infected[index] == infected {
            return;
        }
        self.infected[index] = infected;
        self.writes.push(InfectionCellWrite { cell, infected });
    }

    fn height_raw(&self, cell: [u8; 2]) -> i16 {
        i16::from(self.heights[cell_index(cell)] as i8) * 0x20
    }

    fn below_sea_level(&self, cell: [u8; 2]) -> bool {
        self.height_raw(cell) < self.sea_level_raw
    }

    fn infected_neighborhood_count(&self, center: [u8; 2]) -> u8 {
        let mut count = 0;
        for dx in [u8::MAX, 0, 1] {
            for dz in [u8::MAX, 0, 1] {
                let cell = [center[0].wrapping_add(dx), center[1].wrapping_add(dz)];
                count += u8::from(self.is_infected(cell));
            }
        }
        count
    }
}

fn cell_index(cell: [u8; 2]) -> usize {
    usize::from(cell[0]) * GRID_SIZE + usize::from(cell[1])
}

/// Census matching live `DAT_004dc684`: terrain-type bit `0x10` over the
/// current 256×256 Section-10 grid. Overlay 51 reads this through
/// [`crate::world_complete_results::landscape_virus_percent`].
pub fn infected_cell_count(terrain: &TerrainGrid) -> u32 {
    terrain
        .cells
        .iter()
        .filter(|cell| cell.terrain_type & INFECTION_TERRAIN_TYPE_BIT != 0)
        .count() as u32
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct InfectionTail {
    timer_us: i32,
    cell: [u8; 2],
    direction: u8,
    remaining_steps: i8,
}

/// Retained mutable words/list owned by one `FUN_0041BEB0` component.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HiveInfectionEvolution {
    accumulator_us: i32,
    tails: VecDeque<InfectionTail>,
}

impl HiveInfectionEvolution {
    pub const fn accumulator_us(&self) -> i32 {
        self.accumulator_us
    }

    pub fn pending_tail_count(&self) -> usize {
        self.tails.len()
    }

    /// Advance one component callback.
    ///
    /// `animation_words[1..=3]` are respectively the natural-evolution
    /// interval, maximum infected random-walk run, and maximum queued tail
    /// length. Equality at both interval boundaries is intentionally not a
    /// crossing. Queued tails advance only when the natural interval crosses,
    /// exactly where `FUN_0041BEB0` calls `FUN_00436750`.
    pub fn advance(
        &mut self,
        elapsed_us: i32,
        objective_hostile_present: bool,
        natural_spread_authority: bool,
        controller_state: u32,
        animation_words: [u32; 6],
        terrain: &mut InfectionTerrainSnapshot,
        effects: &mut impl InfectionEvolutionEffects,
    ) {
        let interval_us = animation_words[1] as i32;
        if !objective_hostile_present || interval_us <= 0 {
            return;
        }

        let total = self.accumulator_us.wrapping_add(elapsed_us);
        self.accumulator_us = total;
        if interval_us >= total {
            return;
        }

        let crossings = total / interval_us;
        self.accumulator_us = total - crossings * interval_us;
        evolve_natural_cells(
            crossings,
            animation_words[2] as i32,
            animation_words[3] as i32,
            natural_spread_authority,
            controller_state == 1,
            &mut self.tails,
            terrain,
            effects,
        );
        advance_tails(&mut self.tails, elapsed_us, terrain, effects);
    }
}

fn evolve_natural_cells(
    crossings: i32,
    maximum_infected_run: i32,
    maximum_tail_steps: i32,
    natural_spread_authority: bool,
    dense_spread_enabled: bool,
    tails: &mut VecDeque<InfectionTail>,
    terrain: &mut InfectionTerrainSnapshot,
    effects: &mut impl InfectionEvolutionEffects,
) {
    if !natural_spread_authority {
        // Multiplayer clients skip FUN_00436960 entirely, including its two
        // selector draws, but FUN_00436750 still advances existing tails.
        return;
    }
    for _ in 0..crossings {
        let selected = [
            effects.next_shared_random_u16() as u8,
            effects.next_shared_random_u16() as u8,
        ];
        if !terrain.is_infected(selected) {
            continue;
        }

        let neighbors = terrain.infected_neighborhood_count(selected);
        if neighbors < 6 {
            if neighbors < 5 {
                terrain.set_infected(selected, false);
            }
            continue;
        }
        if !dense_spread_enabled {
            continue;
        }
        let direction_word = effects.next_shared_random_u16();
        let direction = direction_word as u8 & 3;
        let mut cell = selected;
        let mut infected_run = 0i32;
        loop {
            let step_word = effects.next_shared_random_u16();
            step_random_walk(&mut cell, direction_word, direction, step_word);
            if !terrain.is_infected(cell) {
                terrain.set_infected(cell, true);
                let mut tail_steps = if terrain.below_sea_level(cell) {
                    0
                } else {
                    infected_run / 2
                };
                if maximum_tail_steps < tail_steps {
                    tail_steps = maximum_tail_steps;
                }
                if tail_steps != 0 {
                    tails.push_back(InfectionTail {
                        timer_us: INFECTION_TAIL_STEP_US,
                        cell,
                        direction,
                        remaining_steps: tail_steps as i8,
                    });
                }
                break;
            }

            infected_run += 1;
            if maximum_infected_run < infected_run {
                break;
            }
        }
    }
}

fn step_random_walk(cell: &mut [u8; 2], direction_word: u16, direction: u8, step_word: u16) {
    if step_word < 0x8001 {
        let delta = signed_unit(step_word & 1 != 0);
        if direction_word & 1 == 0 {
            cell[0] = cell[0].wrapping_add_signed(delta);
        } else {
            cell[1] = cell[1].wrapping_add_signed(delta);
        }
    } else {
        let delta = signed_unit(direction & 2 != 0);
        if direction_word & 1 == 0 {
            cell[1] = cell[1].wrapping_add_signed(delta);
        } else {
            cell[0] = cell[0].wrapping_add_signed(delta);
        }
    }
}

fn signed_unit(positive: bool) -> i8 {
    if positive {
        1
    } else {
        -1
    }
}

fn advance_tails(
    tails: &mut VecDeque<InfectionTail>,
    elapsed_us: i32,
    terrain: &mut InfectionTerrainSnapshot,
    effects: &mut impl InfectionEvolutionEffects,
) {
    let mut index = 0;
    while index < tails.len() {
        let mut tail = tails[index];
        tail.timer_us = tail.timer_us.wrapping_sub(elapsed_us);
        while tail.timer_us < 0 && tail.remaining_steps > 0 {
            tail.timer_us = tail.timer_us.wrapping_add(INFECTION_TAIL_STEP_US);
            if !terrain.is_infected(tail.cell) {
                tail.remaining_steps = 0;
                break;
            }

            let step_word = effects.next_shared_random_u16();
            step_random_walk(
                &mut tail.cell,
                u16::from(tail.direction),
                tail.direction,
                step_word,
            );
            effects.queue_infection_tail_sound(InfectionTailSound {
                global_sound_id: INFECTION_TAIL_SOUND_ID,
                position_raw: [
                    i16::from(tail.cell[0]) << 8,
                    terrain.height_raw(tail.cell),
                    i16::from(tail.cell[1]) << 8,
                ],
            });
            terrain.set_infected(tail.cell, true);
            if terrain.below_sea_level(tail.cell) {
                tail.remaining_steps = 0;
            } else {
                tail.remaining_steps = tail.remaining_steps.wrapping_sub(1);
            }
        }

        if tail.remaining_steps == 0 {
            tails.remove(index);
        } else {
            tails[index] = tail;
            index += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use v2k_formats::terrain::TerrainCell;

    const WORDS: [u32; 6] = [50_000, 5_000, 5, 5, 0, 0];

    fn terrain(height: i8, sea_level_raw: i16) -> TerrainGrid {
        TerrainGrid {
            header: [i32::from(sea_level_raw) << 8, 0, 0, 0, 0],
            cells: vec![
                TerrainCell {
                    height: height as u8,
                    attribute: 0,
                    terrain_type: 0,
                };
                GRID_SIZE * GRID_SIZE
            ],
        }
    }

    fn infect(grid: &mut TerrainGrid, cell: [u8; 2]) {
        grid.cells[cell_index(cell)].terrain_type |= INFECTION_TERRAIN_TYPE_BIT;
    }

    #[test]
    fn infected_cell_count_reads_only_terrain_type_bit_0x10() {
        let mut grid = terrain(0, -0x1000);
        grid.cells[cell_index([1, 2])].terrain_type = 0x08;
        infect(&mut grid, [3, 4]);
        infect(&mut grid, [3, 4]);
        infect(&mut grid, [5, 6]);
        assert_eq!(infected_cell_count(&grid), 2);
    }

    struct ScriptedEffects<'a> {
        words: &'a [u16],
        calls: std::rc::Rc<std::cell::Cell<usize>>,
        sounds: Vec<InfectionTailSound>,
    }

    impl InfectionEvolutionEffects for ScriptedEffects<'_> {
        fn next_shared_random_u16(&mut self) -> u16 {
            let index = self.calls.get();
            self.calls.set(index + 1);
            self.words[index]
        }

        fn queue_infection_tail_sound(&mut self, sound: InfectionTailSound) {
            self.sounds.push(sound);
        }
    }

    fn scripted_effects(
        words: &[u16],
    ) -> (ScriptedEffects<'_>, std::rc::Rc<std::cell::Cell<usize>>) {
        let calls = std::rc::Rc::new(std::cell::Cell::new(0));
        (
            ScriptedEffects {
                words,
                calls: calls.clone(),
                sounds: Vec::new(),
            },
            calls,
        )
    }

    #[test]
    fn strict_interval_preserves_accumulator_while_objective_gate_is_closed() {
        let grid = terrain(0, -0x1000);
        let mut view = InfectionTerrainSnapshot::from_terrain(&grid);
        let mut runtime = HiveInfectionEvolution::default();

        runtime.advance(
            5_000,
            true,
            true,
            1,
            WORDS,
            &mut view,
            &mut scripted_effects(&[]).0,
        );
        assert_eq!(runtime.accumulator_us(), 5_000);
        assert!(view.writes().is_empty());

        runtime.advance(
            99_000,
            false,
            true,
            1,
            WORDS,
            &mut view,
            &mut scripted_effects(&[]).0,
        );
        assert_eq!(
            runtime.accumulator_us(),
            5_000,
            "FUN_0041BEB0 does not reset time while the objective predicate is false"
        );

        let (mut effects, calls) = scripted_effects(&[90, 80, 91, 81]);
        runtime.advance(5_001, true, true, 1, WORDS, &mut view, &mut effects);
        assert_eq!(runtime.accumulator_us(), 1);
        assert_eq!(calls.get(), 4, "two strict catch-up crossings");
    }

    #[test]
    fn sparse_four_clears_five_holds_and_neighborhood_wraps() {
        let center = [0, 0];
        let mut sparse = terrain(0, -0x1000);
        for cell in [center, [u8::MAX, 0], [0, u8::MAX], [1, 0]] {
            infect(&mut sparse, cell);
        }
        let mut sparse_view = InfectionTerrainSnapshot::from_terrain(&sparse);
        let mut runtime = HiveInfectionEvolution::default();
        runtime.advance(
            5_001,
            true,
            true,
            1,
            WORDS,
            &mut sparse_view,
            &mut scripted_effects(&[0, 0]).0,
        );
        assert_eq!(
            sparse_view.writes(),
            &[InfectionCellWrite {
                cell: center,
                infected: false
            }]
        );

        let mut five = sparse;
        infect(&mut five, [0, 1]);
        let mut five_view = InfectionTerrainSnapshot::from_terrain(&five);
        let mut runtime = HiveInfectionEvolution::default();
        runtime.advance(
            5_001,
            true,
            true,
            1,
            WORDS,
            &mut five_view,
            &mut scripted_effects(&[0, 0]).0,
        );
        assert!(five_view.writes().is_empty());
    }

    #[test]
    fn dense_walk_sets_first_empty_cell_and_enqueues_half_the_run() {
        let mut grid = terrain(0, -0x1000);
        for x in 9..=11 {
            for z in 9..=11 {
                infect(&mut grid, [x, z]);
            }
        }
        infect(&mut grid, [12, 10]);
        let words = [10, 10, 0, 1, 1, 1];
        let (mut effects, calls) = scripted_effects(&words);
        let mut view = InfectionTerrainSnapshot::from_terrain(&grid);
        let mut runtime = HiveInfectionEvolution::default();
        runtime.advance(5_001, true, true, 1, WORDS, &mut view, &mut effects);

        assert_eq!(calls.get(), words.len());
        assert_eq!(
            view.writes(),
            &[InfectionCellWrite {
                cell: [13, 10],
                infected: true
            }]
        );
        assert_eq!(runtime.pending_tail_count(), 1);
    }

    #[test]
    fn dense_walk_honors_authored_run_limit_and_controller_state() {
        let mut grid = terrain(0, -0x1000);
        for x in 9..=20 {
            for z in 9..=11 {
                infect(&mut grid, [x, z]);
            }
        }
        let mut view = InfectionTerrainSnapshot::from_terrain(&grid);
        let mut runtime = HiveInfectionEvolution::default();
        let (mut effects, calls) = scripted_effects(&[10, 10, 0, 1, 1, 1, 1, 1, 1]);
        runtime.advance(5_001, true, true, 1, WORDS, &mut view, &mut effects);
        assert_eq!(
            calls.get(),
            9,
            "two selectors, direction, then six scanned cells"
        );
        assert!(view.writes().is_empty());

        let mut disabled_view = InfectionTerrainSnapshot::from_terrain(&grid);
        let mut runtime = HiveInfectionEvolution::default();
        let (mut effects, calls) = scripted_effects(&[10, 10]);
        runtime.advance(
            5_001,
            true,
            true,
            2,
            WORDS,
            &mut disabled_view,
            &mut effects,
        );
        assert_eq!(
            calls.get(),
            2,
            "non-state-1 dense branch consumes no direction RNG"
        );
    }

    #[test]
    fn queued_tail_uses_strict_timer_and_emits_before_setting() {
        let mut grid = terrain(2, -0x1000);
        for x in 9..=12 {
            for z in 9..=11 {
                infect(&mut grid, [x, z]);
            }
        }
        let mut view = InfectionTerrainSnapshot::from_terrain(&grid);
        let mut runtime = HiveInfectionEvolution::default();
        runtime.advance(
            5_001,
            true,
            true,
            1,
            WORDS,
            &mut view,
            &mut scripted_effects(&[10, 10, 0, 1, 1, 1, 1]).0,
        );
        assert_eq!(runtime.pending_tail_count(), 1);

        let remaining_delay_us = runtime.tails[0].timer_us;
        let mut effects = scripted_effects(&[]).0;
        advance_tails(
            &mut runtime.tails,
            remaining_delay_us,
            &mut view,
            &mut effects,
        );
        assert!(effects.sounds.is_empty());
        let mut effects = scripted_effects(&[1]).0;
        advance_tails(&mut runtime.tails, 1, &mut view, &mut effects);
        assert_eq!(effects.sounds.len(), 1);
        assert_eq!(
            effects.sounds[0],
            InfectionTailSound {
                global_sound_id: INFECTION_TAIL_SOUND_ID,
                position_raw: [14 << 8, 2 * 0x20, 10 << 8],
            }
        );
        assert!(view.is_infected([14, 10]));
        assert_eq!(runtime.pending_tail_count(), 0);
    }

    #[test]
    fn queued_tail_stops_without_rng_when_its_anchor_was_cleared() {
        let grid = terrain(0, -0x1000);
        let mut view = InfectionTerrainSnapshot::from_terrain(&grid);
        let mut runtime = HiveInfectionEvolution {
            accumulator_us: 0,
            tails: VecDeque::from([InfectionTail {
                timer_us: 1,
                cell: [40, 50],
                direction: 0,
                remaining_steps: 2,
            }]),
        };
        let (mut effects, calls) = scripted_effects(&[200, 200]);
        runtime.advance(5_001, true, true, 1, WORDS, &mut view, &mut effects);
        assert_eq!(calls.get(), 2, "only the natural selector draws");
        assert_eq!(runtime.pending_tail_count(), 0);
    }

    #[test]
    fn queued_tail_catches_up_in_order_and_consumes_one_word_per_step() {
        let mut grid = terrain(3, -0x1000);
        infect(&mut grid, [40, 50]);
        let mut view = InfectionTerrainSnapshot::from_terrain(&grid);
        let mut tails = VecDeque::from([InfectionTail {
            timer_us: 0,
            cell: [40, 50],
            direction: 0,
            remaining_steps: 3,
        }]);
        let (mut effects, calls) = scripted_effects(&[1, 1, 1]);

        advance_tails(&mut tails, 700_001, &mut view, &mut effects);

        assert_eq!(calls.get(), 3);
        assert_eq!(
            effects
                .sounds
                .iter()
                .map(|sound| sound.position_raw)
                .collect::<Vec<_>>(),
            vec![
                [41 << 8, 3 * 0x20, 50 << 8],
                [42 << 8, 3 * 0x20, 50 << 8],
                [43 << 8, 3 * 0x20, 50 << 8],
            ]
        );
        assert!(tails.is_empty());
        assert!(view.is_infected([41, 50]));
        assert!(view.is_infected([42, 50]));
        assert!(view.is_infected([43, 50]));
    }

    #[test]
    fn underwater_new_cell_never_enqueues_a_tail() {
        let mut grid = terrain(-2, 0);
        for x in 9..=12 {
            for z in 9..=11 {
                infect(&mut grid, [x, z]);
            }
        }
        let mut view = InfectionTerrainSnapshot::from_terrain(&grid);
        let mut runtime = HiveInfectionEvolution::default();
        runtime.advance(
            5_001,
            true,
            true,
            1,
            WORDS,
            &mut view,
            &mut scripted_effects(&[10, 10, 0, 1, 1, 1, 1]).0,
        );
        assert!(view.is_infected([13, 10]));
        assert_eq!(runtime.pending_tail_count(), 0);
    }

    #[test]
    fn negative_interval_and_multiplayer_client_authority_fail_closed() {
        let mut grid = terrain(0, -0x1000);
        infect(&mut grid, [0, 0]);
        let mut view = InfectionTerrainSnapshot::from_terrain(&grid);
        let mut runtime = HiveInfectionEvolution::default();
        let mut negative_words = WORDS;
        negative_words[1] = u32::MAX;
        runtime.advance(
            50_000,
            true,
            true,
            1,
            negative_words,
            &mut view,
            &mut scripted_effects(&[]).0,
        );
        assert_eq!(runtime.accumulator_us(), 0);

        runtime.tails.push_back(InfectionTail {
            timer_us: 1,
            cell: [0, 0],
            direction: 0,
            remaining_steps: 1,
        });
        let (mut effects, calls) = scripted_effects(&[1]);
        runtime.advance(5_001, true, false, 1, WORDS, &mut view, &mut effects);
        assert_eq!(
            calls.get(),
            1,
            "a client skips natural selectors but still advances its existing tail"
        );
        assert_eq!(effects.sounds.len(), 1);
    }
}
