use serde::{Deserialize, Serialize};

use super::{
    cards::{Card, Suit},
    game::{
        Announcement, CloseError, Exchange, ExchangeError, Game, MarriageError, Outcome, Phase,
        PlayError, Player, ResolveError, Variant,
    },
    observation::Observation,
    rules::MatchConfig,
};

pub use super::rules::SchneiderRule;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MatchResult {
    pub winner: Player,
    pub awarded_bummerls: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DealError {
    HandInProgress,
    MatchFinished,
    MatchInProgress,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Match {
    hand: Game,
    forehand: Player,
    remaining: [u8; 2],
    completed_hands: u8,
    hand_accounted: bool,
    result: Option<MatchResult>,
    bummerls: [u32; 2],
    config: MatchConfig,
}

impl Match {
    pub fn new(
        forehand: Player,
        variant: Variant,
        schneider: SchneiderRule,
        choose_index: impl FnMut(usize) -> usize,
    ) -> Self {
        Self::with_config(
            forehand,
            MatchConfig {
                profile: variant,
                schneider,
                ..MatchConfig::default()
            },
            choose_index,
        )
    }

    pub fn with_config(
        forehand: Player,
        config: MatchConfig,
        choose_index: impl FnMut(usize) -> usize,
    ) -> Self {
        Self {
            hand: Game::with_rules(forehand, config.variant, config.profile, choose_index),
            forehand,
            remaining: [config.variant.start_points(); 2],
            completed_hands: 0,
            hand_accounted: false,
            result: None,
            bummerls: [0; 2],
            config,
        }
    }

    pub fn observation(&self, player: Player) -> Observation {
        self.hand
            .observation(player, self.forehand, self.remaining, self.config)
    }

    pub fn hand(&self) -> &Game {
        &self.hand
    }
    pub fn forehand(&self) -> Player {
        self.forehand
    }
    pub fn variant(&self) -> Variant {
        self.config.profile
    }
    pub fn schneider(&self) -> SchneiderRule {
        self.config.schneider
    }
    pub fn config(&self) -> MatchConfig {
        self.config
    }

    pub fn remaining(&self, player: Player) -> u8 {
        self.remaining[player as usize]
    }
    pub fn completed_hands(&self) -> u8 {
        self.completed_hands
    }
    pub fn result(&self) -> Option<MatchResult> {
        self.result
    }
    pub fn bummerls(&self, player: Player) -> u32 {
        self.bummerls[player as usize]
    }

    /// Checks a decoded save before it is used for further play.
    pub fn validate_snapshot(&self) -> bool {
        let start = self.config.variant.start_points();
        if self.hand.game_variant() != self.config.variant
            || self.hand.variant() != self.config.profile
            || !self
                .config
                .variant
                .profiles()
                .contains(&self.config.profile)
            || self.remaining.iter().any(|&points| points > start)
            || !self.hand.validate_snapshot(self.forehand)
        {
            return false;
        }

        let finished = match self.hand.phase() {
            Phase::Finished(outcome) => Some(outcome),
            _ => None,
        };
        if self.hand_accounted != finished.is_some() {
            return false;
        }
        match self.result {
            Some(result) => {
                let loser_points = self.remaining(result.winner.other());
                if !finished.is_some_and(|outcome| outcome.winner == result.winner)
                    || self.remaining(result.winner) != 0
                    || loser_points == 0
                    || result.awarded_bummerls
                        != self.config.schneider.bummerls(loser_points, start)
                {
                    return false;
                }
            }
            None if self.remaining.contains(&0) => return false,
            None => {}
        }

        let spent = self.remaining.map(|points| start - points);
        let Some(outcome) = finished else {
            return plausible_hand_count(spent, self.completed_hands);
        };
        let Some(previous_hands) = self.completed_hands.checked_sub(1) else {
            return false;
        };
        let winner = outcome.winner as usize;
        // A hand ending the match may spend fewer points than awarded.
        let min_charge = if self.result.is_some() {
            1
        } else {
            outcome.game_points
        };
        (min_charge..=outcome.game_points).any(|charge| {
            let Some(before) = spent[winner].checked_sub(charge) else {
                return false;
            };
            let mut previous_spent = spent;
            previous_spent[winner] = before;
            plausible_hand_count(previous_spent, previous_hands)
        })
    }

    pub fn reset_bummerls(&mut self) {
        self.bummerls = [0; 2];
    }

    pub fn next_hand(&mut self, choose_index: impl FnMut(usize) -> usize) -> Result<(), DealError> {
        if self.result.is_some() {
            return Err(DealError::MatchFinished);
        }
        if !self.hand_accounted {
            return Err(DealError::HandInProgress);
        }
        self.forehand = self.forehand.other();
        self.hand = Game::with_rules(
            self.forehand,
            self.config.variant,
            self.config.profile,
            choose_index,
        );
        self.hand_accounted = false;
        Ok(())
    }

    pub fn new_match(
        &mut self,
        variant: Variant,
        schneider: SchneiderRule,
        choose_index: impl FnMut(usize) -> usize,
    ) -> Result<(), DealError> {
        self.new_match_with_config(
            MatchConfig {
                profile: variant,
                schneider,
                ..self.config
            },
            choose_index,
        )
    }

    pub fn new_match_with_config(
        &mut self,
        config: MatchConfig,
        choose_index: impl FnMut(usize) -> usize,
    ) -> Result<(), DealError> {
        if self.result.is_none() {
            return Err(DealError::MatchInProgress);
        }
        let mut next = Self::with_config(self.forehand.other(), config, choose_index);
        next.bummerls = self.bummerls;
        *self = next;
        Ok(())
    }

    pub fn play_card(&mut self, player: Player, card: Card) -> Result<(), PlayError> {
        self.hand.play_card(player, card)
    }

    pub fn announce_and_play(
        &mut self,
        player: Player,
        suit: Suit,
        card: Card,
    ) -> Result<Announcement, MarriageError> {
        let announcement = self.hand.announce_and_play(player, suit, card)?;
        self.account_finished_hand();
        Ok(announcement)
    }

    pub fn exchange_trump(&mut self, player: Player) -> Result<Exchange, ExchangeError> {
        self.hand.exchange_trump(player)
    }

    pub fn close_talon(&mut self, player: Player) -> Result<(), CloseError> {
        self.hand.close_talon(player)
    }

    pub fn resolve_trick(&mut self) -> Result<Player, ResolveError> {
        let winner = self.hand.resolve_trick()?;
        self.account_finished_hand();
        Ok(winner)
    }

    fn account_finished_hand(&mut self) {
        if let Some(result) = self.hand.result() {
            self.account_outcome(result.outcome);
        }
    }

    fn account_outcome(&mut self, outcome: Outcome) {
        if self.hand_accounted || self.result.is_some() {
            return;
        }
        self.hand_accounted = true;
        self.completed_hands += 1;
        let winner = outcome.winner;
        let remaining = &mut self.remaining[winner as usize];
        *remaining = remaining.saturating_sub(outcome.game_points);
        if *remaining == 0 {
            let loser = winner.other() as usize;
            let awarded_bummerls = self
                .config
                .schneider
                .bummerls(self.remaining[loser], self.config.variant.start_points());
            self.bummerls[loser] = self.bummerls[loser].saturating_add(u32::from(awarded_bummerls));
            self.result = Some(MatchResult {
                winner,
                awarded_bummerls,
            });
        }
    }
}

fn plausible_hand_count(spent: [u8; 2], hands: u8) -> bool {
    let minimum: u16 = spent
        .iter()
        .map(|points| u16::from(points.div_ceil(3)))
        .sum();
    let maximum: u16 = spent.iter().copied().map(u16::from).sum();
    (minimum..=maximum).contains(&u16::from(hands))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::game::FinishReason;

    fn ordered(upper: usize) -> usize {
        upper - 1
    }

    fn session(rule: SchneiderRule) -> Match {
        Match::new(Player::Local, Variant::Soft, rule, ordered)
    }

    fn points(winner: Player, game_points: u8) -> Outcome {
        Outcome {
            winner,
            game_points,
            reason: FinishReason::Reached66,
            closure: None,
        }
    }

    #[test]
    fn snapshot_rejects_contradictory_match_accounting() {
        let game = session(SchneiderRule::Double);
        assert!(game.validate_snapshot());
        let mut forged = game.clone();
        forged.remaining[Player::Local as usize] = 6;
        assert!(!forged.validate_snapshot());
        forged = game.clone();
        forged.hand_accounted = true;
        assert!(!forged.validate_snapshot());
        forged = game.clone();
        forged.config.profile = Variant::Sharp;
        assert!(!forged.validate_snapshot());
        forged = game;
        forged.result = Some(MatchResult {
            winner: Player::Local,
            awarded_bummerls: 2,
        });
        assert!(!forged.validate_snapshot());
    }

    #[test]
    fn match_configuration_changes_only_after_the_match_finishes() {
        use super::super::rules::{Difficulty, RuleProfile};

        let initial = MatchConfig::default();
        let mut selection = initial;
        let mut game = Match::with_config(Player::Local, selection, ordered);
        selection.profile = RuleProfile::Sharp;
        selection.schneider = SchneiderRule::Single;
        selection.difficulty = Difficulty::Hard;
        assert_eq!(
            game.new_match_with_config(selection, ordered),
            Err(DealError::MatchInProgress)
        );

        for _ in 0..2 {
            game.account_outcome(points(Player::Local, 3));
            assert_eq!(
                game.new_match_with_config(selection, ordered),
                Err(DealError::MatchInProgress)
            );
            game.next_hand(ordered).unwrap();
            assert_eq!(game.config(), initial);
            assert_eq!(game.hand().variant(), initial.profile);
            assert_eq!(game.hand().game_variant(), initial.variant);
        }

        game.account_outcome(points(Player::Local, 1));
        assert_eq!(game.bummerls(Player::Opponent), 2);
        game.new_match_with_config(selection, ordered).unwrap();
        assert_eq!(game.config(), selection);
        assert_eq!(game.hand().variant(), selection.profile);
        assert_eq!(game.hand().game_variant(), selection.variant);
        assert_eq!(game.bummerls(Player::Opponent), 2);
    }

    #[test]
    fn counts_each_hand_once_and_alternates_forehand_independent_of_winner() {
        let mut game = session(SchneiderRule::default());
        assert_eq!(game.remaining, [7, 7]);
        assert_eq!(game.completed_hands(), 0);
        assert_eq!(game.schneider(), SchneiderRule::Double);
        assert_eq!(game.next_hand(ordered), Err(DealError::HandInProgress));
        assert_eq!(
            game.new_match(Variant::Sharp, SchneiderRule::Single, ordered),
            Err(DealError::MatchInProgress)
        );
        for (index, (winner, score, expected)) in [
            (Player::Local, 2, [5, 7]),
            (Player::Opponent, 3, [5, 4]),
            (Player::Local, 1, [4, 4]),
        ]
        .into_iter()
        .enumerate()
        {
            let outcome = points(winner, score);
            game.account_outcome(outcome);
            game.account_outcome(outcome);
            assert_eq!(game.remaining, expected);
            assert_eq!(game.completed_hands(), index as u8 + 1);
            assert_eq!(game.bummerls, [0, 0]);
            let next = game.forehand().other();
            game.next_hand(ordered).unwrap();
            assert_eq!(game.forehand(), next);
            assert_eq!(game.hand().next_player(), Some(next));
            assert_eq!(game.remaining, expected);
        }
    }

    #[test]
    fn saturates_at_zero_and_awards_bummerls_only_at_match_end() {
        for (rule, scoring_loser, expected) in [
            (SchneiderRule::Double, false, 2),
            (SchneiderRule::Double, true, 1),
            (SchneiderRule::Single, false, 1),
        ] {
            let mut game = session(rule);
            if scoring_loser {
                game.account_outcome(points(Player::Local, 1));
                game.next_hand(ordered).unwrap();
            }
            for index in 0..3 {
                game.account_outcome(points(Player::Opponent, 3));
                if index != 2 {
                    assert!(game.result().is_none());
                    game.next_hand(ordered).unwrap();
                }
            }
            assert_eq!(game.remaining, [7 - u8::from(scoring_loser), 0]);
            let result = Some(MatchResult {
                winner: Player::Opponent,
                awarded_bummerls: expected,
            });
            assert_eq!(game.result(), result);
            assert_eq!(game.next_hand(ordered), Err(DealError::MatchFinished));
            game.account_outcome(points(Player::Local, 3));
            assert_eq!(game.result(), result);
            assert_eq!(game.bummerls, [u32::from(expected), 0]);
        }
    }

    #[test]
    fn preserves_cumulative_bummerls_until_explicit_reset() {
        let mut game = session(SchneiderRule::Double);
        for expected in [2, 3] {
            for _ in 0..2 {
                game.account_outcome(points(Player::Local, 3));
                game.next_hand(ordered).unwrap();
            }
            game.account_outcome(points(Player::Local, 2));
            assert_eq!(game.bummerls(Player::Opponent), expected);
            let next = game.forehand().other();
            game.new_match(Variant::Sharp, SchneiderRule::Single, ordered)
                .unwrap();
            assert_eq!(game.forehand(), next);
            assert_eq!(game.remaining, [7, 7]);
            assert_eq!(game.completed_hands(), 0);
            assert_eq!(game.variant(), Variant::Sharp);
            assert_eq!(game.schneider(), SchneiderRule::Single);
            assert_eq!(game.result(), None);
            assert_eq!(game.bummerls(Player::Opponent), expected);
        }
        game.reset_bummerls();
        assert_eq!(game.bummerls, [0, 0]);
        assert_eq!(game.remaining, [7, 7]);
    }

    #[test]
    fn automatic_finish_counts_once_across_multiple_hands() {
        let mut session = session(SchneiderRule::Double);
        let mut expected = [7_u8; 2];
        for completed in 1..=2 {
            while session.hand().result().is_none() {
                let player = session.hand().next_player().unwrap();
                let card = session.hand().legal_cards(player)[0];
                session.play_card(player, card).unwrap();
                if session.hand().view(player).played.len() == 2 {
                    session.resolve_trick().unwrap();
                }
            }
            let outcome = session.hand().result().unwrap().outcome;
            expected[outcome.winner as usize] -= outcome.game_points;
            assert_eq!(session.remaining, expected);
            assert!(session.resolve_trick().is_err());
            assert_eq!(session.completed_hands(), completed);
            let next_forehand = session.forehand().other();
            session.next_hand(ordered).unwrap();
            assert!(session.hand().result().is_none());
            assert_eq!(session.forehand(), next_forehand);
            assert_eq!(session.remaining, expected);
        }
    }

    #[test]
    fn automatic_marriage_finish_is_accounted_before_an_opponent_reply() {
        for seed in 0_u64..128 {
            let mut random = seed;
            let mut session = Match::new(
                Player::Local,
                Variant::Soft,
                SchneiderRule::Double,
                |upper| {
                    random = random.wrapping_mul(6364136223846793005).wrapping_add(1);
                    (random >> 32) as usize % upper
                },
            );
            while let Some(player) = session.hand().next_player() {
                if let Some(marriage) = session.hand().legal_marriages(player).first().copied() {
                    session
                        .announce_and_play(player, marriage.suit, marriage.king())
                        .unwrap();
                    if let Some(result) = session.hand().result() {
                        let outcome = result.outcome;
                        assert_eq!(outcome.reason, FinishReason::Reached66);
                        assert_eq!(outcome.winner, player);
                        assert_eq!(session.hand().view(player).played.len(), 1);
                        assert_eq!(session.remaining(player), 7 - outcome.game_points);
                        assert_eq!(session.remaining(player.other()), 7);
                        assert!(session.resolve_trick().is_err());
                        assert_eq!(session.completed_hands(), 1);
                        return;
                    }
                } else {
                    let card = session.hand().legal_cards(player)[0];
                    session.play_card(player, card).unwrap();
                }
                if session.hand().view(player).played.len() == 2 {
                    session.resolve_trick().unwrap();
                }
            }
        }
        panic!("test deals must include an automatic marriage finish");
    }

    #[test]
    fn real_closed_hand_results_are_accounted_without_revaluing_them() {
        let mut outcomes = [false; 2];
        for seed in 0_u64..64 {
            let mut random = seed;
            let mut session = Match::new(
                Player::Local,
                Variant::Soft,
                SchneiderRule::Double,
                |upper| {
                    random = random.wrapping_mul(6364136223846793005).wrapping_add(1);
                    (random >> 32) as usize % upper
                },
            );
            for player in [Player::Local, Player::Opponent] {
                let card = session.hand().legal_cards(player)[0];
                session.play_card(player, card).unwrap();
            }
            let closer = session.resolve_trick().unwrap();
            session.close_talon(closer).unwrap();
            while session.hand().result().is_none() {
                let player = session.hand().next_player().unwrap();
                let card = session.hand().legal_cards(player)[0];
                session.play_card(player, card).unwrap();
                if session.hand().view(Player::Local).played.len() == 2 {
                    session.resolve_trick().unwrap();
                }
            }
            let outcome = session.hand().result().unwrap().outcome;
            assert!(matches!(
                outcome.reason,
                FinishReason::SuccessfulClosure | FinishReason::FailedClosure
            ));
            outcomes[usize::from(outcome.reason == FinishReason::SuccessfulClosure)] = true;
            assert_eq!(session.remaining(outcome.winner), 7 - outcome.game_points);
            assert_eq!(session.remaining(outcome.winner.other()), 7);
            assert_eq!(session.completed_hands(), 1);
            assert!(session.resolve_trick().is_err());
            assert_eq!(session.completed_hands(), 1);
            if outcomes == [true, true] {
                break;
            }
        }
        assert_eq!(outcomes, [true, true]);
    }
}
