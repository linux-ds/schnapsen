use crate::core::{
    cards::{Card, Rank},
    game::Phase,
};

use super::{
    action_card, card_actions, marriage_eyes, unplayed_opponent_candidates, Action, Observation,
    Strategy,
};

pub(super) struct Medium;

impl Strategy for Medium {
    fn decide(&self, view: &Observation) -> Option<Action> {
        if useful_exchange(view) {
            return Some(Action::Exchange);
        }
        let candidates = unplayed_opponent_candidates(view);
        card_actions(view).max_by_key(|&action| tactic(view, &candidates, action))
    }
}

pub(super) fn useful_exchange(view: &Observation) -> bool {
    let can_finish_with_marriage = view.own_tricks > 0
        && view
            .legal_marriages
            .iter()
            .any(|entry| view.own_eyes + entry.eyes >= view.target_eyes());
    !can_finish_with_marriage
        && view
            .exchange
            .is_some_and(|exchange| exchange.received.points() > exchange.given.points())
}

pub(super) fn reserve_cost(view: &Observation, card: Card) -> i32 {
    let points = i32::from(card.points());
    points
        + if card.suit == view.trump {
            9 + points / 2
        } else {
            0
        }
}

fn pair_potential(view: &Observation, candidates: &[Card], card: Card) -> i32 {
    if !matches!(card.rank, Rank::King | Rank::Ober) {
        return 0;
    }
    let partner = Card {
        suit: card.suit,
        rank: if card.rank == Rank::King {
            Rank::Ober
        } else {
            Rank::King
        },
    };
    let value = if card.suit == view.trump { 14 } else { 8 };
    if view.hand.contains(&partner) {
        value
    } else if view.phase == Phase::OpenTalon && candidates.contains(&partner) {
        // It may turn up from the talon; keeping either half has modest value.
        value / 4
    } else {
        0
    }
}

pub(super) fn tactic(view: &Observation, candidates: &[Card], action: Action) -> i32 {
    let card = action_card(action);
    let announced = marriage_eyes(view, action);
    let effective = if view.own_tricks > 0 { announced } else { 0 };
    let own = u16::from(view.own_eyes) + u16::from(effective);
    let target = u16::from(view.target_eyes());
    if own >= target {
        return 1000;
    }

    let reserve = reserve_cost(view, card);
    let preserve = if announced == 0 {
        pair_potential(view, candidates, card)
    } else {
        0
    };
    let announcement_value = i32::from(announced) * if view.own_tricks > 0 { 3 } else { 2 };

    if let Some(&(_, lead)) = view.played.first() {
        let trick = u16::from(lead.points() + card.points());
        if view.beats(card, lead) {
            if own + trick + u16::from(view.own_pending_eyes) >= target {
                return 1000 - reserve;
            }
            let urgency = if u16::from(view.opponent_eyes + view.opponent_pending_eyes) + trick + 5
                >= target
            {
                24
            } else {
                0
            };
            let lead_value = if view.phase == Phase::OpenTalon {
                23
            } else {
                16
            };
            return (i32::from(lead.points()) * 3)
                + i32::from(card.points())
                + lead_value
                + urgency
                - reserve
                - preserve;
        }
        let loss = if u16::from(view.opponent_eyes + view.opponent_pending_eyes) + trick >= target {
            100
        } else {
            0
        };
        return -(i32::from(card.points()) * 3) - reserve - preserve - loss;
    }

    let forced = view.phase != Phase::OpenTalon;
    let same_suit = candidates
        .iter()
        .filter(|other| other.suit == card.suit)
        .count();
    let higher = candidates
        .iter()
        .filter(|&&other| view.beats(other, card))
        .count();
    let risk = if forced && same_suit >= view.opponent_hand_count {
        // Obligations favor higher cards of the led suit before trump is considered.
        let higher_same = candidates
            .iter()
            .filter(|&&other| other.suit == card.suit && view.beats(other, card))
            .count();
        higher_same * 8 + higher.saturating_sub(higher_same) * 2
    } else {
        higher * 7
    };
    let secure = if higher == 0 {
        28 + i32::from(card.points()) * 2
    } else {
        0
    };
    let draw_trump = if forced && card.suit == view.trump && higher == 0 {
        14
    } else {
        0
    };
    let open_lead = if forced { 0 } else { i32::from(card.points()) };
    announcement_value + secure + draw_trump - risk as i32 - reserve - preserve - open_lead
}
