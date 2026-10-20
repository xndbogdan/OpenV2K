use super::*;

fn tables() -> LineTables {
    LineTables {
        kill: LineTable(vec!["@1 was fragged by @2".into()]),
        die: LineTable(vec!["@1 died".into()]),
        self_kill: LineTable(vec!["@1 shot @2self".into()]),
        names: LineTable(vec![String::new()]),
        host: LineTable(vec!["60".into(), "@1 wins".into()]),
    }
}

fn named() -> Scoring {
    let mut scoring = Scoring::new();
    scoring.record_local_name(1, ("Ann", "AN"), &tables(), true);
    scoring.names[2] = "Bob".into();
    scoring.known[2] = true;
    scoring
}

#[test]
fn line_files_skip_blanks_and_comments_and_fall_back_to_the_built_in_line() {
    let table = LineTable::parse(
        Some("  # comment\r\n\r\n  first\r\nsecond\n"),
        10,
        "built-in",
    );
    assert_eq!(table.0, vec!["first", "second"]);
    assert_eq!(LineTable::parse(None, 10, "built-in").0, vec!["built-in"]);
    assert_eq!(LineTable::parse(Some("a\nb\nc"), 2, "").0, vec!["a", "b"]);
}

/// The host reads its frag limit from `Host.txt`'s first line.
#[test]
fn the_hosts_frag_limit_comes_from_host_txt_clamped() {
    assert_eq!(named().frag_limit, 60);
    for (line, limit) in [("10", 50), ("9000", 5000), ("70000", 4464), ("x", 0)] {
        let mut scoring = Scoring::new();
        let mut t = tables();
        t.host = LineTable(vec![line.into()]);
        scoring.record_local_name(1, ("Ann", "AN"), &t, true);
        assert_eq!(scoring.frag_limit, limit, "{line}");
    }
}

#[test]
fn a_hit_moves_points_toward_the_attacker() {
    let mut scoring = named();
    scoring.table[2].score = 5;
    // Behind: two points, taken from the victim.
    assert!(scoring.report_hit(2, 1, true).is_empty());
    assert_eq!((scoring.table[1].score, scoring.table[2].score), (2, 3));
    assert_eq!(scoring.killer, 1);
    assert_eq!(scoring.killer_hold_us, KILLER_HOLD_US);
    // A client reports the hit instead.
    assert_eq!(
        scoring.report_hit(1, 2, false),
        vec![ScoringEffect::SendKillReport {
            victim: 1,
            attacker: 2
        }]
    );
}

/// A kill scores 20 (30 when behind); the frag limit names the winner and
/// freezes the scores.
#[test]
fn kills_score_and_the_limit_names_a_winner() {
    let mut scoring = named();
    scoring.table[2].score = 10;
    let effects = scoring.announce_kill(2, 1, &tables(), 0, true);
    assert_eq!(
        effects,
        vec![ScoringEffect::SendLine("Bob was fragged by Ann".into())]
    );
    assert_eq!(
        scoring.table[1],
        ScoreRow {
            kills: 1,
            deaths: 0,
            score: 30
        }
    );
    assert_eq!(scoring.table[2].deaths, 1);
    scoring.table[1].score = 50;
    let effects = scoring.announce_kill(2, 1, &tables(), 0, true);
    assert!(effects.contains(&ScoringEffect::WinnerFlash));
    assert_eq!(scoring.winner, 1);
    assert!(scoring.announce_kill(2, 1, &tables(), 0, true).is_empty());
}

#[test]
fn a_death_soon_after_a_hit_credits_the_held_attacker() {
    let mut scoring = named();
    scoring.report_hit(1, 2, false);
    scoring.age_killer(500_000);
    let effects = scoring.announce_kill(1, 0, &tables(), 0, false);
    assert_eq!(scoring.last_killer, 2);
    assert_eq!(effects[0], ScoringEffect::SendScoreWord(1 | 2 << 16));
    scoring.age_killer(500_000);
    scoring.announce_kill(1, 0, &tables(), 0, false);
    assert_eq!(scoring.last_killer, 0);
    assert_eq!(
        scoring.lines.iter().filter(|l| l.1 == "Ann died").count(),
        1
    );
}

#[test]
fn the_host_scores_a_clients_counter_once() {
    let mut scoring = named();
    scoring.apply_score_word(2, 1 | 1 << 16);
    scoring.apply_score_word(2, 1 | 1 << 16);
    assert_eq!(scoring.table[1].kills, 1);
    assert_eq!(scoring.table[2].deaths, 1);
}

#[test]
fn names_fill_lines_and_strangers_stay_unnamed() {
    let scoring = named();
    assert_eq!(
        scoring.format("@1 met @2 and @3", 1, 7),
        "Ann met a stranger and 3"
    );
}

#[test]
fn four_lines_live_seven_and_a_half_seconds() {
    let mut scoring = Scoring::new();
    for text in ["a", "b", "c", "d", "e"] {
        scoring.add_line(text);
    }
    assert_eq!(scoring.age_lines(0), vec!["b", "c", "d", "e"]);
    assert!(scoring.age_lines(LINE_LIFETIME_US).is_empty());
}

#[test]
fn the_winner_line_repeats_every_150_ticks() {
    let mut scoring = named();
    scoring.winner = 1;
    assert_eq!(
        scoring.host_announcement(300, &tables(), 0),
        Some("Ann wins".into())
    );
    assert_eq!(scoring.host_announcement(310, &tables(), 0), None);
    assert!(scoring.host_announcement(450, &tables(), 0).is_some());
}
