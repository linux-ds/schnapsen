//! Reproducible, headless comparison of the strategies used by the application.

use std::{env, process::ExitCode, time::Instant};

use schnapsen::{
    ai::{self, Action},
    core::{
        game::{FinishReason, Player},
        match_play::Match,
        rules::{Difficulty, MatchConfig, RuleProfile},
    },
};
use serde::Serialize;

const PLAYERS: [Player; 2] = [Player::Local, Player::Opponent];
const PAIRINGS: [(Difficulty, Difficulty); 6] = [
    (Difficulty::Easy, Difficulty::Medium),
    (Difficulty::Easy, Difficulty::Hard),
    (Difficulty::Medium, Difficulty::Hard),
    (Difficulty::Easy, Difficulty::Easy),
    (Difficulty::Medium, Difficulty::Medium),
    (Difficulty::Hard, Difficulty::Hard),
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
struct Scores {
    matches_won: u64,
    hands_won: u64,
    game_points: u64,
    card_points: u64,
    schneider_matches: u64,
    closures_won: u64,
    closures_lost: u64,
    announcements: u64,
    exchanges: u64,
    decisions: u64,
    invalid_actions: u64,
}

#[derive(Default)]
struct Side {
    scores: Scores,
    decision_time_ns: u128,
    slowest_decision_ns: u128,
}

impl Side {
    fn report(&self, hands: u64) -> serde_json::Value {
        serde_json::json!({
            "counts": self.scores,
            "mean_card_points": self.scores.card_points as f64 / hands as f64,
            "mean_decision_us": self.decision_time_ns as f64 / self.scores.decisions as f64 / 1000.0,
            "max_decision_us": self.slowest_decision_ns as f64 / 1000.0,
        })
    }
}

#[derive(Default)]
struct Totals {
    matches: u64,
    hands: u64,
    pairs_by_left_wins: [u64; 3],
    sides: [Side; 2],
}

// SplitMix64. Each match has its own stream; mirrored seats get exactly the
// same sequence of deals, even when their plays and drawing order differ.
fn shuffle(seed: u64) -> impl FnMut(usize) -> usize {
    let mut state = seed;
    move |upper| {
        state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        ((value ^ (value >> 31)) % upper as u64) as usize
    }
}

fn play_match(
    profile: RuleProfile,
    strategies: [Difficulty; 2],
    seats: [usize; 2],
    seed: u64,
    totals: &mut Totals,
) -> Result<(), String> {
    let mut shuffle = shuffle(seed);
    let mut session = Match::with_config(
        Player::Local,
        MatchConfig {
            profile,
            ..MatchConfig::default()
        },
        &mut shuffle,
    );

    // At most 13 hands to seven points and fewer than 40 steps per hand
    // with the current rules. Also catch strategies that fail to make progress.
    for _ in 0..520 {
        if let Some(result) = session.hand().result() {
            totals.hands += 1;
            let outcome = result.outcome;
            for player in PLAYERS {
                totals.sides[seats[player as usize]].scores.card_points +=
                    u64::from(result.score(player).eyes);
            }
            let winner = &mut totals.sides[seats[outcome.winner as usize]].scores;
            winner.hands_won += 1;
            winner.game_points += u64::from(outcome.game_points);
            if let Some(closure) = outcome.closure {
                let closer = &mut totals.sides[seats[closure.closer as usize]].scores;
                if outcome.reason == FinishReason::SuccessfulClosure {
                    closer.closures_won += 1;
                } else {
                    closer.closures_lost += 1;
                }
            }
            if let Some(result) = session.result() {
                totals.matches += 1;
                let winner = &mut totals.sides[seats[result.winner as usize]].scores;
                winner.matches_won += 1;
                if result.awarded_bummerls == 2 {
                    winner.schneider_matches += 1;
                }
                return Ok(());
            }
            session
                .next_hand(&mut shuffle)
                .map_err(|error| format!("next_hand: {error:?}"))?;
            continue;
        }

        let Some(player) = session.hand().next_player() else {
            session
                .resolve_trick()
                .map_err(|error| format!("resolve_trick: {error:?}"))?;
            continue;
        };
        let index = seats[player as usize];
        let side = &mut totals.sides[index];
        let start = Instant::now();
        let action = ai::decide(strategies[index], &session.observation(player));
        let elapsed = start.elapsed().as_nanos();
        side.decision_time_ns += elapsed;
        side.slowest_decision_ns = side.slowest_decision_ns.max(elapsed);
        side.scores.decisions += 1;
        let result = action
            .ok_or_else(|| "no_action".to_owned())
            .and_then(|action| {
                ai::apply(&mut session, player, action)
                    .map_err(|error| format!("{action:?}: {error:?}"))?;
                match action {
                    Action::Announce { .. } => side.scores.announcements += 1,
                    Action::Exchange => side.scores.exchanges += 1,
                    _ => {}
                }
                Ok(())
            });
        if let Err(error) = result {
            side.scores.invalid_actions += 1;
            return Err(format!(
                "{profile:?} seed={seed} seats={seats:?} strategy={:?} player={player:?} invalid_actions={} {error}",
                strategies[index], side.scores.invalid_actions
            ));
        }
    }

    Err(format!(
        "{profile:?} seed={seed} seats={seats:?}: step_limit_exceeded"
    ))
}

fn compare(
    profile: RuleProfile,
    strategies: [Difficulty; 2],
    seeds: std::ops::Range<u64>,
) -> Result<Totals, String> {
    let mut totals = Totals::default();
    for seed in seeds {
        let before = totals.sides[0].scores.matches_won;
        for seats in [[0, 1], [1, 0]] {
            play_match(profile, strategies, seats, seed, &mut totals)?;
        }
        let left_wins = totals.sides[0].scores.matches_won - before;
        totals.pairs_by_left_wins[left_wins as usize] += 1;
    }
    Ok(totals)
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let count = args
        .next()
        .map(|arg| arg.parse::<u64>())
        .transpose()
        .map_err(|_| "invalid_seed_count")?
        .unwrap_or(32);
    let first = args
        .next()
        .map(|arg| arg.parse::<u64>())
        .transpose()
        .map_err(|_| "invalid_first_seed")?
        .unwrap_or(1);
    let end = first.checked_add(count).ok_or("seed_range_overflow")?;
    if count == 0 {
        return Err("invalid_seed_count".into());
    }
    if args.next().is_some() {
        return Err("too_many_arguments".into());
    }

    for profile in [RuleProfile::Soft, RuleProfile::Sharp] {
        for (left, right) in PAIRINGS {
            let totals = compare(profile, [left, right], first..end)?;
            println!(
                "{}",
                serde_json::json!({
                    "profile": profile,
                    "first_seed": first,
                    "last_seed": end - 1,
                    "matches": totals.matches,
                    "hands": totals.hands,
                    "paired_match_results": {
                        "left_sweeps": totals.pairs_by_left_wins[2],
                        "splits": totals.pairs_by_left_wins[1],
                        "right_sweeps": totals.pairs_by_left_wins[0],
                    },
                    "left": {"difficulty": left, "stats": totals.sides[0].report(totals.hands)},
                    "right": {"difficulty": right, "stats": totals.sides[1].report(totals.hands)}
                })
            );
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{}", serde_json::json!({"error": error}));
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_mirrored_matches_are_legal_reproducible_and_balanced_in_controls() {
        for profile in [RuleProfile::Soft, RuleProfile::Sharp] {
            for (left, right) in PAIRINGS {
                let scores = |totals: Totals| {
                    assert_eq!(totals.matches, 2);
                    assert!(totals.hands >= 6);
                    let scores = totals.sides.map(|side| side.scores);
                    assert_eq!(scores.iter().map(|side| side.matches_won).sum::<u64>(), 2);
                    assert_eq!(
                        scores.iter().map(|side| side.hands_won).sum::<u64>(),
                        totals.hands
                    );
                    assert!(scores.iter().all(|side| side.invalid_actions == 0));
                    scores
                };
                let first = scores(compare(profile, [left, right], 8..9).unwrap());
                let replay = scores(compare(profile, [left, right], 8..9).unwrap());
                assert_eq!(first, replay);
                if left == right {
                    assert_eq!(first[0], first[1]);
                }
            }
        }
    }
}
