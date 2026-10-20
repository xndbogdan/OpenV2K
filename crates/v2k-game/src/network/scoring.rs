//! Names, scores and message lines ([MULTIPLAYER.md](../../../../docs/re/MULTIPLAYER.md),
//! "Names and announcements", "Kills and scores").

use super::slots::SLOTS;

/// `FUN_00438EF0`: a message line shows for 7.5 seconds.
pub const LINE_LIFETIME_US: u32 = 7_500_000;
/// `FUN_00439DB0`: a hit by a player is remembered this long, so a death
/// soon after still credits that player.
pub const KILLER_HOLD_US: u32 = 750_000;
/// `FUN_00438CB0`: once there is a winner the host repeats a winner line
/// every 150 ticks (3 seconds).
pub const ANNOUNCE_PERIOD_TICKS: u32 = 150;

/// A line table: one of `kill.txt`, `Die.txt`, `selfkill.txt`, `Names.txt`
/// and `Host.txt` (`FUN_004393C0`), each line ending at a line break.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LineTable(pub Vec<String>);

impl LineTable {
    /// `FUN_004394F0`'s reading: each line loses its leading blanks and its
    /// line break; empty lines and lines starting with `#` are skipped; at
    /// most `limit` lines (10, or 2 for `Names.txt`). Without a line,
    /// `FUN_004393C0` keeps the built-in text as the only one.
    pub fn parse(text: Option<&str>, limit: usize, built_in: &str) -> Self {
        let lines: Vec<String> = text
            .unwrap_or("")
            .split('\n')
            .map(|line| {
                let line = line.trim_start();
                line.split('\r').next().unwrap_or("").to_string()
            })
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .take(limit)
            .collect();
        if lines.is_empty() {
            Self(vec![built_in.to_string()])
        } else {
            Self(lines)
        }
    }

    pub fn pick(&self, random: u32) -> &str {
        if self.0.is_empty() {
            return "";
        }
        &self.0[(random & 0xFFFF) as usize % self.0.len()]
    }
}

/// The five tables.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LineTables {
    pub kill: LineTable,
    pub die: LineTable,
    pub self_kill: LineTable,
    pub names: LineTable,
    pub host: LineTable,
}

/// A slot's row of the score table (`004FE7E0`, 6 bytes a slot).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScoreRow {
    pub kills: u16,
    pub deaths: u16,
    pub score: u16,
}

/// What a step of the scoring asks the session to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScoringEffect {
    /// Send a kill report (kind 17) to the host.
    SendKillReport { victim: u8, attacker: u8 },
    /// Send the client's kill counter (kind 16).
    SendScoreWord(u32),
    /// Send a line to everyone (kind 21).
    SendLine(String),
    /// `FUN_00456750`: the session's flash request (`+295`).
    WinnerFlash,
}

/// The multiplayer state `FUN_00439290` resets.
#[derive(Clone, Debug)]
pub struct Scoring {
    /// `004FE700`, 16 bytes a slot.
    pub names: [String; SLOTS],
    /// `004FE7A0`, 6 bytes a slot.
    pub initials: [String; SLOTS],
    /// `004FE6E0`.
    pub known: [bool; SLOTS],
    pub table: [ScoreRow; SLOTS],
    /// `004DC9C0`'s low word: 0 plays without a limit.
    pub frag_limit: u16,
    /// `004DC9C0`'s high word: the winner's slot.
    pub winner: u8,
    /// `004DC9A4`: the kill counter and the last killer.
    pub kill_counter: u16,
    pub last_killer: u8,
    /// `004DC9C8`/`004DC9C4`.
    pub killer: u8,
    pub killer_hold_us: u32,
    /// `004DC7C8`: each client's last score word, on the host.
    pub score_words: [u32; SLOTS],
    /// `004DC9A0`: the last 150-second period announced.
    pub announced_period: u32,
    /// `004DC7F0`: four message lines and their lifetimes.
    pub lines: [(u32, String); 4],
    /// `004DC9DC`: the oldest line.
    pub oldest_line: usize,
}

impl Scoring {
    /// `FUN_00439290`: names "Player %d", nothing known or scored.
    pub fn new() -> Self {
        Self {
            names: std::array::from_fn(|slot| format!("Player {slot}")),
            initials: Default::default(),
            known: [false; SLOTS],
            table: [ScoreRow::default(); SLOTS],
            frag_limit: 0,
            winner: 0,
            kill_counter: 0,
            last_killer: 0,
            killer: 0,
            killer_hold_us: 0,
            score_words: [0; SLOTS],
            announced_period: 0,
            lines: Default::default(),
            oldest_line: 0,
        }
    }

    /// `FUN_00439010`'s first part for the local slot: the profile's name and
    /// initials, replaced by `Names.txt`'s lines when present; on the host,
    /// the frag limit from `Host.txt`'s first line, clamped to 50..=5000.
    pub fn record_local_name(
        &mut self,
        slot: u8,
        profile: (&str, &str),
        tables: &LineTables,
        host: bool,
    ) {
        let slot = usize::from(slot);
        if slot == 0 || slot >= SLOTS || self.known[slot] {
            return;
        }
        self.names[slot] = truncate(profile.0, 15);
        self.initials[slot] = truncate(profile.1, 5);
        if let Some(name) = tables.names.0.first().filter(|name| !name.is_empty()) {
            self.names[slot] = truncate(name, 15);
            if let Some(initials) = tables.names.0.get(1).filter(|text| !text.is_empty()) {
                self.initials[slot] = truncate(initials, 5);
            }
        }
        self.known[slot] = true;
        if host {
            if let Some(first) = tables.host.0.first() {
                self.frag_limit = atoi(first) as u16;
                if self.frag_limit != 0 {
                    self.frag_limit = self.frag_limit.clamp(50, 5000);
                }
            }
        }
    }

    /// `FUN_00439DB0`: a player hit by a player. The attacker is held for
    /// 750 ms; the host scores the hit, a client reports it.
    pub fn report_hit(&mut self, victim: u8, attacker: u8, host: bool) -> Vec<ScoringEffect> {
        self.killer = attacker;
        self.killer_hold_us = KILLER_HOLD_US;
        if host {
            self.score_hit(victim, attacker)
        } else {
            vec![ScoringEffect::SendKillReport { victim, attacker }]
        }
    }

    /// `FUN_00439A70`, on the host: a self-hit costs a point; otherwise the
    /// attacker gains one (two when behind) and the victim loses as many
    /// when it has more. Reaching the limit names the winner.
    pub fn score_hit(&mut self, victim: u8, attacker: u8) -> Vec<ScoringEffect> {
        let (v, a) = (usize::from(victim), usize::from(attacker));
        if self.winner != 0 || v >= SLOTS || a >= SLOTS {
            return Vec::new();
        }
        if v == a {
            if self.table[a].score != 0 {
                self.table[a].score -= 1;
            }
        } else {
            let gain = u16::from(self.table[a].score < self.table[v].score) + 1;
            self.table[a].score = self.table[a].score.wrapping_add(gain);
            if gain < self.table[v].score {
                self.table[v].score -= gain;
            }
        }
        self.check_winner(attacker)
    }

    /// `FUN_00439990`, on the host: a kill counts for the killer and a
    /// death for the victim; the killer scores 20 (30 when behind), a
    /// self-kill costs 20.
    pub fn score_kill(&mut self, victim: u8, killer: u8) -> Vec<ScoringEffect> {
        let (v, k) = (usize::from(victim), usize::from(killer));
        if self.winner != 0 || v >= SLOTS || k >= SLOTS {
            return Vec::new();
        }
        let behind = self.table[k].score < self.table[v].score;
        self.table[v].deaths = self.table[v].deaths.wrapping_add(1);
        if v == k {
            self.table[k].score = self.table[k].score.saturating_sub(20);
        } else {
            self.table[k].kills = self.table[k].kills.wrapping_add(1);
            self.table[k].score = self.table[k]
                .score
                .wrapping_add(if behind { 30 } else { 20 });
        }
        self.check_winner(killer)
    }

    fn check_winner(&mut self, slot: u8) -> Vec<ScoringEffect> {
        if self.frag_limit != 0 && self.frag_limit <= self.table[usize::from(slot)].score {
            self.winner = slot;
            return vec![ScoringEffect::WinnerFlash];
        }
        Vec::new()
    }

    /// `FUN_00439C90`: the local player died. The victim names the killer,
    /// or the held attacker when the world killed it soon after a hit; the
    /// line is a self-kill, death or kill line with the names in. The host
    /// scores the kill; a client advances its kill counter and sends it.
    pub fn announce_kill(
        &mut self,
        victim: u8,
        killer: u8,
        tables: &LineTables,
        random: u32,
        host: bool,
    ) -> Vec<ScoringEffect> {
        if self.winner != 0 {
            return Vec::new();
        }
        let mut killer = killer;
        if killer == 0 && self.killer_hold_us != 0 {
            killer = self.killer;
        }
        self.kill_counter = self.kill_counter.wrapping_add(1);
        self.last_killer = killer;
        let (template, second) = if victim == killer {
            (tables.self_kill.pick(random), killer)
        } else if killer == 0 {
            (tables.die.pick(random), 0)
        } else {
            (tables.kill.pick(random), killer)
        };
        let line = self.format(template, victim, second);
        self.add_line(&line);
        let mut effects = Vec::new();
        if host {
            effects.extend(self.score_kill(victim, killer));
        } else {
            effects.push(ScoringEffect::SendScoreWord(self.score_word()));
        }
        effects.push(ScoringEffect::SendLine(line));
        effects
    }

    /// `004DC9A4` as sent in kind 16.
    pub fn score_word(&self) -> u32 {
        u32::from(self.kill_counter) | u32::from(self.last_killer) << 16
    }

    /// `FUN_00439940` on the host: a client's counter moved, so its player
    /// died to the named killer.
    pub fn apply_score_word(&mut self, sender: u8, word: u32) -> Vec<ScoringEffect> {
        let slot = usize::from(sender);
        if slot >= SLOTS || self.score_words[slot] as u16 == word as u16 {
            return Vec::new();
        }
        self.score_words[slot] = word;
        self.score_kill(sender, (word >> 16) as u8)
    }

    /// `FUN_004398E0`: the host's table; a new winner flashes.
    pub fn apply_scores(&mut self, limit_winner: u32, table: &[[u16; 3]; 8]) -> Vec<ScoringEffect> {
        let previous = self.winner;
        self.frag_limit = limit_winner as u16;
        self.winner = (limit_winner >> 16) as u8;
        for (slot, row) in table.iter().enumerate() {
            self.table[slot + 1] = ScoreRow {
                kills: row[0],
                deaths: row[1],
                score: row[2],
            };
        }
        if previous != self.winner {
            vec![ScoringEffect::WinnerFlash]
        } else {
            Vec::new()
        }
    }

    /// `FUN_00439B40`'s record.
    pub fn scores_record(&self) -> (u32, [[u16; 3]; 8]) {
        let mut table = [[0u16; 3]; 8];
        for (slot, row) in table.iter_mut().enumerate() {
            let source = self.table[slot + 1];
            *row = [source.kills, source.deaths, source.score];
        }
        (
            u32::from(self.frag_limit) | u32::from(self.winner) << 16,
            table,
        )
    }

    /// `FUN_00438CB0`'s host announcement: once there is a winner, a random
    /// `Host.txt` line after the first, every 150 ticks of the 50 Hz clock
    /// `DAT_004FED60`.
    pub fn host_announcement(
        &mut self,
        ticks: u32,
        tables: &LineTables,
        random: u32,
    ) -> Option<String> {
        if self.winner == 0 || tables.host.0.len() < 2 {
            return None;
        }
        let period = ticks / ANNOUNCE_PERIOD_TICKS;
        if self.announced_period == period {
            return None;
        }
        self.announced_period = period;
        let lines = &tables.host.0[1..];
        let template = &lines[(random & 0xFFFF) as usize % lines.len()];
        let line = self.format(template, self.winner, self.winner);
        self.add_line(&line);
        Some(line)
    }

    /// `FUN_00439BE0`: `@1` and `@2` become the two players' names, "a
    /// stranger" when unknown; at most 79 characters.
    pub fn format(&self, template: &str, first: u8, second: u8) -> String {
        let name = |slot: u8| {
            let slot = usize::from(slot);
            if slot < SLOTS && self.known[slot] {
                self.names[slot].clone()
            } else {
                "a stranger".to_string()
            }
        };
        let mut out = String::new();
        let mut chars = template.chars().peekable();
        while let Some(c) = chars.next() {
            if out.len() >= 79 {
                break;
            }
            if c == '@' {
                match chars.peek() {
                    Some('1') => {
                        chars.next();
                        out.push_str(&name(first));
                    }
                    Some('2') => {
                        chars.next();
                        out.push_str(&name(second));
                    }
                    _ => {}
                }
                continue;
            }
            out.push(c);
        }
        out.truncate(79);
        out
    }

    /// `FUN_00438EF0` with the line in: a free line, else the oldest.
    pub fn add_line(&mut self, text: &str) {
        let free = (0..4)
            .map(|i| (i + self.oldest_line) & 3)
            .find(|&i| self.lines[i].0 == 0);
        let index = free.unwrap_or_else(|| {
            let index = self.oldest_line;
            self.oldest_line = (self.oldest_line + 1) & 3;
            index
        });
        self.lines[index] = (LINE_LIFETIME_US, truncate(text, 79));
    }

    /// `FUN_00438A20`'s countdown: the live lines, oldest first.
    pub fn age_lines(&mut self, elapsed_us: u32) -> Vec<String> {
        for line in &mut self.lines {
            line.0 = line.0.saturating_sub(elapsed_us);
        }
        for _ in 0..4 {
            if self.lines[self.oldest_line].0 == 0 {
                self.oldest_line = (self.oldest_line + 1) & 3;
            }
        }
        (0..4)
            .map(|i| (self.oldest_line + i) & 3)
            .take_while(|&i| self.lines[i].0 != 0)
            .map(|i| self.lines[i].1.clone())
            .collect()
    }

    /// The session tick's countdown of the killer hold.
    pub fn age_killer(&mut self, elapsed_us: u32) {
        self.killer_hold_us = self.killer_hold_us.saturating_sub(elapsed_us);
    }

    /// `FUN_00439010`'s departure: a slot whose player changed loses its
    /// score row and the counter its client sent; when a player was there,
    /// its name is forgotten too.
    pub fn forget_slot(&mut self, slot: u8, had_player: bool) {
        let slot = usize::from(slot);
        if slot < SLOTS {
            self.table[slot] = ScoreRow::default();
            self.score_words[slot] &= 0xFFFF_0000;
            if had_player {
                self.known[slot] = false;
            }
        }
    }
}

impl Default for Scoring {
    fn default() -> Self {
        Self::new()
    }
}

fn truncate(text: &str, limit: usize) -> String {
    text.chars().take(limit).collect()
}

/// The CRT `atoi` of `FUN_004B41D0`: leading blanks, a sign, digits.
fn atoi(text: &str) -> i32 {
    let text = text.trim_start();
    let (sign, digits) = match text.strip_prefix('-') {
        Some(rest) => (-1, rest),
        None => (1, text.strip_prefix('+').unwrap_or(text)),
    };
    let mut value: i32 = 0;
    for c in digits.chars() {
        let Some(digit) = c.to_digit(10) else { break };
        value = value.wrapping_mul(10).wrapping_add(digit as i32);
    }
    sign * value
}

#[cfg(test)]
#[path = "scoring_tests.rs"]
mod tests;
