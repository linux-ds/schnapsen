use super::cards::{full_deck, Card, Rank, Suit};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Variant {
    #[default]
    Soft,
    Sharp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExchangeError {
    TalonNotOpen,
    NoOpenTrump,
    TrickStarted,
    CannotLead,
    MissingTrumpUnder,
    FirstTrickNotCompleted,
    NotEnoughTalonCards,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Exchange {
    pub player: Player,
    pub given: Card,
    pub received: Card,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarriageError {
    CannotLead,
    PairMissing,
    AlreadyAnnounced,
    NoTrickWon,
    InvalidLeadCard,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Marriage {
    pub suit: Suit,
    pub eyes: u8,
    pub may_play_ober: bool,
}

impl Marriage {
    pub fn king(self) -> Card {
        Card {
            suit: self.suit,
            rank: Rank::King,
        }
    }

    pub fn ober(self) -> Card {
        Card {
            suit: self.suit,
            rank: Rank::Ober,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Announcement {
    pub player: Player,
    pub marriage: Marriage,
    pub played: Card,
    trick_index: usize,
}

impl Announcement {
    pub fn partner(self) -> Card {
        if self.played == self.marriage.king() {
            self.marriage.ober()
        } else {
            self.marriage.king()
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EyeBreakdown {
    pub tricks: u8,
    pub effective_announcements: u8,
    pub pending_announcements: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Player {
    Local = 0,
    Opponent = 1,
}

impl Player {
    pub fn other(self) -> Self {
        match self {
            Self::Local => Self::Opponent,
            Self::Opponent => Self::Local,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    OpenTalon,
    EmptyTalon,
    ClosedTalon(TalonClosure),
    Finished(Outcome),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TalonClosure {
    pub closer: Player,
    pub opponent_at_closing: Score,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloseError {
    FirstTrickNotCompleted,
    TalonNotOpen,
    TrickStarted,
    CannotLead,
    NotEnoughTalonCards,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FinishReason {
    Declared66,
    FalseDeclaration,
    LastTrick,
    SuccessfulClosure,
    FailedClosure,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReviewedTrick {
    pub number: usize,
    pub winner: Player,
    pub cards: [Card; 2],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrickReviewError {
    Forbidden,
    Empty,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub winner: Player,
    pub reason: FinishReason,
    pub game_points: u8,
    pub closure: Option<TalonClosure>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Score {
    pub eyes: u8,
    pub tricks: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HandResult {
    pub outcome: Outcome,
    pub loser: Player,
    scores: [Score; 2],
}

impl HandResult {
    pub fn score(self, player: Player) -> Score {
        self.scores[player as usize]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeclareError {
    HandFinished,
    NotBetweenOwnTrickAndLead,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayError {
    UnsupportedPhase(Phase),
    TrickPending,
    OutOfTurn { expected: Player },
    CardNotInHand,
    MustBeat(Suit),
    MustFollow(Suit),
    MustTrump,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Reply {
    Beat,
    Follow,
    Trump,
    Discard,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResolveError {
    IncompleteTrick,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Game {
    hands: [Vec<Card>; 2],
    // Draw order, with the face-up trump card at the bottom.
    talon: Vec<Card>,
    trump_suit: Suit,
    leader: Player,
    played: Vec<(Player, Card)>,
    tricks: [Vec<[Card; 2]>; 2],
    trick_winners: Vec<Player>,
    variant: Variant,
    announcements: Vec<Announcement>,
    phase: Phase,
}

pub struct View<'a> {
    pub hand: &'a [Card],
    pub opponent_hand_count: usize,
    pub hidden_talon_count: usize,
    pub open_trump: Option<Card>,
    pub talon_closed: bool,
    pub played: &'a [(Player, Card)],
    pub own_trick_card_count: usize,
    pub opponent_trick_card_count: usize,
}

impl Game {
    /// `choose_index(upper)` must return a random index in `0..upper`.
    pub fn new(leader: Player, choose_index: impl FnMut(usize) -> usize) -> Self {
        Self::with_variant(leader, Variant::default(), choose_index)
    }

    pub fn with_variant(
        leader: Player,
        variant: Variant,
        mut choose_index: impl FnMut(usize) -> usize,
    ) -> Self {
        let mut deck = full_deck();
        for upper in (2..=deck.len()).rev() {
            deck[..upper].swap(upper - 1, choose_index(upper));
        }

        let mut hands = [Vec::with_capacity(5), Vec::with_capacity(5)];
        let forehand = leader as usize;
        let dealer = leader.other() as usize;
        hands[forehand].extend_from_slice(&deck[..3]);
        hands[dealer].extend_from_slice(&deck[3..6]);
        let trump = deck[6];
        hands[forehand].extend_from_slice(&deck[7..9]);
        hands[dealer].extend_from_slice(&deck[9..11]);
        let mut talon = deck[11..].to_vec();
        talon.push(trump);

        Self {
            hands,
            talon,
            trump_suit: trump.suit,
            leader,
            played: Vec::new(),
            tricks: [Vec::new(), Vec::new()],
            trick_winners: Vec::new(),
            variant,
            announcements: Vec::new(),
            phase: Phase::OpenTalon,
        }
    }

    pub fn trump_suit(&self) -> Suit {
        self.trump_suit
    }

    pub fn trump_under(&self) -> Card {
        Card {
            suit: self.trump_suit,
            rank: Rank::Unter,
        }
    }

    pub fn trump_exchange(&self, player: Player) -> Result<Exchange, ExchangeError> {
        self.check_talon_action(player)
            .map_err(|error| match error {
                CloseError::TalonNotOpen => ExchangeError::TalonNotOpen,
                CloseError::TrickStarted => ExchangeError::TrickStarted,
                CloseError::CannotLead => ExchangeError::CannotLead,
                CloseError::FirstTrickNotCompleted => ExchangeError::FirstTrickNotCompleted,
                CloseError::NotEnoughTalonCards => ExchangeError::NotEnoughTalonCards,
            })?;
        let received = self
            .talon
            .last()
            .copied()
            .ok_or(ExchangeError::NoOpenTrump)?;
        let given = self.trump_under();
        if !self.hands[player as usize].contains(&given) {
            return Err(ExchangeError::MissingTrumpUnder);
        }
        Ok(Exchange {
            player,
            given,
            received,
        })
    }

    pub fn exchange_trump(&mut self, player: Player) -> Result<Exchange, ExchangeError> {
        let exchange = self.trump_exchange(player)?;
        let held = self.hands[player as usize]
            .iter_mut()
            .find(|card| **card == exchange.given)
            .expect("the exchange was checked");
        *held = exchange.received;
        *self.talon.last_mut().expect("the exchange was checked") = exchange.given;
        Ok(exchange)
    }

    pub fn leader(&self) -> Player {
        self.leader
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn check_close_talon(&self, player: Player) -> Result<(), CloseError> {
        self.check_talon_action(player)
    }

    fn check_talon_action(&self, player: Player) -> Result<(), CloseError> {
        if self.phase != Phase::OpenTalon {
            return Err(CloseError::TalonNotOpen);
        }
        if !self.played.is_empty() {
            return Err(CloseError::TrickStarted);
        }
        if self.leader != player {
            return Err(CloseError::CannotLead);
        }
        if self.trick_index() == 0 {
            return Err(CloseError::FirstTrickNotCompleted);
        }
        if self.talon.len() < 4 {
            return Err(CloseError::NotEnoughTalonCards);
        }
        Ok(())
    }

    pub fn close_talon(&mut self, player: Player) -> Result<(), CloseError> {
        self.check_close_talon(player)?;
        self.phase = Phase::ClosedTalon(TalonClosure {
            closer: player,
            opponent_at_closing: self.score(player.other()),
        });
        Ok(())
    }

    pub fn talon_closure(&self) -> Option<TalonClosure> {
        match self.phase {
            Phase::ClosedTalon(closure) => Some(closure),
            Phase::Finished(outcome) => outcome.closure,
            _ => None,
        }
    }

    pub fn awaiting_final_declaration(&self) -> bool {
        matches!(self.phase, Phase::ClosedTalon(_))
            && self.played.is_empty()
            && self.hands.iter().all(Vec::is_empty)
    }

    pub fn decline_final_declaration(&mut self, player: Player) -> bool {
        if self.awaiting_final_declaration() && self.leader == player {
            let closure = self
                .talon_closure()
                .expect("final declaration requires a closure");
            self.finish_failed_closure(closure);
            true
        } else {
            false
        }
    }

    fn finish_failed_closure(&mut self, closure: TalonClosure) {
        self.phase = Phase::Finished(Outcome {
            winner: closure.closer.other(),
            reason: FinishReason::FailedClosure,
            game_points: failed_closure_points(closure),
            closure: Some(closure),
        });
    }

    pub fn eyes(&self, player: Player) -> u8 {
        let eyes = self.eye_breakdown(player);
        eyes.tricks + eyes.effective_announcements
    }

    pub fn eye_breakdown(&self, player: Player) -> EyeBreakdown {
        let tricks = &self.tricks[player as usize];
        let announced = self
            .announcements
            .iter()
            .filter(|event| event.player == player)
            .map(|event| event.marriage.eyes)
            .sum();
        let (effective_announcements, pending_announcements) = if tricks.is_empty() {
            (0, announced)
        } else {
            (announced, 0)
        };
        EyeBreakdown {
            tricks: tricks.iter().flatten().copied().map(Card::points).sum(),
            effective_announcements,
            pending_announcements,
        }
    }

    fn score(&self, player: Player) -> Score {
        Score {
            eyes: self.eyes(player),
            tricks: self.tricks[player as usize].len(),
        }
    }

    pub fn variant(&self) -> Variant {
        self.variant
    }

    pub fn announcements(&self) -> &[Announcement] {
        &self.announcements
    }

    pub fn marriage(&self, player: Player, suit: Suit) -> Result<Marriage, MarriageError> {
        if self.next_player() != Some(player) || !self.played.is_empty() {
            return Err(MarriageError::CannotLead);
        }
        let marriage = Marriage {
            suit,
            eyes: if suit == self.trump_suit { 40 } else { 20 },
            may_play_ober: self.variant == Variant::Soft,
        };
        if self
            .announcements
            .iter()
            .any(|event| event.marriage.suit == suit)
        {
            return Err(MarriageError::AlreadyAnnounced);
        }
        let hand = &self.hands[player as usize];
        if !hand.contains(&marriage.king()) || !hand.contains(&marriage.ober()) {
            return Err(MarriageError::PairMissing);
        }
        if self.variant == Variant::Sharp && self.tricks[player as usize].is_empty() {
            return Err(MarriageError::NoTrickWon);
        }
        Ok(marriage)
    }

    pub fn legal_marriages(&self, player: Player) -> Vec<Marriage> {
        [Suit::Hearts, Suit::Bells, Suit::Leaves, Suit::Acorns]
            .into_iter()
            .filter_map(|suit| self.marriage(player, suit).ok())
            .collect()
    }

    pub fn announce_and_play(
        &mut self,
        player: Player,
        suit: Suit,
        card: Card,
    ) -> Result<Announcement, MarriageError> {
        let marriage = self.marriage(player, suit)?;
        if card != marriage.king() && !(marriage.may_play_ober && card == marriage.ober()) {
            return Err(MarriageError::InvalidLeadCard);
        }
        // All conditions for a free lead have already been checked.
        self.play_card(player, card)
            .expect("a validated marriage is a legal lead");
        let event = Announcement {
            player,
            marriage,
            played: card,
            trick_index: self.trick_index(),
        };
        self.announcements.push(event);
        Ok(event)
    }

    fn trick_index(&self) -> usize {
        self.tricks.iter().map(Vec::len).sum()
    }

    pub fn can_declare_after_announcement(&self, player: Player) -> bool {
        self.announcement_lead(player) && self.can_declare(player)
    }

    pub fn announcement_lead(&self, player: Player) -> bool {
        matches!(self.played.as_slice(), [(leader, _)] if *leader == player)
            && self.announcements.last().is_some_and(|event| {
                event.player == player && event.trick_index == self.trick_index()
            })
    }

    pub fn result(&self) -> Option<HandResult> {
        let Phase::Finished(outcome) = self.phase else {
            return None;
        };
        Some(HandResult {
            outcome,
            loser: outcome.winner.other(),
            scores: [self.score(Player::Local), self.score(Player::Opponent)],
        })
    }

    /// Review from the local player's perspective, using the active rule profile.
    pub fn review_tricks(&self, owner: Player) -> Result<Vec<ReviewedTrick>, TrickReviewError> {
        if self.variant == Variant::Sharp {
            return Err(TrickReviewError::Forbidden);
        }
        let limit = if owner == Player::Local {
            usize::MAX
        } else {
            1
        };
        let tricks: Vec<_> = self
            .trick_winners
            .iter()
            .enumerate()
            .filter(|&(_, &winner)| winner == owner)
            .zip(self.tricks[owner as usize].iter().copied())
            .take(limit)
            .map(|((index, &winner), cards)| ReviewedTrick {
                number: index + 1,
                winner,
                cards,
            })
            .collect();
        if tricks.is_empty() {
            Err(TrickReviewError::Empty)
        } else {
            Ok(tricks)
        }
    }

    fn check_declaration(&self, player: Player) -> Result<(), DeclareError> {
        if matches!(self.phase, Phase::Finished(_)) {
            return Err(DeclareError::HandFinished);
        }
        let after_own_trick = self.played.is_empty() && !self.tricks[player as usize].is_empty();
        if self.leader != player || !(after_own_trick || self.announcement_lead(player)) {
            return Err(DeclareError::NotBetweenOwnTrickAndLead);
        }
        Ok(())
    }

    pub fn may_declare(&self, player: Player) -> bool {
        self.check_declaration(player).is_ok()
    }

    pub fn can_declare(&self, player: Player) -> bool {
        self.may_declare(player) && self.eyes(player) >= 66
    }

    pub fn declare(&mut self, player: Player) -> Result<(), DeclareError> {
        self.check_declaration(player)?;
        if self.eyes(player) < 66 {
            let winner = player.other();
            let closure = self.talon_closure();
            let game_points = match closure {
                Some(closure) if player == closure.closer => failed_closure_points(closure),
                Some(closure) => declaration_points(closure.opponent_at_closing),
                None => declaration_points(self.score(winner)),
            };
            self.phase = Phase::Finished(Outcome {
                winner,
                reason: FinishReason::FalseDeclaration,
                game_points,
                closure,
            });
            return Ok(());
        }
        match self.phase {
            Phase::ClosedTalon(closure) if player == closure.closer => {
                self.phase = Phase::Finished(Outcome {
                    winner: player,
                    reason: FinishReason::SuccessfulClosure,
                    game_points: declaration_points(closure.opponent_at_closing),
                    closure: Some(closure),
                });
            }
            Phase::ClosedTalon(closure) => self.finish_failed_closure(closure),
            _ => {
                self.phase = Phase::Finished(Outcome {
                    winner: player,
                    reason: FinishReason::Declared66,
                    game_points: declaration_points(self.score(player.other())),
                    closure: None,
                });
            }
        }
        Ok(())
    }

    pub fn next_player(&self) -> Option<Player> {
        if matches!(self.phase, Phase::Finished(_)) || self.awaiting_final_declaration() {
            return None;
        }
        match self.played.len() {
            0 => Some(self.leader),
            1 => Some(self.leader.other()),
            _ => None,
        }
    }

    pub fn legal_cards(&self, player: Player) -> Vec<Card> {
        self.legal_replies(player).0
    }

    fn legal_replies(&self, player: Player) -> (Vec<Card>, Option<PlayError>) {
        if self.next_player() != Some(player) {
            return (Vec::new(), None);
        }
        let hand = &self.hands[player as usize];
        let Some(&(_, lead)) = self
            .played
            .first()
            .filter(|_| matches!(self.phase, Phase::EmptyTalon | Phase::ClosedTalon(_)))
        else {
            return (hand.clone(), None);
        };
        let priority = |card: Card| {
            if card.suit == lead.suit {
                if beats(card, lead, self.trump_suit) {
                    Reply::Beat
                } else {
                    Reply::Follow
                }
            } else if card.suit == self.trump_suit {
                Reply::Trump
            } else {
                Reply::Discard
            }
        };
        let best = hand
            .iter()
            .copied()
            .map(priority)
            .min()
            .unwrap_or(Reply::Discard);
        let restriction = match best {
            Reply::Beat => Some(PlayError::MustBeat(lead.suit)),
            Reply::Follow => Some(PlayError::MustFollow(lead.suit)),
            Reply::Trump => Some(PlayError::MustTrump),
            Reply::Discard => None,
        };
        (
            hand.iter()
                .copied()
                .filter(|&card| priority(card) == best)
                .collect(),
            restriction,
        )
    }

    /// Records one play. A complete trick stays visible until it is resolved.
    pub fn play_card(&mut self, player: Player, card: Card) -> Result<(), PlayError> {
        if matches!(self.phase, Phase::Finished(_)) || self.awaiting_final_declaration() {
            return Err(PlayError::UnsupportedPhase(self.phase));
        }
        let expected = self.next_player().ok_or(PlayError::TrickPending)?;
        if expected != player {
            return Err(PlayError::OutOfTurn { expected });
        }
        let index = self.hands[player as usize]
            .iter()
            .position(|&held| held == card)
            .ok_or(PlayError::CardNotInHand)?;
        let (legal, restriction) = self.legal_replies(player);
        if let Some(error) = restriction.filter(|_| !legal.contains(&card)) {
            return Err(error);
        }
        let played = self.hands[player as usize].remove(index);
        self.played.push((player, played));
        Ok(())
    }

    pub fn resolve_trick(&mut self) -> Result<Player, ResolveError> {
        let &[(leader, first), (follower, second)] = self.played.as_slice() else {
            return Err(ResolveError::IncompleteTrick);
        };
        let winner = if beats(second, first, self.trump_suit) {
            follower
        } else {
            leader
        };
        let cards = [first, second];
        self.tricks[winner as usize].push(cards);
        self.trick_winners.push(winner);
        self.played.clear();
        self.leader = winner;

        if self.phase == Phase::OpenTalon {
            for (player, card) in [winner, winner.other()]
                .into_iter()
                .zip(self.talon.drain(..2))
            {
                self.hands[player as usize].push(card);
            }
            if self.talon.is_empty() {
                self.phase = Phase::EmptyTalon;
            }
        } else if self.hands.iter().all(Vec::is_empty) {
            if let Phase::ClosedTalon(closure) = self.phase {
                if winner != closure.closer || !self.can_declare(winner) {
                    self.finish_failed_closure(closure);
                }
            } else {
                self.phase = Phase::Finished(Outcome {
                    winner,
                    reason: FinishReason::LastTrick,
                    game_points: 1,
                    closure: None,
                });
            }
        }
        Ok(winner)
    }

    pub fn view(&self, player: Player) -> View<'_> {
        View {
            hand: &self.hands[player as usize],
            opponent_hand_count: self.hands[player.other() as usize].len(),
            hidden_talon_count: self.talon.len().saturating_sub(1),
            open_trump: self
                .talon
                .last()
                .copied()
                .filter(|_| self.talon_closure().is_none()),
            talon_closed: self.talon_closure().is_some(),
            played: &self.played,
            own_trick_card_count: 2 * self.tricks[player as usize].len(),
            opponent_trick_card_count: 2 * self.tricks[player.other() as usize].len(),
        }
    }
}

fn failed_closure_points(closure: TalonClosure) -> u8 {
    if closure.opponent_at_closing.tricks == 0 {
        3
    } else {
        2
    }
}

fn declaration_points(opponent: Score) -> u8 {
    match opponent {
        Score { eyes: 33.., .. } => 1,
        Score { tricks: 0, .. } => 3,
        _ => 2,
    }
}

fn beats(challenger: Card, lead: Card, trump: Suit) -> bool {
    if challenger.suit == lead.suit {
        // For the five Schnapsen ranks the eye values have the same ordering.
        challenger.points() > lead.points()
    } else {
        challenger.suit == trump
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn review_grants_only_the_profile_allowed_pairs_in_hand_order() {
        for variant in [Variant::Soft, Variant::Sharp] {
            let mut game = Game::with_variant(Player::Local, variant, |upper| upper - 1);
            let empty = match variant {
                Variant::Soft => TrickReviewError::Empty,
                Variant::Sharp => TrickReviewError::Forbidden,
            };
            for owner in [Player::Local, Player::Opponent] {
                assert_eq!(game.review_tricks(owner), Err(empty));
            }
            let mut played_tricks = Vec::new();
            for number in 1..=10 {
                let mut pair = [full_deck()[0]; 2];
                for card in &mut pair {
                    let player = game.next_player().unwrap();
                    *card = game.legal_cards(player)[0];
                    game.play_card(player, *card).unwrap();
                }
                let winner = game.resolve_trick().unwrap();
                played_tricks.push(ReviewedTrick {
                    number,
                    winner,
                    cards: pair,
                });
                let before = game.clone();
                for owner in [Player::Local, Player::Opponent] {
                    let limit = if owner == Player::Local { 10 } else { 1 };
                    let expected: Vec<_> = played_tricks
                        .iter()
                        .copied()
                        .filter(|trick| trick.winner == owner)
                        .take(limit)
                        .collect();
                    match variant {
                        Variant::Sharp => {
                            assert_eq!(game.review_tricks(owner), Err(TrickReviewError::Forbidden))
                        }
                        Variant::Soft if expected.is_empty() => {
                            assert_eq!(game.review_tricks(owner), Err(TrickReviewError::Empty))
                        }
                        Variant::Soft => {
                            let reviewed = game.review_tricks(owner).unwrap();
                            assert_eq!(reviewed, expected);
                            let view = game.view(Player::Local);
                            assert!(reviewed.iter().flat_map(|trick| trick.cards).all(
                                |card| !game.talon.contains(&card)
                                    && !view.hand.contains(&card)
                                    && !game.hands[1].contains(&card)
                            ));
                        }
                    }
                }
                assert_eq!(game, before);
            }
            assert!(
                played_tricks
                    .iter()
                    .filter(|trick| trick.winner == Player::Opponent)
                    .count()
                    > 1
            );
        }
    }

    fn complete_first_trick(game: &mut Game) {
        let deck = full_deck();
        game.play_card(Player::Local, deck[0]).unwrap();
        game.play_card(Player::Opponent, deck[3]).unwrap();
        assert_eq!(game.resolve_trick(), Ok(Player::Local));
    }

    #[test]
    fn talon_actions_require_a_resolved_trick_an_open_talon_and_the_lead() {
        for variant in [Variant::Soft, Variant::Sharp] {
            let mut game = exchange_deal(variant);
            let before = game.clone();
            assert_eq!(
                game.close_talon(Player::Local),
                Err(CloseError::FirstTrickNotCompleted)
            );
            assert_eq!(
                game.exchange_trump(Player::Local),
                Err(ExchangeError::FirstTrickNotCompleted)
            );
            assert_eq!(
                game.close_talon(Player::Opponent),
                Err(CloseError::CannotLead)
            );
            assert_eq!(
                game.exchange_trump(Player::Opponent),
                Err(ExchangeError::CannotLead)
            );
            assert_eq!(game, before);

            let deck = full_deck();
            game.play_card(Player::Local, deck[0]).unwrap();
            for complete in [false, true] {
                if complete {
                    game.play_card(Player::Opponent, deck[3]).unwrap();
                }
                let before = game.clone();
                assert_eq!(
                    game.close_talon(Player::Local),
                    Err(CloseError::TrickStarted)
                );
                assert_eq!(
                    game.exchange_trump(Player::Local),
                    Err(ExchangeError::TrickStarted)
                );
                assert_eq!(game, before);
            }
            game.resolve_trick().unwrap();
            assert!(game.trump_exchange(Player::Local).is_ok());
            assert!(game.check_close_talon(Player::Local).is_ok());

            for count in [3, 4] {
                let mut boundary = game.clone();
                boundary.talon.drain(..boundary.talon.len() - count);
                let before = boundary.clone();
                if count == 3 {
                    assert_eq!(
                        boundary.close_talon(Player::Local),
                        Err(CloseError::NotEnoughTalonCards)
                    );
                    assert_eq!(
                        boundary.exchange_trump(Player::Local),
                        Err(ExchangeError::NotEnoughTalonCards)
                    );
                    assert_eq!(boundary, before);
                } else {
                    boundary.exchange_trump(Player::Local).unwrap();
                    boundary.close_talon(Player::Local).unwrap();
                    let before = boundary.clone();
                    assert_eq!(
                        boundary.close_talon(Player::Local),
                        Err(CloseError::TalonNotOpen)
                    );
                    assert_eq!(
                        boundary.exchange_trump(Player::Local),
                        Err(ExchangeError::TalonNotOpen)
                    );
                    assert_eq!(boundary, before);
                }
            }
            game.phase = Phase::EmptyTalon;
            game.talon.clear();
            let before = game.clone();
            assert_eq!(
                game.close_talon(Player::Local),
                Err(CloseError::TalonNotOpen)
            );
            assert_eq!(
                game.exchange_trump(Player::Local),
                Err(ExchangeError::TalonNotOpen)
            );
            assert_eq!(game, before);
        }
    }

    #[test]
    fn talon_actions_do_not_require_a_personal_trick() {
        // A staged lead isolates this check. In normal play the winner of
        // the preceding trick automatically has the lead.
        for variant in [Variant::Soft, Variant::Sharp] {
            let mut game = exchange_deal(variant);
            complete_first_trick(&mut game);
            game.tricks.swap(0, 1);
            game.trick_winners[0] = Player::Opponent;
            assert!(game.tricks[0].is_empty());
            game.exchange_trump(Player::Local).unwrap();
            game.close_talon(Player::Local).unwrap();
            assert_deck_partition(&game);
        }
    }

    #[test]
    fn closed_talon_preserves_all_cards_enforces_following_and_allows_announcements() {
        let deck = full_deck();
        let mut game = Game::new(Player::Local, |upper| upper - 1);
        complete_first_trick(&mut game);
        let before = game.clone();
        game.close_talon(Player::Local).unwrap();
        assert_eq!(game.hands, before.hands);
        assert_eq!(game.trump_suit(), before.trump_suit());
        assert_eq!(game.next_player(), Some(Player::Local));
        assert!(game.view(Player::Local).talon_closed);
        assert!(game.view(Player::Local).open_trump.is_none());
        assert_eq!(
            game.exchange_trump(Player::Local),
            Err(ExchangeError::TalonNotOpen)
        );
        game.announce_and_play(Player::Local, Suit::Bells, deck[7])
            .unwrap();
        assert_eq!(game.legal_cards(Player::Opponent), [deck[5]]);
        game.play_card(Player::Opponent, deck[5]).unwrap();
        game.resolve_trick().unwrap();
        assert_eq!(game.hands.each_ref().map(Vec::len), [4, 4]);
        assert_eq!(game.talon, before.talon);
        assert_deck_partition(&game);
    }

    #[test]
    fn closure_uses_the_frozen_score_for_wins_and_opponent_declarations() {
        let deck = full_deck();
        let cases = [(0, 3, 3), (1, 2, 2), (2, 1, 2)];
        for (count, success_points, failure_points) in cases {
            let mut game = Game::new(Player::Local, |upper| upper - 1);
            complete_first_trick(&mut game);
            game.tricks[1] = [[deck[6], deck[11]], [deck[16], deck[17]]][..count].to_vec();
            game.close_talon(Player::Local).unwrap();
            let snapshot = game.talon_closure().unwrap();
            let before = game.clone();
            assert_eq!(
                game.declare(Player::Opponent),
                Err(DeclareError::NotBetweenOwnTrickAndLead)
            );
            assert_eq!(game, before);
            game.tricks[0] = [[0, 5], [10, 15], [1, 2], [7, 12]]
                .map(|[a, b]| [deck[a], deck[b]])
                .to_vec();
            game.tricks[1].push([deck[9], deck[14]]);
            let mut failed = game.clone();
            game.declare(Player::Local).unwrap();
            let outcome = game.result().unwrap().outcome;
            assert_eq!(
                (outcome.winner, outcome.reason, outcome.game_points),
                (
                    Player::Local,
                    FinishReason::SuccessfulClosure,
                    success_points
                )
            );
            assert_eq!(game.talon_closure(), Some(snapshot));
            assert_ne!(
                game.result().unwrap().score(Player::Opponent),
                snapshot.opponent_at_closing
            );
            failed.leader = Player::Opponent;
            failed.tricks[1] = std::mem::take(&mut failed.tricks[0]);
            failed.declare(Player::Opponent).unwrap();
            let outcome = failed.result().unwrap().outcome;
            assert_eq!(
                (outcome.winner, outcome.reason, outcome.game_points),
                (
                    Player::Opponent,
                    FinishReason::FailedClosure,
                    failure_points
                )
            );
            assert_eq!(failed.talon_closure(), Some(snapshot));
        }
    }

    #[test]
    fn closed_talon_final_trick_requires_a_declaration_and_supports_forfeiting() {
        let deck = full_deck();
        let mut game = Game::new(Player::Local, |upper| upper - 1);
        complete_first_trick(&mut game);
        game.close_talon(Player::Local).unwrap();
        game.hands = [vec![deck[0]], vec![deck[3]]];
        game.tricks[0].clear(); // Set both score fixtures independently of the opening trick.
        let mut low_score = game.clone();
        game.tricks[0] = [[5, 10], [15, 1], [6, 11]]
            .map(|[a, b]| [deck[a], deck[b]])
            .to_vec();
        for hand in [&mut game, &mut low_score] {
            hand.play_card(Player::Local, deck[0]).unwrap();
            hand.play_card(Player::Opponent, deck[3]).unwrap();
            let pending = hand.clone();
            assert!(!hand.awaiting_final_declaration());
            assert!(!hand.decline_final_declaration(Player::Local));
            assert_eq!(*hand, pending);
            assert_eq!(hand.resolve_trick(), Ok(Player::Local));
            assert!(hand.legal_cards(Player::Local).is_empty());
        }
        let low_outcome = low_score.result().unwrap().outcome;
        assert_eq!(
            (
                low_outcome.winner,
                low_outcome.reason,
                low_outcome.game_points
            ),
            (Player::Opponent, FinishReason::FailedClosure, 3)
        );
        let before = low_score.clone();
        assert_eq!(
            low_score.declare(Player::Local),
            Err(DeclareError::HandFinished)
        );
        assert_eq!(
            low_score.close_talon(Player::Local),
            Err(CloseError::TalonNotOpen)
        );
        assert_eq!(
            low_score.play_card(Player::Local, deck[0]),
            Err(PlayError::UnsupportedPhase(before.phase()))
        );
        assert_eq!(low_score, before);
        assert!(game.awaiting_final_declaration());
        assert!(game.result().is_none());
        let mut declined = game.clone();
        assert!(declined.decline_final_declaration(Player::Local));
        assert_eq!(
            declined.result().unwrap().outcome.reason,
            FinishReason::FailedClosure
        );
        game.declare(Player::Local).unwrap();
        assert_eq!(
            game.result().unwrap().outcome.reason,
            FinishReason::SuccessfulClosure
        );
    }

    fn exchange_deal(variant: Variant) -> Game {
        // Exchange deck positions 7 and 9 so forehand receives the trump Unter.
        Game::with_variant(Player::Local, variant, |upper| {
            if upper == 10 {
                7
            } else {
                upper - 1
            }
        })
    }

    #[test]
    fn exchange_after_first_trick_preserves_order_and_cards() {
        let mut game = exchange_deal(Variant::Soft);
        complete_first_trick(&mut game);
        let before = game.clone();
        let exchange = game.exchange_trump(Player::Local).unwrap();
        assert_eq!(exchange.given, game.trump_under());
        assert_eq!(
            Some(exchange.received),
            before.view(Player::Local).open_trump
        );
        assert_eq!(game.view(Player::Local).open_trump, Some(exchange.given));
        let mut expected = before.hands[0].clone();
        *expected
            .iter_mut()
            .find(|card| **card == exchange.given)
            .unwrap() = exchange.received;
        assert_eq!(game.hands[0], expected);
        assert_eq!(&game.talon[..7], &before.talon[..7]);
        assert_eq!(game.trump_suit(), before.trump_suit());
        assert_eq!(game.next_player(), before.next_player());
        assert_eq!(game.eyes(Player::Local), before.eyes(Player::Local));
        assert!(game.played.is_empty());
        assert_deck_partition(&game);
        let before = game.clone();
        assert_eq!(
            game.exchange_trump(Player::Local),
            Err(ExchangeError::MissingTrumpUnder)
        );
        assert_eq!(game, before);
    }

    #[test]
    fn neither_profile_can_swap_at_the_last_draw() {
        for variant in [Variant::Soft, Variant::Sharp] {
            let mut game = exchange_deal(variant);
            let used: Vec<_> = game.talon.drain(..8).collect();
            game.tricks[0].extend_from_slice(used.as_chunks::<2>().0);
            let top = game.talon[0];
            let trump = game.talon[1];
            let before = game.clone();
            assert_eq!(
                game.exchange_trump(Player::Local),
                Err(ExchangeError::NotEnoughTalonCards)
            );
            assert_eq!(game, before);
            let deck = full_deck();
            game.play_card(Player::Local, deck[0]).unwrap();
            game.play_card(Player::Opponent, deck[3]).unwrap();
            assert_eq!(game.resolve_trick(), Ok(Player::Local));
            assert_eq!(game.hands[0].last(), Some(&top));
            assert_eq!(game.hands[1].last(), Some(&trump));
            assert_eq!(game.phase(), Phase::EmptyTalon);
            assert_deck_partition(&game);
        }
    }

    #[test]
    fn additional_exchange_depends_only_on_current_card_ownership() {
        let mut game = exchange_deal(Variant::Soft);
        complete_first_trick(&mut game);
        let first = game.exchange_trump(Player::Local).unwrap();
        // Fixture for a second eligible position. Regular play cannot recover
        // the Unter from the bottom before the talon is exhausted.
        let opponent_index = game.hands[1]
            .iter()
            .position(|card| card.suit == first.given.suit)
            .unwrap();
        std::mem::swap(
            &mut game.hands[1][opponent_index],
            game.talon.last_mut().unwrap(),
        );
        game.leader = Player::Opponent;
        let second = game.exchange_trump(Player::Opponent).unwrap();
        assert_eq!(second.given, first.given);
        assert_eq!(game.view(Player::Local).open_trump, Some(first.given));
        assert_deck_partition(&game);
    }

    #[test]
    fn soft_trump_marriage_plays_either_card_and_activates_once_on_first_win() {
        let deck = full_deck();
        for card in [deck[7], deck[8]] {
            let mut game = Game::new(Player::Local, |upper| upper - 1);
            let before = game.clone();
            assert_eq!(
                game.announce_and_play(Player::Opponent, Suit::Bells, card),
                Err(MarriageError::CannotLead)
            );
            assert_eq!(
                game.announce_and_play(Player::Local, Suit::Hearts, deck[2]),
                Err(MarriageError::PairMissing)
            );
            assert_eq!(
                game.announce_and_play(Player::Local, Suit::Bells, deck[2]),
                Err(MarriageError::InvalidLeadCard)
            );
            assert_eq!(game, before);
            let event = game
                .announce_and_play(Player::Local, Suit::Bells, card)
                .unwrap();
            assert_eq!(event.marriage.eyes, 40);
            assert_eq!(game.announcements(), &[event]);
            assert_eq!(game.view(Player::Local).played, &[(Player::Local, card)]);
            assert!(game.view(Player::Local).hand.contains(&event.partner()));
            assert_eq!(
                game.eye_breakdown(Player::Local),
                EyeBreakdown {
                    tricks: 0,
                    effective_announcements: 0,
                    pending_announcements: 40,
                }
            );
            assert_deck_partition(&game);
            game.play_card(Player::Opponent, deck[3]).unwrap();
            assert_eq!(game.resolve_trick(), Ok(Player::Local));
            assert_eq!(
                game.eye_breakdown(Player::Local),
                EyeBreakdown {
                    tricks: card.points() + deck[3].points(),
                    effective_announcements: 40,
                    pending_announcements: 0,
                }
            );
            let before = game.clone();
            assert_eq!(
                game.announce_and_play(Player::Local, Suit::Bells, card),
                Err(MarriageError::AlreadyAnnounced)
            );
            assert_eq!(game, before);
            assert_deck_partition(&game);
        }
    }

    #[test]
    fn sharp_requires_an_earlier_trick_and_the_king() {
        let deck = full_deck();
        let mut game = Game::with_variant(Player::Local, Variant::Sharp, |upper| upper - 1);
        let before = game.clone();
        assert_eq!(
            game.announce_and_play(Player::Local, Suit::Bells, deck[7]),
            Err(MarriageError::NoTrickWon)
        );
        assert_eq!(game, before);
        game.play_card(Player::Local, deck[0]).unwrap();
        game.play_card(Player::Opponent, deck[3]).unwrap();
        assert_eq!(game.resolve_trick(), Ok(Player::Local));
        let before = game.clone();
        assert_eq!(
            game.announce_and_play(Player::Local, Suit::Bells, deck[8]),
            Err(MarriageError::InvalidLeadCard)
        );
        assert_eq!(game, before);
        let event = game
            .announce_and_play(Player::Local, Suit::Bells, deck[7])
            .unwrap();
        assert_eq!(event.marriage.eyes, 40);
        assert_eq!(
            game.eye_breakdown(Player::Local).effective_announcements,
            40
        );
        assert_eq!(game.variant(), Variant::Sharp);
        assert_deck_partition(&game);
    }

    #[test]
    fn nontrump_marriage_with_empty_talon_can_enable_immediate_declaration() {
        let deck = full_deck();
        let mut game = Game::new(Player::Local, |upper| upper - 1);
        game.phase = Phase::EmptyTalon;
        game.talon.clear();
        game.tricks[0] = [[0, 5], [10, 15], [1, 4]]
            .map(|[a, b]| [deck[a], deck[b]])
            .to_vec();
        game.hands[0] = vec![deck[2], deck[3]];
        assert_eq!(game.eyes(Player::Local), 56);
        assert!(!game.can_declare(Player::Local));
        let event = game
            .announce_and_play(Player::Local, Suit::Hearts, deck[3])
            .unwrap();
        assert_eq!(event.marriage.eyes, 20);
        assert_eq!(game.eyes(Player::Local), 76);
        assert!(game.can_declare_after_announcement(Player::Local));
        let mut continuing = game.clone();
        let reply = continuing.legal_cards(Player::Opponent)[0];
        continuing.play_card(Player::Opponent, reply).unwrap();
        assert_eq!(
            continuing.declare(Player::Local),
            Err(DeclareError::NotBetweenOwnTrickAndLead)
        );
        game.declare(Player::Local).unwrap();
        assert_eq!(
            game.result().unwrap().outcome.reason,
            FinishReason::Declared66
        );
        assert_eq!(
            game.announce_and_play(Player::Local, Suit::Hearts, deck[2]),
            Err(MarriageError::CannotLead)
        );
    }

    #[test]
    fn deal_gives_three_then_two_and_places_open_trump_last_in_draw_order() {
        let deck = full_deck();
        let game = Game::new(Player::Opponent, |upper| upper - 1);
        assert_eq!(game.hands[1], [&deck[..3], &deck[7..9]].concat());
        assert_eq!(game.hands[0], [&deck[3..6], &deck[9..11]].concat());
        assert_eq!(game.talon, [&deck[11..], &deck[6..7]].concat());
        assert_eq!(game.view(Player::Local).open_trump, Some(deck[6]));
        assert_eq!(game.trump_suit(), deck[6].suit);
        assert_eq!(game.leader(), Player::Opponent);
        assert_eq!(game.phase(), Phase::OpenTalon);
    }

    #[test]
    fn controlled_shuffle_is_reproducible_and_all_areas_partition_the_deck() {
        let deal = || Game::new(Player::Local, |upper| upper / 2);
        let game = deal();
        assert_eq!(game, deal());
        assert_ne!(game, Game::new(Player::Local, |upper| upper - 1));
        assert_eq!(game.hands.each_ref().map(Vec::len), [5, 5]);
        assert_eq!(game.talon.len(), 10);
        assert_deck_partition(&game);
        let view = game.view(Player::Local);
        assert_eq!(view.hand.len(), 5);
        assert_eq!(view.opponent_hand_count, 5);
        assert_eq!(view.hidden_talon_count, 9);
        assert!(view.played.is_empty());
        assert_eq!(
            view.own_trick_card_count + view.opponent_trick_card_count,
            0
        );
    }

    fn assert_deck_partition(game: &Game) {
        let cards: Vec<_> = game
            .hands
            .iter()
            .flatten()
            .chain(&game.talon)
            .chain(game.played.iter().map(|(_, card)| card))
            .chain(game.tricks.iter().flatten().flatten())
            .copied()
            .collect();
        assert_eq!(cards.len(), 20);
        assert_eq!(
            cards.into_iter().collect::<HashSet<_>>(),
            HashSet::from(full_deck())
        );
    }

    #[test]
    fn deterministic_hands_preserve_all_cards_across_every_finish_reason() {
        let mut finishes = [false; 4];
        let mut rejected_exchange_before_last_draw = false;
        let mut declared_after_marriage = false;
        let mut announced_after_closing = false;
        let mut opponent_declared_after_closing = false;

        for variant in [Variant::Soft, Variant::Sharp] {
            for seed in 0_u64..384 {
                let mut random = seed;
                let forehand = if seed % 2 == 0 {
                    Player::Local
                } else {
                    Player::Opponent
                };
                let mut game = Game::with_variant(forehand, variant, |upper| {
                    random = random.wrapping_mul(6364136223846793005).wrapping_add(1);
                    (random >> 32) as usize % upper
                });
                for _ in 0..=20 {
                    assert_deck_partition(&game);
                    if game.result().is_some() {
                        break;
                    }
                    let leader = game.leader();
                    let before = game.clone();
                    let rejected = game.play_card(leader.other(), full_deck()[0]);
                    assert!(rejected.is_err());
                    assert_eq!(game, before);

                    if seed % 3 == 0
                        && game.trick_index() == (seed / 3 % 5) as usize
                        && game.check_close_talon(leader).is_ok()
                    {
                        game.close_talon(leader).unwrap();
                    }
                    if seed % 4 != 0 && game.can_declare(leader) {
                        opponent_declared_after_closing |= game
                            .talon_closure()
                            .is_some_and(|closure| closure.closer != leader);
                        game.declare(leader).unwrap();
                        break;
                    }
                    if game.awaiting_final_declaration() {
                        assert!(game.decline_final_declaration(leader));
                        break;
                    }
                    if game.talon.len() == 2
                        && game.hands[leader as usize].contains(&game.trump_under())
                    {
                        let before = game.clone();
                        assert_eq!(
                            game.exchange_trump(leader),
                            Err(ExchangeError::NotEnoughTalonCards)
                        );
                        assert_eq!(game, before);
                        rejected_exchange_before_last_draw = true;
                    }
                    if let Some(marriage) =
                        crate::ai::choose_marriage(&game.legal_marriages(leader))
                    {
                        game.announce_and_play(leader, marriage.suit, marriage.king())
                            .unwrap();
                        announced_after_closing |= game.talon_closure().is_some();
                        if seed % 4 != 0 && game.can_declare_after_announcement(leader) {
                            opponent_declared_after_closing |= game
                                .talon_closure()
                                .is_some_and(|closure| closure.closer != leader);
                            game.declare(leader).unwrap();
                            declared_after_marriage = true;
                            break;
                        }
                    } else {
                        let card = crate::ai::choose_card(&game.legal_cards(leader)).unwrap();
                        game.play_card(leader, card).unwrap();
                    }
                    assert_deck_partition(&game);
                    let reply = leader.other();
                    assert_eq!(game.next_player(), Some(reply));
                    let card = crate::ai::choose_card(&game.legal_cards(reply)).unwrap();
                    game.play_card(reply, card).unwrap();
                    assert_deck_partition(&game);
                    let pending = game.clone();
                    assert!(game.declare(leader).is_err());
                    assert!(game.declare(reply).is_err());
                    assert!(game.play_card(reply, card).is_err());
                    assert_eq!(game, pending);
                    let winner = game.resolve_trick().unwrap();
                    assert_eq!(game.leader(), winner);
                }

                assert_deck_partition(&game);
                let result = game
                    .result()
                    .expect("the hand must finish within ten tricks");
                finishes[match result.outcome.reason {
                    FinishReason::Declared66 => 0,
                    FinishReason::LastTrick => 1,
                    FinishReason::SuccessfulClosure => 2,
                    FinishReason::FailedClosure => 3,
                    FinishReason::FalseDeclaration => {
                        panic!("the simulation never declares below 66")
                    }
                }] = true;
                assert!((1..=3).contains(&result.outcome.game_points));
                let finished = game.clone();
                assert!(game.declare(forehand).is_err());
                assert!(game.resolve_trick().is_err());
                assert!(game.close_talon(forehand).is_err());
                assert!(game.exchange_trump(forehand).is_err());
                assert!(game.play_card(forehand, full_deck()[0]).is_err());
                assert_eq!(game, finished);
            }
        }
        assert_eq!(finishes, [true; 4]);
        assert!(rejected_exchange_before_last_draw);
        assert!(declared_after_marriage);
        assert!(announced_after_closing);
        assert!(opponent_declared_after_closing);
    }

    #[test]
    fn a_fresh_game_replaces_previous_cards_and_resets_table_and_tricks() {
        let mut game = Game::new(Player::Local, |upper| upper - 1);
        for _ in 0..2 {
            let player = game.next_player().unwrap();
            game.play_card(player, game.legal_cards(player)[0]).unwrap();
        }
        game.resolve_trick().unwrap();
        let player = game.next_player().unwrap();
        game.play_card(player, game.legal_cards(player)[0]).unwrap();
        let old_trump = game.view(Player::Local).open_trump;

        game = Game::new(Player::Local, |_| 0);
        let view = game.view(Player::Local);
        assert_ne!(view.open_trump, old_trump);
        assert_eq!(view.hand.len(), 5);
        assert_eq!(view.opponent_hand_count, 5);
        assert_eq!(view.hidden_talon_count, 9);
        assert!(view.played.is_empty());
        assert_eq!(
            (view.own_trick_card_count, view.opponent_trick_card_count),
            (0, 0)
        );
        assert_eq!(
            (game.eyes(Player::Local), game.eyes(Player::Opponent)),
            (0, 0)
        );
    }

    #[test]
    fn ranks_and_suits_decide_the_open_talon_trick() {
        let deck = full_deck();
        for (index, &lead) in deck[..5].iter().enumerate() {
            for (other, &challenger) in deck[..5].iter().enumerate() {
                assert_eq!(beats(challenger, lead, Suit::Bells), other < index);
            }
        }
        assert!(beats(deck[9], deck[0], Suit::Bells));
        assert!(!beats(deck[0], deck[9], Suit::Bells));
        assert!(!beats(deck[10], deck[4], Suit::Bells));
    }

    #[test]
    fn validates_turn_membership_and_pending_trick_without_mutating_on_error() {
        let mut game = Game::new(Player::Opponent, |upper| upper - 1);
        let local = game.hands[0][3];
        let opponent = game.hands[1][0];
        let before = game.clone();
        assert_eq!(game.resolve_trick(), Err(ResolveError::IncompleteTrick));
        assert_eq!(
            game.play_card(Player::Local, local),
            Err(PlayError::OutOfTurn {
                expected: Player::Opponent
            })
        );
        assert_eq!(
            game.play_card(Player::Opponent, local),
            Err(PlayError::CardNotInHand)
        );
        assert_eq!(game, before);
        game.play_card(Player::Opponent, opponent).unwrap();
        assert_eq!(game.next_player(), Some(Player::Local));
        let before = game.clone();
        assert_eq!(game.resolve_trick(), Err(ResolveError::IncompleteTrick));
        assert_eq!(game, before);
        game.play_card(Player::Local, local).unwrap();
        assert_eq!(game.next_player(), None);
        let before = game.clone();
        assert_eq!(
            game.play_card(Player::Local, game.hands[0][0]),
            Err(PlayError::TrickPending)
        );
        assert_eq!(game, before);
        assert_eq!(game.resolve_trick(), Ok(Player::Local));
        assert_eq!(game.tricks[0], [[opponent, local]]);
        assert_eq!(game.eyes(Player::Local), 13);
        assert_eq!(game.next_player(), Some(Player::Local));
    }

    #[test]
    fn follows_any_suit_and_draws_winner_first_through_the_open_trump() {
        let mut game = Game::new(Player::Local, |upper| upper - 1);
        let trump = game.view(Player::Local).open_trump.unwrap();
        for trick in 0..5 {
            let [top, next] = [game.talon[0], game.talon[1]];
            let first_player = game.next_player().unwrap();
            let second_player = first_player.other();
            let first = game.legal_cards(first_player)[0];
            game.play_card(first_player, first).unwrap();
            let legal = game.legal_cards(second_player);
            assert_eq!(legal.len(), 5);
            let second = legal[0];
            game.play_card(second_player, second).unwrap();
            assert_deck_partition(&game);
            let winner = game.resolve_trick().unwrap();
            assert_eq!(game.tricks[winner as usize].last(), Some(&[first, second]));
            assert_eq!(game.hands[winner as usize].last(), Some(&top));
            assert_eq!(game.hands[winner.other() as usize].last(), Some(&next));
            assert_eq!(game.hands.each_ref().map(Vec::len), [5, 5]);
            assert_eq!(game.talon.len(), 8 - trick * 2);
            assert!(game.played.is_empty());
            assert_deck_partition(&game);
            assert_eq!(
                game.eyes(winner),
                game.tricks[winner as usize]
                    .iter()
                    .flatten()
                    .map(|card| card.points())
                    .sum::<u8>()
            );
            if trick < 4 {
                assert_eq!(game.next_player(), Some(winner));
                assert_eq!(game.view(Player::Local).open_trump, Some(trump));
                assert_eq!(game.phase(), Phase::OpenTalon);
            } else {
                assert_eq!(next, trump);
                assert_eq!(game.view(Player::Local).open_trump, None);
                assert_eq!(game.trump_suit(), trump.suit);
                assert_eq!(game.phase(), Phase::EmptyTalon);
                assert_eq!(game.next_player(), Some(winner));
            }
        }
    }

    #[test]
    fn reply_priorities_and_rejections_are_shared_by_queries_and_actions() {
        let deck = full_deck();
        type ReplyCase = (
            &'static [usize],
            &'static [usize],
            Option<(usize, PlayError)>,
        );
        let cases: &[ReplyCase] = &[
            (
                &[3, 5, 0, 1],
                &[0, 1],
                Some((3, PlayError::MustBeat(Suit::Hearts))),
            ),
            (
                &[5, 3, 4, 10],
                &[3, 4],
                Some((5, PlayError::MustFollow(Suit::Hearts))),
            ),
            (&[10, 5, 6], &[5, 6], Some((10, PlayError::MustTrump))),
            (&[10, 15], &[10, 15], None),
        ];
        for &(hand, expected, rejected) in cases {
            let mut game = Game::new(Player::Local, |upper| upper - 1);
            game.phase = Phase::EmptyTalon;
            game.talon.clear();
            game.hands = [vec![deck[2]], hand.iter().map(|&i| deck[i]).collect()];
            game.play_card(Player::Local, deck[2]).unwrap();
            let legal = game.legal_cards(Player::Opponent);
            assert_eq!(legal, expected.iter().map(|&i| deck[i]).collect::<Vec<_>>());
            if let Some((index, error)) = rejected {
                let before = game.clone();
                assert_eq!(game.play_card(Player::Opponent, deck[index]), Err(error));
                assert_eq!(game, before);
            }
            for card in legal {
                assert_eq!(game.clone().play_card(Player::Opponent, card), Ok(()));
            }
            game.phase = Phase::OpenTalon;
            assert_eq!(game.legal_cards(Player::Opponent), game.hands[1]);
        }
    }

    #[test]
    fn ten_tricks_exhaust_all_cards_and_the_computer_obeys_forced_replies() {
        use crate::ai::choose_card;
        let mut forced_reply_seen = false;
        for seed in 0..16 {
            let mut game = Game::new(Player::Local, |upper| (seed + upper / 2) % upper);
            for trick in 0..10 {
                for _ in 0..2 {
                    let player = game.next_player().unwrap();
                    let legal = game.legal_cards(player);
                    let card = if player == Player::Opponent {
                        if game.phase() == Phase::EmptyTalon
                            && !game.played.is_empty()
                            && game.hands[1].len() > 1
                            && legal.len() == 1
                        {
                            forced_reply_seen = true;
                        }
                        choose_card(&legal).unwrap()
                    } else {
                        legal[0]
                    };
                    game.play_card(player, card).unwrap();
                }
                let winner = game.resolve_trick().unwrap();
                assert_eq!(game.next_player(), (trick < 9).then_some(winner));
                assert_eq!(game.talon.len(), 10_usize.saturating_sub(2 * (trick + 1)));
                assert_eq!(game.hands.each_ref().map(Vec::len), [5.min(9 - trick); 2]);
                assert_deck_partition(&game);
            }
            let result = game.result().unwrap();
            assert_eq!(result.outcome.reason, FinishReason::LastTrick);
            assert_eq!(result.outcome.winner, game.leader());
            assert_eq!(result.outcome.game_points, 1);
            assert_eq!(game.eyes(Player::Local) + game.eyes(Player::Opponent), 120);
            let before = game.clone();
            for player in [Player::Local, Player::Opponent] {
                assert!(game.legal_cards(player).is_empty());
                assert_eq!(
                    game.play_card(player, full_deck()[0]),
                    Err(PlayError::UnsupportedPhase(game.phase()))
                );
                assert_eq!(game.declare(player), Err(DeclareError::HandFinished));
            }
            assert_eq!(game, before);
        }
        assert!(forced_reply_seen);
    }

    #[test]
    fn declaration_awards_points_from_both_scores_and_can_only_happen_once() {
        let deck = full_deck();
        for (opponent_pairs, points) in [
            (vec![], 3),
            (vec![[6, 11]], 2),
            (vec![[6, 11], [16, 17]], 1),
        ] {
            let mut game = Game::new(Player::Local, |upper| upper - 1);
            game.tricks[0] = [[0, 5], [10, 15], [1, 2], [7, 12]]
                .map(|[a, b]| [deck[a], deck[b]])
                .to_vec();
            game.tricks[1] = opponent_pairs
                .iter()
                .map(|&[a, b]| [deck[a], deck[b]])
                .collect();
            assert_eq!(game.eyes(Player::Local), 66);
            assert!(game.can_declare(Player::Local));
            assert_eq!(game.declare(Player::Local), Ok(()));
            let result = game.result().unwrap();
            assert_eq!(
                result.outcome,
                Outcome {
                    winner: Player::Local,
                    reason: FinishReason::Declared66,
                    game_points: points,
                    closure: None,
                }
            );
            assert_eq!(result.loser, Player::Opponent);
            assert_eq!(
                result.score(Player::Local),
                Score {
                    eyes: 66,
                    tricks: 4
                }
            );
            assert_eq!(result.score(Player::Opponent).tricks, opponent_pairs.len());
            let before = game.clone();
            assert_eq!(game.declare(Player::Local), Err(DeclareError::HandFinished));
            assert_eq!(
                game.play_card(Player::Local, deck[4]),
                Err(PlayError::UnsupportedPhase(game.phase()))
            );
            assert_eq!(game, before);
        }
        assert_eq!(declaration_points(Score { eyes: 0, tricks: 1 }), 2);
        assert_eq!(declaration_points(Score { eyes: 0, tricks: 0 }), 3);
        assert_eq!(
            declaration_points(Score {
                eyes: 32,
                tricks: 1
            }),
            2
        );
        assert_eq!(
            declaration_points(Score {
                eyes: 33,
                tricks: 1
            }),
            1
        );
    }

    #[test]
    fn false_declaration_after_marriage_scores_the_opponents_current_standing() {
        let deck = full_deck();
        for (opponent_tricks, points) in [
            (vec![], 3),
            (vec![[deck[5], deck[10]]], 2),
            (vec![[deck[5], deck[10]], [deck[15], deck[6]]], 1),
        ] {
            let mut game = Game::new(Player::Local, |upper| upper - 1);
            game.tricks[1] = opponent_tricks;
            let pair = game.legal_marriages(Player::Local)[0];
            game.announce_and_play(Player::Local, pair.suit, pair.king())
                .unwrap();
            assert!(game.may_declare(Player::Local));
            assert!(!game.can_declare(Player::Local));
            assert_eq!(game.eyes(Player::Local), 0);
            game.declare(Player::Local).unwrap();
            let outcome = game.result().unwrap().outcome;
            assert_eq!(outcome.winner, Player::Opponent);
            assert_eq!(outcome.reason, FinishReason::FalseDeclaration);
            assert_eq!(outcome.game_points, points);
            let before = game.clone();
            assert_eq!(game.declare(Player::Local), Err(DeclareError::HandFinished));
            assert_eq!(game, before);
        }
    }

    #[test]
    fn false_declaration_with_closed_talon_uses_the_frozen_standing() {
        let deck = full_deck();
        for (tricks_at_closing, closer_loss, closer_win) in [
            (vec![], 3, 3),
            (vec![[deck[5], deck[10]], [deck[15], deck[6]]], 2, 1),
        ] {
            let mut game = Game::new(Player::Local, |upper| upper - 1);
            complete_first_trick(&mut game);
            game.tricks[1] = tricks_at_closing;
            game.close_talon(Player::Local).unwrap();
            let snapshot = game.talon_closure();
            game.tricks[0].push([deck[0], deck[3]]);
            game.tricks[1].push([deck[9], deck[14]]);
            for (declarer, points) in [(Player::Local, closer_loss), (Player::Opponent, closer_win)]
            {
                let mut game = game.clone();
                game.leader = declarer;
                assert!(!game.can_declare(declarer));
                game.declare(declarer).unwrap();
                let outcome = game.result().unwrap().outcome;
                assert_eq!(
                    outcome,
                    Outcome {
                        winner: declarer.other(),
                        reason: FinishReason::FalseDeclaration,
                        game_points: points,
                        closure: snapshot,
                    }
                );
            }
        }
    }

    #[test]
    fn declaration_requires_enough_eyes_and_must_precede_the_next_lead() {
        let mut game = Game::new(Player::Local, |upper| upper - 1);
        let before = game.clone();
        assert_eq!(
            game.declare(Player::Local),
            Err(DeclareError::NotBetweenOwnTrickAndLead)
        );
        assert_eq!(game, before);
        let deck = full_deck();
        game.tricks[0] = [[0, 5], [10, 1], [6, 2], [7, 12]]
            .map(|[a, b]| [deck[a], deck[b]])
            .to_vec();
        assert_eq!(game.eyes(Player::Local), 65);
        assert!(!game.can_declare(Player::Local));
        let mut falsely_declared = game.clone();
        falsely_declared.declare(Player::Local).unwrap();
        assert_eq!(
            falsely_declared.result().unwrap().outcome.reason,
            FinishReason::FalseDeclaration
        );
        game.tricks[0] = [[0, 5], [10, 15], [1, 6], [11, 16]]
            .map(|[a, b]| [deck[a], deck[b]])
            .to_vec();
        game.tricks[1] = [[2, 7], [3, 8]].map(|[a, b]| [deck[a], deck[b]]).to_vec();
        game.phase = Phase::EmptyTalon;
        game.talon.clear();
        game.hands = [vec![deck[4], deck[9]], vec![deck[13], deck[14]]];
        assert_eq!(game.eyes(Player::Local), 84);
        assert!(game.can_declare(Player::Local));
        assert_eq!(game.clone().declare(Player::Local), Ok(()));
        let before = game.clone();
        assert_eq!(
            game.declare(Player::Opponent),
            Err(DeclareError::NotBetweenOwnTrickAndLead)
        );
        assert_eq!(game, before);
        game.play_card(Player::Local, deck[4]).unwrap();
        let before = game.clone();
        assert_eq!(
            game.declare(Player::Local),
            Err(DeclareError::NotBetweenOwnTrickAndLead)
        );
        assert_eq!(game, before);
        assert!(game.result().is_none());
    }
}
