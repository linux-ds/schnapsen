use super::{
    cards::{Card, Suit},
    game::{
        Announcement, CloseError, DeclareError, Exchange, ExchangeError, Game, MarriageError,
        Outcome, PlayError, Player, ResolveError, Variant,
    },
};

const START_POINTS: u8 = 7;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SchneiderRule {
    #[default]
    Double,
    Single,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

#[derive(Debug, PartialEq, Eq)]
pub struct Match {
    hand: Game,
    forehand: Player,
    remaining: [u8; 2],
    completed_hands: u8,
    hand_accounted: bool,
    result: Option<MatchResult>,
    bummerls: [u32; 2],
    schneider: SchneiderRule,
}

impl Match {
    pub fn new(
        forehand: Player,
        variant: Variant,
        schneider: SchneiderRule,
        choose_index: impl FnMut(usize) -> usize,
    ) -> Self {
        Self {
            hand: Game::with_variant(forehand, variant, choose_index),
            forehand,
            remaining: [START_POINTS; 2],
            completed_hands: 0,
            hand_accounted: false,
            result: None,
            bummerls: [0; 2],
            schneider,
        }
    }

    pub fn hand(&self) -> &Game {
        &self.hand
    }
    pub fn forehand(&self) -> Player {
        self.forehand
    }
    pub fn variant(&self) -> Variant {
        self.hand.variant()
    }
    pub fn schneider(&self) -> SchneiderRule {
        self.schneider
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
        self.hand = Game::with_variant(self.forehand, self.variant(), choose_index);
        self.hand_accounted = false;
        Ok(())
    }

    pub fn new_match(
        &mut self,
        variant: Variant,
        schneider: SchneiderRule,
        choose_index: impl FnMut(usize) -> usize,
    ) -> Result<(), DealError> {
        if self.result.is_none() {
            return Err(DealError::MatchInProgress);
        }
        let mut next = Self::new(self.forehand.other(), variant, schneider, choose_index);
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
        self.hand.announce_and_play(player, suit, card)
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

    pub fn declare(&mut self, player: Player) -> Result<(), DeclareError> {
        self.hand.declare(player)?;
        self.account_finished_hand();
        Ok(())
    }

    pub fn decline_final_declaration(&mut self, player: Player) -> bool {
        let declined = self.hand.decline_final_declaration(player);
        if declined {
            self.account_finished_hand();
        }
        declined
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
            let awarded_bummerls = if self.schneider == SchneiderRule::Double
                && self.remaining[loser] == START_POINTS
            {
                2
            } else {
                1
            };
            self.bummerls[loser] += u32::from(awarded_bummerls);
            self.result = Some(MatchResult {
                winner,
                awarded_bummerls,
            });
        }
    }
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
            reason: FinishReason::Declared66,
            closure: None,
        }
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
    fn false_declaration_counts_once_and_allows_the_next_hand() {
        let mut session = session(SchneiderRule::Double);
        let lead = session.hand().legal_cards(Player::Local)[0];
        session.play_card(Player::Local, lead).unwrap();
        let reply = session.hand().legal_cards(Player::Opponent)[0];
        session.play_card(Player::Opponent, reply).unwrap();
        assert_eq!(session.resolve_trick(), Ok(Player::Local));
        session.declare(Player::Local).unwrap();
        assert_eq!(
            session.hand().result().unwrap().outcome.reason,
            FinishReason::FalseDeclaration
        );
        assert_eq!(session.remaining, [7, 4]);
        assert_eq!(
            session.declare(Player::Local),
            Err(DeclareError::HandFinished)
        );
        assert_eq!(session.completed_hands(), 1);
        session.next_hand(ordered).unwrap();
        assert!(session.hand().result().is_none());
        assert_eq!(session.forehand(), Player::Opponent);
        assert_eq!(session.remaining, [7, 4]);
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
                if let Some(player) = [Player::Local, Player::Opponent]
                    .into_iter()
                    .find(|&player| session.hand().can_declare(player))
                {
                    session.declare(player).unwrap();
                    break;
                }
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
            assert!(session.declare(outcome.winner).is_err());
            assert_eq!(session.completed_hands(), 1);
            if outcomes == [true, true] {
                break;
            }
        }
        assert_eq!(outcomes, [true, true]);
    }
}
