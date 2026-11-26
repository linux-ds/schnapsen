//! Aggregates for complete simulated matches, grouped by strategy slot.

use std::fmt;
use std::time::Duration;

use serde::Serialize;

use super::simulation::RunConfig;
use crate::{
    ai,
    core::{
        game::{FinishReason, HandResult, Player},
        match_play::MatchResult,
        rules::Difficulty,
    },
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Scores {
    pub matches_won: u64,
    pub matches_lost: u64,
    pub hands_won: u64,
    pub hands_lost: u64,
    pub game_points: u64,
    pub card_points: u64,
    pub schneider_matches: u64,
    pub closures_won: u64,
    pub closures_lost: u64,
    pub announcements: u64,
    pub exchanges: u64,
    pub decisions: u64,
    pub invalid_actions: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SideReport {
    pub difficulty: Difficulty,
    pub strategy_id: &'static str,
    pub counts: Scores,
    pub match_win_rate: f64,
    pub hand_win_rate: f64,
    pub mean_card_points: f64,
    pub total_decision_us: f64,
    pub mean_decision_us: f64,
    pub max_decision_us: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct FinishCounts {
    pub reached_66: u64,
    pub last_trick: u64,
    pub successful_closure: u64,
    pub failed_closure: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct PairedResults {
    pub left_sweeps: u64,
    pub splits: u64,
    pub right_sweeps: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Report {
    pub config: RunConfig,
    pub matches: u64,
    pub hands: u64,
    pub paired_match_results: Option<PairedResults>,
    pub finish_reasons: FinishCounts,
    pub invalid_actions: u64,
    pub inconsistent_states: u64,
    pub duration_ms: f64,
    pub left: SideReport,
    pub right: SideReport,
}

impl Report {
    /// Keep the Auftrag 25 JSONL entry contract for existing consumers.
    pub fn legacy_json(&self) -> serde_json::Value {
        let side = |side: &SideReport| {
            serde_json::json!({
                "difficulty": side.difficulty,
                "stats": {
                    "counts": side.counts,
                    "mean_card_points": side.mean_card_points,
                    "mean_decision_us": side.mean_decision_us,
                    "max_decision_us": side.max_decision_us,
                }
            })
        };
        serde_json::json!({
            "profile": self.config.setup.profile,
            "first_seed": self.config.first_seed,
            "last_seed": self.config.first_seed + (self.config.seed_count - 1),
            "matches": self.matches,
            "hands": self.hands,
            "paired_match_results": self.paired_match_results,
            "left": side(&self.left),
            "right": side(&self.right),
        })
    }
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "{:?} / {:?} / {:?}: {} matches, {} hands, {:.1} ms",
            self.config.setup.variant,
            self.config.setup.profile,
            self.config.setup.schneider,
            self.matches,
            self.hands,
            self.duration_ms,
        )?;
        writeln!(
            f,
            "Finish reasons: 66={}, last trick={}, successful closure={}, failed closure={}",
            self.finish_reasons.reached_66,
            self.finish_reasons.last_trick,
            self.finish_reasons.successful_closure,
            self.finish_reasons.failed_closure,
        )?;
        writeln!(
            f,
            "Invalid actions: {}, inconsistent states: {}",
            self.invalid_actions, self.inconsistent_states,
        )?;
        for (slot, side) in [("Left", &self.left), ("Right", &self.right)] {
            let score = side.counts;
            writeln!(
                f,
                "{slot} ({}): matches {} won / {} lost ({:.1}%), hands {} won / {} lost ({:.1}%)",
                side.difficulty.id(),
                score.matches_won,
                score.matches_lost,
                side.match_win_rate * 100.0,
                score.hands_won,
                score.hands_lost,
                side.hand_win_rate * 100.0
            )?;
            writeln!(
                f,
                "  {} game points, {:.2} card points/hand, {} Schneider, {} announcements, {} exchanges, {} won / {} lost closures",
                score.game_points,
                side.mean_card_points,
                score.schneider_matches,
                score.announcements,
                score.exchanges,
                score.closures_won,
                score.closures_lost,
            )?;
            writeln!(
                f,
                "  {} decisions, {:.2} ms total, {:.2} µs mean, {:.2} µs max",
                score.decisions,
                side.total_decision_us / 1000.0,
                side.mean_decision_us,
                side.max_decision_us,
            )?;
        }
        if let Some(pairs) = self.paired_match_results {
            write!(
                f,
                "Mirrored pairs: {} left sweeps / {} splits / {} right sweeps",
                pairs.left_sweeps, pairs.splits, pairs.right_sweeps,
            )?;
        }
        Ok(())
    }
}

#[derive(Default)]
pub(super) struct Side {
    pub(super) scores: Scores,
    decision_time_ns: u128,
    slowest_decision_ns: u128,
}

#[derive(Default)]
pub(super) struct Totals {
    pub(super) matches: u64,
    hands: u64,
    pairs: [u64; 3],
    finish_reasons: FinishCounts,
    pub(super) sides: [Side; 2],
}

fn quotient(value: u64, base: u64) -> f64 {
    if base == 0 {
        0.0
    } else {
        value as f64 / base as f64
    }
}

impl Side {
    pub(super) fn decision(&mut self, elapsed: Duration) {
        let nanos = elapsed.as_nanos();
        self.scores.decisions += 1;
        self.decision_time_ns += nanos;
        self.slowest_decision_ns = self.slowest_decision_ns.max(nanos);
    }

    fn report(&self, difficulty: Difficulty, hands: u64, matches: u64) -> SideReport {
        let counts = self.scores;
        SideReport {
            difficulty,
            strategy_id: ai::strategy_id(difficulty),
            counts,
            match_win_rate: quotient(counts.matches_won, matches),
            hand_win_rate: quotient(counts.hands_won, hands),
            mean_card_points: quotient(counts.card_points, hands),
            total_decision_us: self.decision_time_ns as f64 / 1000.0,
            mean_decision_us: if counts.decisions == 0 {
                0.0
            } else {
                self.decision_time_ns as f64 / counts.decisions as f64 / 1000.0
            },
            max_decision_us: self.slowest_decision_ns as f64 / 1000.0,
        }
    }
}

impl Totals {
    pub(super) fn record_hand(&mut self, result: HandResult, seats: [usize; 2]) {
        self.hands += 1;
        let outcome = result.outcome;
        let finish = match outcome.reason {
            FinishReason::Reached66 => &mut self.finish_reasons.reached_66,
            FinishReason::LastTrick => &mut self.finish_reasons.last_trick,
            FinishReason::SuccessfulClosure => &mut self.finish_reasons.successful_closure,
            FinishReason::FailedClosure => &mut self.finish_reasons.failed_closure,
        };
        *finish += 1;
        for player in [Player::Local, Player::Opponent] {
            let counts = &mut self.sides[seats[player as usize]].scores;
            counts.card_points += u64::from(result.score(player).eyes);
            if outcome.winner == player {
                counts.hands_won += 1;
                counts.game_points += u64::from(outcome.game_points);
            } else {
                counts.hands_lost += 1;
            }
        }
        if let Some(closure) = outcome.closure {
            let closer = &mut self.sides[seats[closure.closer as usize]].scores;
            if outcome.reason == FinishReason::SuccessfulClosure {
                closer.closures_won += 1;
            } else {
                closer.closures_lost += 1;
            }
        }
    }

    pub(super) fn record_match(&mut self, result: MatchResult, seats: [usize; 2]) {
        self.matches += 1;
        let winner = &mut self.sides[seats[result.winner as usize]].scores;
        winner.matches_won += 1;
        if result.awarded_bummerls == 2 {
            winner.schneider_matches += 1;
        }
        self.sides[seats[result.winner.other() as usize]]
            .scores
            .matches_lost += 1;
    }

    pub(super) fn record_pair(&mut self, left_wins: u64) {
        self.pairs[left_wins as usize] += 1;
    }

    pub(super) fn report(&self, config: RunConfig, elapsed: Duration) -> Report {
        Report {
            config,
            matches: self.matches,
            hands: self.hands,
            paired_match_results: config.mirror.then_some(PairedResults {
                left_sweeps: self.pairs[2],
                splits: self.pairs[1],
                right_sweeps: self.pairs[0],
            }),
            left: self.sides[0].report(config.setup.strategies[0], self.hands, self.matches),
            right: self.sides[1].report(config.setup.strategies[1], self.hands, self.matches),
            finish_reasons: self.finish_reasons,
            invalid_actions: self
                .sides
                .iter()
                .map(|item| item.scores.invalid_actions)
                .sum(),
            inconsistent_states: 0,
            duration_ms: elapsed.as_secs_f64() * 1000.0,
        }
    }
}
