//! Hive-completion HUD statistics and the later campaign progress map.
//!
//! `FUN_004567B0` records completion at hive death without changing the mode.
//! In-game `FUN_00452CB0` draws authored Section-2 rows `0xBD..=0xC4` when
//! session `+0x2BC > 500`, with the normal HUD and hive-exit hint still visible.
//! Those rows use `FUN_00452790` field-6 type-on and its shared cue.
//!
//! Only the exit transition selects descriptor `004D0AA0`. Its
//! `FUN_00454390` draws the overlay-51 world map and global string 25 at
//! Section-1 point 78 (Y=95); it does not draw the in-world statistics.
//! Its input table `004C2580` binds Space through `004D0620` to
//! `FUN_00455CC0` (sound 3, session `+0x294=1`). The selected route must
//! survive that screen until continuing loads the destination. The unrelated
//! `004D1090` / `FUN_00456170` / `+0x297` belongs to descriptor `004D0AD8`.

use crate::campaign_transition::CampaignTransition;
use crate::entity_collision_state::{RetailStateWord, DYING_STATE_BIT};
use crate::gameplay_notifications::{
    GameplayNotificationLine, GameplayNotificationPresentation, TextTypewriterCadence,
};

pub use crate::gameplay_notifications::fun_00452270_case_e_hides_line;

/// `FUN_0042E210` mask for controller `+0x1A8` (scientists / capability `0x400`).
pub const FUN_0042E210_CAPABILITY_0X400: u32 = 0x400;
/// `FUN_0042E210` mask for controller `+0x1B0` (peasants / capability `0x800`).
pub const FUN_0042E210_CAPABILITY_0X800: u32 = 0x800;
/// `FUN_0042E210` mask for controller `+0x1A0` (aliens / capability `0x08`).
pub const FUN_0042E210_CAPABILITY_0X08: u32 = 0x08;

/// One `FUN_0042E210` pass over the live list for the overlay-51 buckets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Fun0042e210CapabilityCensus {
    pub cap_0x400: i32,
    pub cap_0x800: i32,
    pub cap_0x08: i32,
}

/// One live-list identity sampled by `FUN_0042E210`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fun0042e210EntitySample {
    pub capability: u32,
    pub state: RetailStateWord,
    /// Component-table Sub-M `+0x68` dword. `None` when table `+0x30` is null.
    pub sub_m_native_count_raw: Option<i32>,
}

/// Retail `FUN_0042E210` live-entity gate: lookup succeeded, `+0x08 != 0`,
/// and dying bit `0x4000` is clear. Unknown bits are already forced to zero
/// in [`RetailStateWord`], matching constructor residue.
pub fn fun_0042e210_entity_is_live(state: RetailStateWord) -> bool {
    let word = state.known_value_bits();
    word != 0 && (word & DYING_STATE_BIT) == 0
}

/// Count capability `0x400` / `0x800` / `0x08` matches the way `FUN_0042E210`
/// increments its mask/count pairs. One entity may contribute to more than
/// one bucket. Sub-M `+0x68` is added into `+0x1A8` before that loop and is
/// not gated on the dying bit.
pub fn fun_0042e210_capability_census(
    entities: impl IntoIterator<Item = Fun0042e210EntitySample>,
) -> Fun0042e210CapabilityCensus {
    let mut census = Fun0042e210CapabilityCensus::default();
    for sample in entities {
        if let Some(native_count) = sample.sub_m_native_count_raw {
            census.cap_0x400 = census.cap_0x400.saturating_add(native_count);
        }
        if !fun_0042e210_entity_is_live(sample.state) {
            continue;
        }
        if sample.capability & FUN_0042E210_CAPABILITY_0X400 != 0 {
            census.cap_0x400 = census.cap_0x400.saturating_add(1);
        }
        if sample.capability & FUN_0042E210_CAPABILITY_0X800 != 0 {
            census.cap_0x800 = census.cap_0x800.saturating_add(1);
        }
        if sample.capability & FUN_0042E210_CAPABILITY_0X08 != 0 {
            census.cap_0x08 = census.cap_0x08.saturating_add(1);
        }
    }
    census
}

/// Global Section-2 ids for the ordinary world-complete card.
pub const WORLD_SAVED_TIME_TEXT_ID: usize = 0xbd;
pub const TIME_TROPHY_COLLECTED_TEXT_ID: usize = 0xbe;
pub const NATIVES_RESCUED_TEXT_ID: usize = 0xbf;
pub const NATIVES_KILLED_TEXT_ID: usize = 0xc0;
pub const ALIENS_KILLED_TEXT_ID: usize = 0xc1;
pub const LANDSCAPE_VIRUSED_TEXT_ID: usize = 0xc2;
pub const RANK_TEXT_ID: usize = 0xc3;
pub const HIDDEN_TROPHY_TEXT_ID: usize = 0xc4;
/// Overlay-2 local 25 / `DAT_004fe628+100`. `FUN_00454390` draws this after
/// the overlay-51 sprite walk, with no type-on prefix.
pub const RESULTS_CONTINUE_TEXT_ID: usize = 25;
/// Overlay-51 Section-1 last packed Y. PRELOAD S1 cumulative index 78 is
/// `DAT_004fe624+0x138`; Y is that point times display height / 100.
pub const RESULTS_CONTINUE_BASELINE_PERCENT: i32 = 95;
/// `FUN_0042A860(3)` confirm tick from progress-map Space `FUN_00455CC0`.
pub const FUN_00455CC0_CONTINUE_SOUND_ID: usize = 3;

/// Retail CD playback is Ambient nonzero and not on overlay 51.
///
/// `FUN_00456a00` plays when settings `+4 != 0` and otherwise saves position
/// then `MCI_STOP`. Overlay-51 `004D0AA0` does not run that poll; occupancy
/// keeps the selected track paused without rewinding, and `FUN_004558A0`'s
/// restore of `004D0918` returns the Ambient zero/nonzero gate. Ambient
/// magnitude is still only zero/nonzero.
pub fn session_music_should_play(ambient_raw: u32, results_active: bool) -> bool {
    ambient_raw != 0 && !results_active
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldCompleteStats {
    /// Session `+0x2BC` 50-Hz ticks stamped by `FUN_004567b0`.
    pub ticks_0x2bc: i32,
    pub natives_rescued: i32,
    pub natives_killed: i32,
    pub aliens_killed: i32,
    pub virus_percent: i32,
    pub time_trophy: bool,
    pub hidden_trophy: bool,
    /// `FUN_00456ef0` name after the recovered threshold table. The EXE
    /// has no writer of `player_state+0x126`; new-game rank is saved-world
    /// count only.
    pub rank_name: &'static str,
}

impl Default for WorldCompleteStats {
    fn default() -> Self {
        Self {
            ticks_0x2bc: 0,
            natives_rescued: 0,
            natives_killed: 0,
            aliens_killed: 0,
            virus_percent: 0,
            time_trophy: false,
            hidden_trophy: false,
            rank_name: RANK_ABSOLUTE_BEGINNER,
        }
    }
}

/// Live per-world counters. World-load `FUN_0042E570` snapshots `+0x170` /
/// `+0x16C`; the first later `FUN_0042DD10` recenses because `+0xD0` stays 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WorldCompleteTally {
    /// Session `+0x2BC`. `FUN_004567b0` stamps `DAT_004FED60` when the abort
    /// byte is clear and this dword is still `< 1`.
    pub ticks_0x2bc: i32,
    /// World-load `puVar2[0x5c] = +0x1B0 + +0x1A8`.
    pub load_natives_0x170: i32,
    /// World-load `puVar2[0x5b] = +0x1A0`.
    pub load_aliens_0x16c: i32,
    pub natives_rescued: i32,
    pub natives_killed: i32,
    pub aliens_killed: i32,
}

impl WorldCompleteTally {
    /// `FUN_0042E570` census then snapshot of selector-3 / selector-6 bases.
    pub fn snapshot_world_load_census(&mut self, census: Fun0042e210CapabilityCensus) {
        self.load_natives_0x170 =
            fun_0042dd10_selector3_rescued(census.cap_0x400, census.cap_0x800);
        self.load_aliens_0x16c = census.cap_0x08;
        self.natives_rescued = 0;
        self.natives_killed = 0;
        self.aliens_killed = 0;
    }

    /// `FUN_0042DD10` prologue recense when `+0xD0 == 0`, then selectors 3/5/6.
    pub fn recense_fun_0042dd10(&mut self, census: Fun0042e210CapabilityCensus) {
        self.natives_rescued = fun_0042dd10_selector3_rescued(census.cap_0x400, census.cap_0x800);
        self.natives_killed = fun_0042dd10_selector5_natives_killed(
            self.load_natives_0x170,
            census.cap_0x400,
            census.cap_0x800,
        );
        self.aliens_killed =
            fun_0042dd10_selector6_aliens_killed(self.load_aliens_0x16c, census.cap_0x08);
    }

    /// `FUN_004567b0`: stamp `DAT_004FED60` into session `+0x2BC` when the
    /// abort byte `+0x28F` is clear and `+0x2BC < 1`.
    /// A true result also admits2EE70's saved-world transaction. Failed or
    /// already-stamped calls must suppress that transaction as well as time.
    pub fn stamp_fun_004567b0(&mut self, abort_0x28f: bool, dat_004fed60: i32) -> bool {
        if abort_0x28f {
            return false;
        }
        if self.ticks_0x2bc < 1 {
            self.ticks_0x2bc = dat_004fed60;
            return true;
        }
        false
    }

    pub fn snapshot_stats(
        self,
        virus_percent: i32,
        time_trophy: bool,
        hidden_trophy: bool,
        saved_worlds: i32,
    ) -> WorldCompleteStats {
        WorldCompleteStats {
            ticks_0x2bc: self.ticks_0x2bc,
            natives_rescued: self.natives_rescued,
            natives_killed: self.natives_killed,
            aliens_killed: self.aliens_killed,
            virus_percent,
            time_trophy,
            hidden_trophy,
            rank_name: rank_name_for_saved_worlds(saved_worlds),
        }
    }
}

const RANK_ABSOLUTE_BEGINNER: &str = "Absolute Beginner";

/// Exact `FUN_004366F0` display percentage from live `DAT_004dc684`.
///
/// `100 - floor(0x06400000 / (infected_count * 99 + 0x00100000))`. The
/// sibling `FUN_00436720` transform over terrain bit `0x08` is a different
/// owner and is not this overlay-51 row.
pub fn landscape_virus_percent(infected_count: u32) -> i32 {
    let denominator = 0x0010_0000u64 + u64::from(infected_count) * 99;
    100 - (0x0640_0000u64 / denominator) as i32
}

/// `FUN_0042DD10` selector 3: controller `+0x1A8` + `+0x1B0`.
///
/// Overlay-51 field 7 `2` (`0xBF`) reads this through `DAT_004fecdc`.
/// The buckets are the current `FUN_0042E210` census of capability `0x400`
/// and `0x800` after the first `FUN_0042DD10` recense.
pub fn fun_0042dd10_selector3_rescued(cap_0x400: i32, cap_0x800: i32) -> i32 {
    cap_0x400.saturating_add(cap_0x800)
}

/// `FUN_0042DD10` selector 5: controller `+0x170` − `+0x1B0` − `+0x1A8`.
///
/// Overlay-51 field 7 `3` (`0xC0`) reads this through `DAT_004fecdc`.
/// World-load `FUN_0042E570` snapshots `+0x170` to the selector-3 sum;
/// the first later `FUN_0042DD10` recenses live buckets because `+0xD0`
/// stays 0.
pub fn fun_0042dd10_selector5_natives_killed(
    baseline_0x170: i32,
    cap_0x400: i32,
    cap_0x800: i32,
) -> i32 {
    baseline_0x170
        .saturating_sub(cap_0x400)
        .saturating_sub(cap_0x800)
}

/// `FUN_0042DD10` selector 6: `max(0, +0x16C − +0x1A0)`.
///
/// Overlay-51 field 7 `4` (`0xC1`) reads this through `DAT_004fecdc`.
/// World-load snapshots `+0x16C` from the capability-`0x08` census count.
pub fn fun_0042dd10_selector6_aliens_killed(baseline_0x16c: i32, cap_0x08: i32) -> i32 {
    baseline_0x16c.saturating_sub(cap_0x08).max(0)
}

/// `FUN_00452270` case 1: minutes = `+0x2BC / 3000`, seconds =
/// `(+0x2BC / 50) % 60`. The 50-Hz dword is signed, matching the `imul` /
/// `idiv` sequence at `0x004522C3`.
pub const fn fun_00452270_case1_minutes_and_seconds(ticks_0x2bc: i32) -> (i32, i32) {
    (ticks_0x2bc / 3000, ticks_0x2bc / 50 % 60)
}

/// Thresholds recovered with `FUN_00456ef0`. Score is the count of control
/// slots that already carry campaign-completion bit `0x01`.
pub fn rank_name_for_saved_worlds(saved_worlds: i32) -> &'static str {
    const RANKS: &[(i32, &str)] = &[
        (90, "Elite"),
        (80, "Genius"),
        (70, "Ace"),
        (60, "Hero"),
        (50, "Expert"),
        (40, "Competent"),
        (30, "Dependable"),
        (25, "Reliable"),
        (20, "Just Qualified"),
        (15, "Cadet"),
        (10, "Good Trainee"),
        (5, "Trainee"),
        (3, "Beginner"),
        (0, RANK_ABSOLUTE_BEGINNER),
    ];
    RANKS
        .iter()
        .find(|(threshold, _)| saved_worlds >= *threshold)
        .map(|(_, name)| *name)
        .unwrap_or(RANK_ABSOLUTE_BEGINNER)
}

impl WorldCompleteStats {
    pub const fn saved_minutes_and_seconds(self) -> (u32, u32) {
        let (minutes, seconds) = fun_00452270_case1_minutes_and_seconds(self.ticks_0x2bc);
        (minutes as u32, seconds as u32)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum CompletionPresentation {
    #[default]
    Hidden,
    HiveStatistics,
    ProgressMap(CampaignTransition),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorldCompleteResultsRuntime {
    presentation: CompletionPresentation,
    age_ms: i32,
    stats: WorldCompleteStats,
}

impl WorldCompleteResultsRuntime {
    /// `FUN_004567B0` does not enter a modal screen. Its stamped result is
    /// presented by the existing in-game text callback, `FUN_00452CB0`.
    pub fn record_hive_completion(&mut self, stats: WorldCompleteStats) {
        self.presentation = CompletionPresentation::HiveStatistics;
        self.age_ms = 0;
        self.stats = stats;
    }

    /// The admitted dead-hive exit owns the map and its destination. Keeping
    /// the route here prevents Space before contact from skipping the wreck.
    pub fn open_progress_map(&mut self, route: CampaignTransition) {
        self.presentation = CompletionPresentation::ProgressMap(route);
        self.age_ms = 0;
    }

    pub const fn is_progress_map_active(&self) -> bool {
        matches!(self.presentation, CompletionPresentation::ProgressMap(_))
    }

    /// Map-S snapshots this route without releasing it; Resume or Space owns
    /// the eventual destination load.
    pub const fn progress_map_route(&self) -> Option<CampaignTransition> {
        match self.presentation {
            CompletionPresentation::ProgressMap(route) => Some(route),
            _ => None,
        }
    }

    pub const fn has_hive_statistics(&self) -> bool {
        matches!(self.presentation, CompletionPresentation::HiveStatistics)
            && self.stats.ticks_0x2bc > 500
    }

    /// `FUN_00454460` uses session `+0x270 / 100` for the map blink.
    pub const fn age_ms(&self) -> i32 {
        self.age_ms
    }

    /// `004D0AA0` replaces the HUD. Completion rows in `004D0918` do not.
    pub const fn replaces_ingame_hud(&self) -> bool {
        self.is_progress_map_active()
    }

    /// Progress-map Space (`FUN_00455CC0`) releases the selected transition;
    /// no completion-HUD input can synthesize a route or dismiss those rows.
    pub fn continue_progress_map(&mut self) -> Option<CampaignTransition> {
        let CompletionPresentation::ProgressMap(route) = self.presentation else {
            return None;
        };
        self.presentation = CompletionPresentation::Hidden;
        Some(route)
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn advance(&mut self, elapsed_micros: u32) {
        if self.presentation == CompletionPresentation::Hidden {
            return;
        }
        self.age_ms = self.age_ms.saturating_add((elapsed_micros / 1_000) as i32);
    }

    pub fn presentation<'a>(
        &self,
        resolve_string: impl FnMut(usize) -> Option<&'a str>,
    ) -> Vec<GameplayNotificationLine> {
        self.present(None, 0, resolve_string).lines
    }

    /// Same row visit as [`Self::presentation`], plus `FUN_00452790`'s shared
    /// `DAT_004F72E8` type-on cue. Table order is delay-gated `0xBD..=0xC4`.
    pub fn presentation_with_typewriter<'a>(
        &self,
        cadence: &mut TextTypewriterCadence,
        retail_tick: i32,
        resolve_string: impl FnMut(usize) -> Option<&'a str>,
    ) -> GameplayNotificationPresentation {
        self.present(Some(cadence), retail_tick, resolve_string)
    }

    /// `FUN_00454390` continue line. Centered on the display width; baseline
    /// is overlay-51 Section-1 index 30 / global S1 78 (Y = 95).
    pub fn continue_prompt_line<'a>(
        &self,
        resolve_string: impl FnOnce(usize) -> Option<&'a str>,
    ) -> Option<GameplayNotificationLine> {
        if !self.is_progress_map_active() {
            return None;
        }
        let text = resolve_string(RESULTS_CONTINUE_TEXT_ID)?.to_owned();
        if text.is_empty() {
            return None;
        }
        Some(GameplayNotificationLine {
            string_id: RESULTS_CONTINUE_TEXT_ID,
            text,
            x_percent: 50,
            baseline_percent: RESULTS_CONTINUE_BASELINE_PERCENT,
            width_percent: 100,
            center_x: true,
        })
    }

    fn present<'a>(
        &self,
        mut cadence: Option<&mut TextTypewriterCadence>,
        retail_tick: i32,
        mut resolve_string: impl FnMut(usize) -> Option<&'a str>,
    ) -> GameplayNotificationPresentation {
        if !self.has_hive_statistics() {
            return GameplayNotificationPresentation::default();
        }
        let mut presentation = GameplayNotificationPresentation::default();
        for &(string_id, required) in RESULT_LINES {
            if !self.should_show(required) {
                continue;
            }
            let Some(raw) = resolve_string(string_id) else {
                continue;
            };
            let Some(parsed) =
                present_results_line(string_id, raw, self.stats, self.age_ms, retail_tick)
            else {
                continue;
            };
            if let Some(cadence) = cadence.as_mut() {
                if cadence.observe_active_line(parsed.reveal_incomplete, retail_tick) {
                    presentation.play_typewriter_sound = true;
                }
            }
            if let Some(line) = parsed.line {
                presentation.lines.push(line);
            }
        }
        presentation
    }

    fn should_show(&self, required: ResultLineRequirement) -> bool {
        match required {
            ResultLineRequirement::Always => true,
            ResultLineRequirement::TimeTrophy => self.stats.time_trophy,
            ResultLineRequirement::HiddenTrophy => self.stats.hidden_trophy,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResultLineRequirement {
    Always,
    TimeTrophy,
    HiddenTrophy,
}

const RESULT_LINES: &[(usize, ResultLineRequirement)] = &[
    (WORLD_SAVED_TIME_TEXT_ID, ResultLineRequirement::Always),
    (
        TIME_TROPHY_COLLECTED_TEXT_ID,
        ResultLineRequirement::TimeTrophy,
    ),
    (NATIVES_RESCUED_TEXT_ID, ResultLineRequirement::Always),
    (NATIVES_KILLED_TEXT_ID, ResultLineRequirement::Always),
    (ALIENS_KILLED_TEXT_ID, ResultLineRequirement::Always),
    (LANDSCAPE_VIRUSED_TEXT_ID, ResultLineRequirement::Always),
    (RANK_TEXT_ID, ResultLineRequirement::Always),
    (HIDDEN_TROPHY_TEXT_ID, ResultLineRequirement::HiddenTrophy),
];

struct PresentedResultsLine {
    line: Option<GameplayNotificationLine>,
    reveal_incomplete: bool,
}

fn present_results_line(
    string_id: usize,
    raw: &str,
    stats: WorldCompleteStats,
    age_ms: i32,
    retail_tick: i32,
) -> Option<PresentedResultsLine> {
    let trimmed = raw.trim_start_matches([' ', '\t']);
    let (parameters, body) = if let Some(rest) = trimmed.strip_prefix('<') {
        let end = rest.find('>')?;
        (&rest[..end], &rest[end + 1..])
    } else {
        ("", trimmed)
    };
    let fields = parameters
        .split(',')
        .map(parse_prefix_integer)
        .collect::<Vec<_>>();
    let delay_ms = fields.first().copied().unwrap_or(0);
    let end_ms = fields.get(1).copied().unwrap_or(0);
    if age_ms < delay_ms || (end_ms != 0 && end_ms < age_ms) {
        return None;
    }
    let x_percent = fields.get(3).copied().unwrap_or(15);
    let baseline_percent = fields.get(4).copied().unwrap_or(30);
    let width_percent = fields.get(5).copied().unwrap_or(85);
    let field7 = fields.get(7).copied().unwrap_or(0);
    let formatted = fun_00452270_apply(field7, body, stats, retail_tick);
    let (text, reveal_incomplete) = type_on_results_body(
        formatted,
        age_ms.saturating_sub(delay_ms),
        fields.get(6).copied().unwrap_or(0),
    );
    let line = (!text.is_empty()).then_some(GameplayNotificationLine {
        string_id,
        text,
        x_percent,
        baseline_percent,
        width_percent,
        center_x: false,
    });
    Some(PresentedResultsLine {
        line,
        reveal_incomplete,
    })
}

/// `FUN_00452790` type-on for a wildcard-layout results prefix.
///
/// Field 6 is `iVar4`. Zero becomes 1 and skips truncation (`1 < iVar4`).
/// The cut is a byte index into the already-formatted body; authored rows
/// are single-byte ASCII after `%d`/`%s` substitution.
fn type_on_results_body(
    formatted: String,
    age_after_delay_ms: i32,
    interval_ms: i32,
) -> (String, bool) {
    let interval_ms = if interval_ms == 0 { 1 } else { interval_ms };
    if interval_ms <= 1 {
        return (formatted, false);
    }
    let visible = (age_after_delay_ms.max(0) / interval_ms) as usize;
    if visible >= formatted.len() {
        return (formatted, false);
    }
    (formatted[..visible].to_owned(), true)
}

/// `FUN_00452270` selected by overlay-51 field 7 (`local_f8`).
///
/// Retail mutates the copied body, then `FUN_00452790` types on that result.
/// Cases `0xC`/`0xE` empty the buffer; they do not skip the type-on visit.
/// Unrecovered face commands keep the authored body instead of inventing the
/// live-entity walk.
fn fun_00452270_apply(
    field7: i32,
    body: &str,
    stats: WorldCompleteStats,
    retail_tick: i32,
) -> String {
    let formatted = match field7 {
        0 => body.to_owned(),
        1 => {
            let (minutes, seconds) = stats.saved_minutes_and_seconds();
            body.replacen("%d", &minutes.to_string(), 1).replacen(
                "%02d",
                &format!("{seconds:02}"),
                1,
            )
        }
        2 => body.replacen("%d", &stats.natives_rescued.to_string(), 1),
        3 => body.replacen("%d", &stats.natives_killed.to_string(), 1),
        4 => body.replacen("%d", &stats.aliens_killed.to_string(), 1),
        5 => body.replacen("%d", &stats.virus_percent.to_string(), 1),
        0x0c => {
            if stats.hidden_trophy {
                body.to_owned()
            } else {
                String::new()
            }
        }
        0x0d => body.replacen("%s", stats.rank_name, 1),
        0x0e => {
            if !stats.time_trophy || fun_00452270_case_e_hides_line(retail_tick) {
                String::new()
            } else {
                body.to_owned()
            }
        }
        0x0f => {
            if fun_00452270_case_e_hides_line(retail_tick) {
                String::new()
            } else {
                body.to_owned()
            }
        }
        _ => body.to_owned(),
    };
    formatted.replace("%%", "%")
}

fn parse_prefix_integer(field: &str) -> i32 {
    let field = field.trim();
    if field.is_empty() || field == "*" {
        0
    } else {
        field.parse().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stats() -> WorldCompleteStats {
        WorldCompleteStats {
            ticks_0x2bc: 6_250,
            natives_rescued: 3,
            natives_killed: 1,
            aliens_killed: 7,
            virus_percent: 12,
            time_trophy: true,
            hidden_trophy: false,
            rank_name: RANK_ABSOLUTE_BEGINNER,
        }
    }

    fn resolve(id: usize) -> Option<&'static str> {
        match id {
            0xbd => Some("        <   *,10000,*,10,10,90,30,1>World saved in %d:%02d."),
            0xbe => Some("\t     <5000,10000,*,15,17,90,30,14>Time trophy collected."),
            0xbf => Some(" <1000,10000,*,15,30,85,30,2>You rescued %d natives."),
            0xc0 => Some(" <1500,10000,*,15,37,85,30,3>%d natives were killed."),
            0xc1 => Some(" <2000,10000,*,15,44,85,30,4>You killed %d alien creatures."),
            0xc2 => Some(" <2500,10000,*,15,51,85,30,5>The landscape was %d%% virused."),
            0xc3 => Some(" <3000,10000,*,15,58,85,30,13>Your rank is %s."),
            0xc4 => Some(" <3500,10000,*,15,65,85,30,12>You found the hidden trophy."),
            _ => None,
        }
    }

    #[test]
    fn world_saved_line_uses_elapsed_minutes_and_seconds() {
        assert_eq!(fun_00452270_case1_minutes_and_seconds(0), (0, 0));
        assert_eq!(fun_00452270_case1_minutes_and_seconds(49), (0, 0));
        assert_eq!(fun_00452270_case1_minutes_and_seconds(50), (0, 1));
        assert_eq!(fun_00452270_case1_minutes_and_seconds(2_999), (0, 59));
        assert_eq!(fun_00452270_case1_minutes_and_seconds(3_000), (1, 0));
        assert_eq!(fun_00452270_case1_minutes_and_seconds(6_250), (2, 5));
        assert_eq!(stats().saved_minutes_and_seconds(), (2, 5));
        let line = present_results_line(0xbd, resolve(0xbd).unwrap(), stats(), 600, 0)
            .unwrap()
            .line
            .unwrap();
        assert_eq!(line.text, "World saved in 2:05.");
        assert_eq!(line.x_percent, 10);
        assert_eq!(line.baseline_percent, 10);
    }

    #[test]
    fn field_six_types_on_after_the_row_delay() {
        let raw = resolve(0xbd).unwrap();
        let at_open = present_results_line(0xbd, raw, stats(), 0, 0).unwrap();
        assert!(at_open.line.is_none());
        assert!(at_open.reveal_incomplete);
        let partial = present_results_line(0xbd, raw, stats(), 150, 0)
            .unwrap()
            .line
            .unwrap();
        assert_eq!(partial.text, "World");
        let complete = present_results_line(0xbd, raw, stats(), 600, 0)
            .unwrap()
            .line
            .unwrap();
        assert_eq!(complete.text, "World saved in 2:05.");
    }

    #[test]
    fn stagger_reveals_the_observed_summary_rows() {
        let mut runtime = WorldCompleteResultsRuntime::default();
        runtime.record_hive_completion(stats());
        assert!(runtime.presentation(resolve).is_empty());

        runtime.age_ms = 4_000;
        let mid = runtime.presentation(resolve);
        assert_eq!(
            mid.iter().map(|line| line.string_id).collect::<Vec<_>>(),
            vec![0xbd, 0xbf, 0xc0, 0xc1, 0xc2, 0xc3]
        );
        assert_eq!(mid[1].text, "You rescued 3 natives.");
        assert_eq!(mid[2].text, "1 natives were killed.");
        assert_eq!(mid[3].text, "You killed 7 alien creatures.");
        assert_eq!(mid[4].text, "The landscape was 12% virused.");
        assert_eq!(mid[5].text, "Your rank is Absolute Beginner.");
    }

    #[test]
    fn optional_trophy_rows_follow_campaign_bits() {
        let mut without = stats();
        without.time_trophy = false;
        without.hidden_trophy = false;
        let mut runtime = WorldCompleteResultsRuntime::default();
        runtime.record_hive_completion(without);
        runtime.age_ms = 10_000;
        let ids = runtime
            .presentation(resolve)
            .iter()
            .map(|line| line.string_id)
            .collect::<Vec<_>>();
        assert!(!ids.contains(&0xbe));
        assert!(!ids.contains(&0xc4));
        assert!(ids.contains(&0xc2));
    }

    #[test]
    fn virus_line_collapses_printf_percent_escapes() {
        let line = present_results_line(0xc2, resolve(0xc2).unwrap(), stats(), 3_500, 0)
            .unwrap()
            .line
            .unwrap();
        assert_eq!(line.text, "The landscape was 12% virused.");
    }

    #[test]
    fn rank_follows_saved_world_thresholds() {
        assert_eq!(rank_name_for_saved_worlds(0), "Absolute Beginner");
        assert_eq!(rank_name_for_saved_worlds(1), "Absolute Beginner");
        assert_eq!(rank_name_for_saved_worlds(3), "Beginner");
        assert_eq!(rank_name_for_saved_worlds(90), "Elite");
    }

    #[test]
    fn landscape_virus_percent_matches_fun_004366f0() {
        assert_eq!(landscape_virus_percent(0), 0);
        assert_eq!(landscape_virus_percent(1), 1);
        assert_eq!(landscape_virus_percent(95), 1);
        assert_eq!(landscape_virus_percent(0x1_0000), 87);
    }

    #[test]
    fn hive_statistics_keep_the_hud_and_cannot_continue_a_world() {
        let mut runtime = WorldCompleteResultsRuntime::default();
        runtime.record_hive_completion(stats());
        runtime.age_ms = 10_000;
        assert!(runtime.has_hive_statistics());
        assert!(!runtime.replaces_ingame_hud());
        assert!(!runtime.presentation(resolve).is_empty());
        assert_eq!(runtime.continue_progress_map(), None);
        assert!(runtime.continue_prompt_line(resolve).is_none());
        runtime.reset();
        assert!(runtime.presentation(resolve).is_empty());
    }

    #[test]
    fn fun_00452270_case_e_hides_on_even_dat_004fed60_over_40() {
        assert!(fun_00452270_case_e_hides_line(0));
        assert!(fun_00452270_case_e_hides_line(39));
        assert!(!fun_00452270_case_e_hides_line(40));
        assert!(!fun_00452270_case_e_hides_line(79));
        assert!(fun_00452270_case_e_hides_line(80));
        let raw = resolve(0xbe).unwrap();
        let hidden = present_results_line(0xbe, raw, stats(), 6_000, 0).unwrap();
        assert!(hidden.line.is_none());
        assert!(!hidden.reveal_incomplete);
        let mid_type_on = present_results_line(0xbe, raw, stats(), 5_150, 0).unwrap();
        assert!(mid_type_on.line.is_none());
        assert!(!mid_type_on.reveal_incomplete);
        let shown = present_results_line(0xbe, raw, stats(), 6_000, 40)
            .unwrap()
            .line
            .unwrap();
        assert_eq!(shown.text, "Time trophy collected.");
    }

    #[test]
    fn fun_00452270_field7_not_string_id_selects_the_case() {
        let time = present_results_line(
            0x99,
            "<*,10000,*,10,10,90,30,1>World saved in %d:%02d.",
            stats(),
            600,
            0,
        )
        .unwrap()
        .line
        .unwrap();
        assert_eq!(time.text, "World saved in 2:05.");
        let rescued = present_results_line(
            0x99,
            "<*,10000,*,15,30,85,30,2>You rescued %d natives.",
            stats(),
            1_000,
            0,
        )
        .unwrap()
        .line
        .unwrap();
        assert_eq!(rescued.text, "You rescued 3 natives.");
    }

    #[test]
    fn fun_004567b0_stamps_dat_004fed60_once_when_abort_is_clear() {
        let mut tally = WorldCompleteTally::default();
        assert!(!tally.stamp_fun_004567b0(true, 6_250));
        assert_eq!(tally.ticks_0x2bc, 0);
        assert!(tally.stamp_fun_004567b0(false, 6_250));
        assert_eq!(tally.ticks_0x2bc, 6_250);
        assert!(!tally.stamp_fun_004567b0(false, 9_000));
        assert_eq!(tally.ticks_0x2bc, 6_250);
    }

    #[test]
    fn overlay_51_stat_selectors_match_fun_0042dd10() {
        assert_eq!(fun_0042dd10_selector3_rescued(2, 7), 9);
        assert_eq!(fun_0042dd10_selector5_natives_killed(9, 2, 7), 0);
        assert_eq!(fun_0042dd10_selector5_natives_killed(9, 2, 4), 3);
        assert_eq!(fun_0042dd10_selector6_aliens_killed(12, 12), 0);
        assert_eq!(fun_0042dd10_selector6_aliens_killed(12, 5), 7);
        assert_eq!(fun_0042dd10_selector6_aliens_killed(5, 12), 0);
    }

    fn sample(
        capability: u32,
        state: RetailStateWord,
        sub_m_native_count_raw: Option<i32>,
    ) -> Fun0042e210EntitySample {
        Fun0042e210EntitySample {
            capability,
            state,
            sub_m_native_count_raw,
        }
    }

    #[test]
    fn fun_0042e210_skips_zero_dying_and_unresolved_state() {
        let live = RetailStateWord::exact(0x0800_0004);
        let dying = RetailStateWord::exact(0x0800_0004 | DYING_STATE_BIT);
        let census = fun_0042e210_capability_census([
            sample(0x1804, live, None),
            sample(0x1804, dying, None),
            sample(0x1804, RetailStateWord::exact(0), None),
            sample(0x1804, RetailStateWord::unknown(), None),
            sample(0x0400, live, None),
            sample(0x0008, live, None),
            sample(0x0008, dying, None),
        ]);
        assert_eq!(
            census,
            Fun0042e210CapabilityCensus {
                cap_0x400: 1,
                cap_0x800: 1,
                cap_0x08: 1,
            }
        );
    }

    #[test]
    fn fun_0042e210_adds_sub_m_native_count_before_capability_loop() {
        let live = RetailStateWord::exact(0x0800_0004);
        let dying = RetailStateWord::exact(0x0800_0004 | DYING_STATE_BIT);
        let census = fun_0042e210_capability_census([
            sample(0x84, live, Some(2)),
            sample(0x20, dying, Some(1)),
            sample(0x0400, live, None),
        ]);
        assert_eq!(
            census,
            Fun0042e210CapabilityCensus {
                cap_0x400: 4,
                cap_0x800: 0,
                cap_0x08: 0,
            }
        );
    }

    #[test]
    fn overlay_51_recense_fills_selectors_3_5_6_from_load_snapshot() {
        let mut tally = WorldCompleteTally::default();
        tally.snapshot_world_load_census(Fun0042e210CapabilityCensus {
            cap_0x400: 0,
            cap_0x800: 6,
            cap_0x08: 7,
        });
        assert_eq!(tally.load_natives_0x170, 6);
        assert_eq!(tally.load_aliens_0x16c, 7);
        tally.recense_fun_0042dd10(Fun0042e210CapabilityCensus {
            cap_0x400: 1,
            cap_0x800: 3,
            cap_0x08: 2,
        });
        assert_eq!(tally.natives_rescued, 4);
        assert_eq!(tally.natives_killed, 2);
        assert_eq!(tally.aliens_killed, 5);
    }
}
