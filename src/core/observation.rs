//! Owned public information available to a strategy for one player.

use serde::{Deserialize, Serialize};

use super::{
    cards::{Card, Suit},
    game::{Announcement, Exchange, Marriage, Phase, Player, Score, TalonClosure},
    rules::{GameVariant, RuleProfile, SchneiderRule},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObservedTrick {
    pub lead_player: Player,
    pub winner: Player,
    pub cards: [Card; 2],
    pub forced: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservedExchange {
    pub completed_tricks: usize,
    pub exchange: Exchange,
}

/// Public facts that are not recoverable from the currently visible position.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PublicMemory {
    pub exchanges: Vec<ObservedExchange>,
    pub last_trump: Option<Card>,
}

impl PublicMemory {
    pub fn record_exchange(&mut self, completed_tricks: usize, exchange: Exchange) {
        self.exchanges.push(ObservedExchange {
            completed_tricks,
            exchange,
        });
    }
}

/// Contains the same categories of information at every difficulty. The
/// opponent hand contributes its size, and the stock its size and public trump.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Observation {
    pub player: Player,
    pub variant: GameVariant,
    pub profile: RuleProfile,
    pub phase: Phase,
    pub hand: Vec<Card>,
    pub trump: Suit,
    pub open_trump: Option<Card>,
    pub known_talon_trump: Option<Card>,
    pub played: Vec<(Player, Card)>,
    pub tricks: Vec<ObservedTrick>,
    pub announcements: Vec<Announcement>,
    pub exchanges: Vec<ObservedExchange>,
    pub known_opponent: Vec<Card>,
    pub opponent_hand_count: usize,
    pub talon_count: usize,
    pub own_eyes: u8,
    pub opponent_eyes: u8,
    pub own_pending_eyes: u8,
    pub opponent_pending_eyes: u8,
    pub own_tricks: usize,
    pub opponent_tricks: usize,
    pub remaining: [u8; 2],
    pub schneider: SchneiderRule,
    pub legal_cards: Vec<Card>,
    pub legal_marriages: Vec<Marriage>,
    pub exchange: Option<Exchange>,
    pub can_close: bool,
}

impl Observation {
    pub fn beats(&self, challenger: Card, lead: Card) -> bool {
        self.variant.beats(challenger, lead, self.trump)
    }

    /// Apply the same reply priority as the engine to a hypothetical hand.
    pub fn hypothetical_replies(&self, hand: &[Card], lead: Card, forced: bool) -> Vec<Card> {
        if forced {
            self.variant.forced_replies(hand, lead, self.trump).0
        } else {
            hand.to_vec()
        }
    }

    pub fn could_hold(&self, player: Player, card: Card) -> bool {
        if player == self.player {
            return self.hand.contains(&card);
        }
        if self.hand.contains(&card)
            || self.known_talon_trump == Some(card)
            || self.played.iter().any(|&(_, played)| played == card)
            || self.tricks.iter().any(|trick| trick.cards.contains(&card))
        {
            return false;
        }

        // No cards are drawn after reply obligations begin. Any still-unseen
        // card in the opponent's hand must have been available at each reply.
        self.tricks
            .iter()
            .filter(|trick| trick.forced && trick.lead_player.other() == player)
            .all(|trick| {
                let [lead, reply] = trick.cards;
                self.variant.reply_priority(card, lead, self.trump)
                    >= self.variant.reply_priority(reply, lead, self.trump)
            })
    }

    pub fn target_eyes(&self) -> u8 {
        self.profile.rules().automatic_finish_eyes
    }

    pub fn last_trick_points(&self) -> u8 {
        self.variant.last_trick_points()
    }

    pub fn match_award(&self, winner: Player, points: u8) -> u8 {
        if points < self.remaining[winner as usize] {
            return 0;
        }
        self.schneider.bummerls(
            self.remaining[winner.other() as usize],
            self.variant.start_points(),
        )
    }

    pub fn winning_points(&self, opponent: Score) -> u8 {
        self.profile.rules().winning_points(opponent)
    }

    pub fn closure_points(&self, opponent: Score) -> (u8, u8) {
        let closure = TalonClosure {
            closer: self.player,
            opponent_at_closing: opponent,
        };
        let rules = self.profile.rules();
        (
            rules.successful_closure_points(closure),
            rules.failed_closure_points(closure),
        )
    }
}
