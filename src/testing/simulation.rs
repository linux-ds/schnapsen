//! Deterministic execution of complete matches through the production engine.

use std::{fmt, time::Instant};

use serde::{Deserialize, Serialize};

use super::{
    case::{CaseStart, RecordedAction, TestCase},
    statistics::{Report, Totals},
    validation::{validate, validate_transition},
};
use crate::{
    ai,
    core::{
        game::Player,
        match_play::Match,
        rules::{Difficulty, GameVariant, MatchConfig, RuleProfile, SchneiderRule},
    },
};

pub const PAIRINGS: [(Difficulty, Difficulty); 6] = [
    (Difficulty::Easy, Difficulty::Medium),
    (Difficulty::Easy, Difficulty::Hard),
    (Difficulty::Medium, Difficulty::Hard),
    (Difficulty::Easy, Difficulty::Easy),
    (Difficulty::Medium, Difficulty::Medium),
    (Difficulty::Hard, Difficulty::Hard),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatchSetup {
    pub variant: GameVariant,
    pub profile: RuleProfile,
    pub schneider: SchneiderRule,
    /// Strategy slots, in seat order before an optional mirror.
    pub strategies: [Difficulty; 2],
    pub forehand: Player,
}

impl Default for MatchSetup {
    fn default() -> Self {
        let config = MatchConfig::default();
        Self {
            variant: config.variant,
            profile: config.profile,
            schneider: config.schneider,
            strategies: [config.difficulty; 2],
            forehand: Player::Local,
        }
    }
}

impl MatchSetup {
    pub fn for_seats(self, mirrored: bool) -> Self {
        let mut result = self;
        if mirrored {
            result.strategies.swap(0, 1);
        }
        result
    }

    pub fn strategies_in_seat_order(self, mirrored: bool) -> [Difficulty; 2] {
        self.for_seats(mirrored).strategies
    }

    pub fn engine_config(self, mirrored: bool) -> MatchConfig {
        MatchConfig {
            variant: self.variant,
            profile: self.profile,
            schneider: self.schneider,
            difficulty: self.strategies_in_seat_order(mirrored)[Player::Opponent as usize],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunConfig {
    pub setup: MatchSetup,
    pub first_seed: u64,
    pub seed_count: u64,
    pub mirror: bool,
    pub validate_each_step: bool,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            setup: MatchSetup::default(),
            first_seed: 1,
            seed_count: 1,
            mirror: false,
            validate_each_step: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureKind {
    InvalidConfiguration,
    InvalidAction,
    InconsistentState,
    StepLimitExceeded,
}

#[derive(Debug, Serialize)]
pub struct Failure {
    pub setup: MatchSetup,
    pub seed: u64,
    pub mirrored: bool,
    pub step: usize,
    pub hand: usize,
    pub kind: FailureKind,
    pub invalid_actions: u64,
    pub inconsistent_states: u64,
    pub reason: String,
    pub failed_action: Option<RecordedAction>,
    pub state: Option<Match>,
    pub case: Option<TestCase>,
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?} seed={} mirrored={} hand={} action={}: {:?}: {}",
            self.setup,
            self.seed,
            self.mirrored,
            self.hand,
            self.step,
            self.failed_action,
            self.reason
        )
    }
}

impl std::error::Error for Failure {}

pub struct Completed {
    pub state: Match,
    pub case: TestCase,
    pub report: Report,
}

// SplitMix64. Each match has its own stream; mirrored seats get exactly the
// same sequence of deals, even when their plays and drawing order differ.
pub fn shuffle(seed: u64) -> impl FnMut(usize) -> usize {
    let mut state = seed;
    move |upper| {
        state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        ((value ^ (value >> 31)) % upper as u64) as usize
    }
}

fn configuration_error(setup: MatchSetup, seed: u64, reason: &str) -> Box<Failure> {
    Box::new(Failure {
        setup,
        seed,
        mirrored: false,
        step: 0,
        hand: 0,
        kind: FailureKind::InvalidConfiguration,
        invalid_actions: 0,
        inconsistent_states: 0,
        reason: reason.to_owned(),
        failed_action: None,
        state: None,
        case: None,
    })
}

fn check_setup(setup: MatchSetup, seed: u64) -> Result<(), Box<Failure>> {
    if setup.variant.profiles().contains(&setup.profile) {
        Ok(())
    } else {
        Err(configuration_error(setup, seed, "unsupported_profile"))
    }
}

struct MatchRun {
    setup: MatchSetup,
    seed: u64,
    mirrored: bool,
    state: Match,
    actions: Vec<RecordedAction>,
}

impl MatchRun {
    fn case(&self) -> TestCase {
        TestCase::new(CaseStart::Seed {
            setup: self.setup.for_seats(self.mirrored),
            seed: self.seed,
        })
    }

    fn failure(
        self,
        hand: usize,
        kind: FailureKind,
        reason: String,
        failed_action: Option<RecordedAction>,
    ) -> Box<Failure> {
        let mut case = self.case();
        case.description = Some(reason.clone());
        let step = self.actions.len()
            + usize::from(failed_action.is_some() || kind == FailureKind::InvalidAction);
        case.actions = self.actions;
        if let Some(action) = &failed_action {
            case.actions.push(*action);
        }
        Box::new(Failure {
            setup: self.setup,
            seed: self.seed,
            mirrored: self.mirrored,
            step,
            hand,
            kind,
            invalid_actions: u64::from(kind == FailureKind::InvalidAction),
            inconsistent_states: u64::from(kind == FailureKind::InconsistentState),
            reason,
            failed_action,
            state: Some(self.state),
            case: Some(case),
        })
    }
}

fn record_match(
    setup: MatchSetup,
    seed: u64,
    mirrored: bool,
    validate_each_step: bool,
    totals: &mut Totals,
) -> Result<MatchRun, Box<Failure>> {
    let mut shuffle = shuffle(seed);
    let mut run = MatchRun {
        setup,
        seed,
        mirrored,
        state: Match::with_config(setup.forehand, setup.engine_config(mirrored), &mut shuffle),
        actions: Vec::new(),
    };
    let seats = if mirrored { [1, 0] } else { [0, 1] };
    let mut hand = 1;
    if validate_each_step {
        if let Err(error) = validate(&run.state) {
            return Err(run.failure(
                hand,
                FailureKind::InconsistentState,
                error.to_string(),
                None,
            ));
        }
    }

    // At most 13 hands, each with fewer than 40 actions, under the current
    // rules. The limit also catches a strategy that cannot make progress.
    for _ in 0..520 {
        let next = if let Some(result) = run.state.hand().result() {
            totals.record_hand(result, seats);
            if let Some(match_result) = run.state.result() {
                totals.record_match(match_result, seats);
                return Ok(run);
            }
            hand += 1;
            RecordedAction::NextHand
        } else if let Some(player) = run.state.hand().next_player() {
            let slot = seats[player as usize];
            let side = &mut totals.sides[slot];
            let started = Instant::now();
            let action = ai::decide(setup.strategies[slot], &run.state.observation(player));
            side.decision(started.elapsed());
            let Some(action) = action else {
                side.scores.invalid_actions += 1;
                return Err(run.failure(
                    hand,
                    FailureKind::InvalidAction,
                    format!("{player:?}: no_action"),
                    None,
                ));
            };
            RecordedAction::from_ai(player, action)
        } else {
            RecordedAction::ResolveTrick
        };

        let before = validate_each_step.then(|| run.state.clone());
        if let Err(error) = next.apply(&mut run.state, &mut shuffle) {
            return Err(run.failure(hand, FailureKind::InvalidAction, error, Some(next)));
        }
        if let Some(before) = before {
            if let Err(error) = validate_transition(&before, &next, &run.state) {
                return Err(run.failure(
                    hand,
                    FailureKind::InconsistentState,
                    error.to_string(),
                    Some(next),
                ));
            }
        }
        match next {
            RecordedAction::Announce { player, .. } => {
                totals.sides[seats[player as usize]].scores.announcements += 1;
            }
            RecordedAction::Exchange { player } => {
                totals.sides[seats[player as usize]].scores.exchanges += 1;
            }
            _ => {}
        }
        run.actions.push(next);
    }

    Err(run.failure(
        hand,
        FailureKind::StepLimitExceeded,
        "step_limit_exceeded".to_owned(),
        None,
    ))
}

pub fn play_one(
    setup: MatchSetup,
    seed: u64,
    mirrored: bool,
    validate_each_step: bool,
) -> Result<Completed, Box<Failure>> {
    check_setup(setup, seed)?;
    let mut totals = Totals::default();
    let started = Instant::now();
    let run = record_match(setup, seed, mirrored, validate_each_step, &mut totals)?;
    let mut case = run.case();
    case.actions = run.actions;
    case.expected_end = Some(Box::new(run.state.clone()));
    let config = RunConfig {
        setup,
        first_seed: seed,
        seed_count: 1,
        mirror: false,
        validate_each_step,
    };
    Ok(Completed {
        state: run.state,
        case,
        report: totals.report(config, started.elapsed()),
    })
}

pub fn simulate(config: &RunConfig) -> Result<Report, Box<Failure>> {
    let RunConfig {
        setup,
        first_seed,
        seed_count,
        mirror,
        validate_each_step,
    } = *config;
    check_setup(setup, first_seed)?;
    let last_seed = seed_count
        .checked_sub(1)
        .and_then(|span| first_seed.checked_add(span))
        .ok_or_else(|| configuration_error(setup, first_seed, "invalid_seed_range"))?;
    let mut totals = Totals::default();
    let started = Instant::now();
    for seed in first_seed..=last_seed {
        let previous = totals.sides[0].scores.matches_won;
        record_match(setup, seed, false, validate_each_step, &mut totals)?;
        if mirror {
            record_match(setup, seed, true, validate_each_step, &mut totals)?;
            totals.record_pair(totals.sides[0].scores.matches_won - previous);
        }
    }
    Ok(totals.report(*config, started.elapsed()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_mirrored_matches_are_legal_reproducible_and_balanced_in_controls() {
        for profile in [RuleProfile::Soft, RuleProfile::Sharp] {
            for (left, right) in PAIRINGS {
                let config = RunConfig {
                    setup: MatchSetup {
                        profile,
                        strategies: [left, right],
                        ..MatchSetup::default()
                    },
                    first_seed: 8,
                    mirror: true,
                    ..RunConfig::default()
                };
                let scores = |report: Report| {
                    assert_eq!(report.matches, 2);
                    assert!(report.hands >= 6);
                    let scores = [report.left.counts, report.right.counts];
                    assert_eq!(scores.iter().map(|side| side.matches_won).sum::<u64>(), 2);
                    assert_eq!(
                        scores.iter().map(|side| side.hands_won).sum::<u64>(),
                        report.hands
                    );
                    assert_eq!(
                        scores.iter().map(|side| side.hands_lost).sum::<u64>(),
                        report.hands
                    );
                    assert_eq!(report.invalid_actions, 0);
                    assert_eq!(report.inconsistent_states, 0);
                    scores
                };
                let first = scores(simulate(&config).unwrap());
                let replay = scores(simulate(&config).unwrap());
                assert_eq!(first, replay);
                if left == right {
                    assert_eq!(first[0], first[1]);
                }
            }
        }
    }

    #[test]
    fn reports_keep_strategy_slots_distinct_from_seats() {
        let setup = MatchSetup {
            strategies: [Difficulty::Easy, Difficulty::Hard],
            forehand: Player::Opponent,
            schneider: SchneiderRule::Single,
            ..MatchSetup::default()
        };
        let mirrored = play_one(setup, 2, true, true).unwrap();
        assert_eq!(mirrored.state.config().difficulty, Difficulty::Easy);
        assert_eq!(mirrored.case.replay().unwrap(), mirrored.state);
        assert_eq!(mirrored.report.left.difficulty, Difficulty::Easy);
        assert_eq!(mirrored.report.right.difficulty, Difficulty::Hard);
        assert_eq!(mirrored.report.matches, 1);
        assert_eq!(
            mirrored.report.left.counts.schneider_matches
                + mirrored.report.right.counts.schneider_matches,
            0
        );
        let counts = mirrored.report.finish_reasons;
        assert_eq!(
            counts.reached_66
                + counts.last_trick
                + counts.successful_closure
                + counts.failed_closure,
            mirrored.report.hands
        );
    }

    #[test]
    fn rejects_overflowing_or_empty_seed_ranges_and_accepts_last_seed() {
        let mut config = RunConfig {
            first_seed: u64::MAX,
            seed_count: 0,
            ..RunConfig::default()
        };
        assert_eq!(
            simulate(&config).unwrap_err().kind,
            FailureKind::InvalidConfiguration
        );
        config.seed_count = 2;
        assert_eq!(
            simulate(&config).unwrap_err().kind,
            FailureKind::InvalidConfiguration
        );
        config.seed_count = 1;
        assert_eq!(simulate(&config).unwrap().matches, 1);
    }
}
