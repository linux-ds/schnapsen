//! Extra checks for replay and simulations. The application keeps its existing
//! snapshot checks on load and does not run these checks after each action.

use std::fmt;

use crate::core::{
    game::{FinishReason, Phase, Player, Score},
    history::{PrivateEvent, PublicEvent},
    match_play::Match,
    rules::{GameVariant, SchneiderRule},
};

use super::case::RecordedAction;

const PLAYERS: [Player; 2] = [Player::Local, Player::Opponent];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ValidationError(pub &'static str);

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for ValidationError {}

/// Check the engine invariants and independently recount eyes from the public
/// cards. Each recorded trick contributes exactly once to its winner.
pub fn validate(state: &Match) -> Result<(), ValidationError> {
    if !state.validate_snapshot() {
        return Err(ValidationError("snapshot violates engine invariants"));
    }

    for player in PLAYERS {
        let observed = state.observation(player);
        let from_tricks: u16 = observed
            .tricks
            .iter()
            .filter(|trick| trick.winner == player)
            .flat_map(|trick| trick.cards)
            .map(|card| u16::from(card.points()))
            .sum();
        let announced: u16 = observed
            .announcements
            .iter()
            .filter(|announcement| announcement.player == player)
            .map(|announcement| {
                if announcement.marriage.suit == observed.trump {
                    40
                } else {
                    20
                }
            })
            .sum();
        let has_trick = observed.tricks.iter().any(|trick| trick.winner == player);
        let (effective, pending) = if has_trick {
            (announced, 0)
        } else {
            (0, announced)
        };
        let actual = state.hand().eye_breakdown(player);
        if (
            u16::from(actual.tricks),
            u16::from(actual.effective_announcements),
            u16::from(actual.pending_announcements),
        ) != (from_tricks, effective, pending)
            || u16::from(state.hand().eyes(player)) != from_tricks + effective
        {
            return Err(ValidationError(
                "eyes do not match cards and effective announcements",
            ));
        }
    }
    match state.config().variant {
        GameVariant::AustrianSchnapsen => validate_schnapsen_result(state)?,
    }
    Ok(())
}

// Reference outcomes from the published project rules. This deliberately
// does not call the engine's finish or scoring functions.
fn validate_schnapsen_result(state: &Match) -> Result<(), ValidationError> {
    let Some(result) = state.hand().result() else {
        if PLAYERS
            .iter()
            .any(|&player| state.hand().eyes(player) >= 66)
        {
            return Err(ValidationError("hand remained active after 66 eyes"));
        }
        return Ok(());
    };
    let outcome = result.outcome;
    let winner = result.score(outcome.winner);
    let loser = result.score(outcome.winner.other());
    let expected = match (outcome.reason, outcome.closure) {
        (FinishReason::Reached66, None) if winner.eyes >= 66 => reference_points(loser),
        (FinishReason::LastTrick, None) if winner.eyes < 66 && loser.eyes < 66 => 1,
        (FinishReason::SuccessfulClosure, Some(closure))
            if closure.closer == outcome.winner && winner.eyes >= 66 =>
        {
            reference_points(closure.opponent_at_closing)
        }
        (FinishReason::FailedClosure, Some(closure))
            if closure.closer != outcome.winner && loser.eyes < 66 =>
        {
            if closure.opponent_at_closing.tricks == 0 {
                3
            } else {
                2
            }
        }
        _ => {
            return Err(ValidationError(
                "hand finish reason contradicts the reference rules",
            ))
        }
    };
    if outcome.game_points != expected {
        return Err(ValidationError(
            "game-point award differs from the reference rules",
        ));
    }
    Ok(())
}

fn reference_points(score: Score) -> u8 {
    if score.eyes >= 33 {
        1
    } else if score.tricks == 0 {
        3
    } else {
        2
    }
}

/// Checks one actual engine transition against independent accounting and shape
/// invariants. The caller supplies the pre-action state to detect duplicate
/// awards even if each snapshot would be plausible in isolation.
pub fn validate_transition(
    before: &Match,
    action: &RecordedAction,
    after: &Match,
) -> Result<(), ValidationError> {
    validate(after)?;
    if after.config() != before.config() {
        return Err(ValidationError("active match configuration changed"));
    }

    let next_hand = matches!(action, RecordedAction::NextHand);
    if next_hand {
        if before.hand().result().is_none()
            || before.result().is_some()
            || after.forehand() != before.forehand().other()
            || after.hand().phase() != Phase::OpenTalon
            || !after.hand().announcements().is_empty()
            || PLAYERS.iter().any(|&player| after.hand().eyes(player) != 0)
        {
            return Err(ValidationError("invalid hand rotation or reset"));
        }
    } else if after.forehand() != before.forehand() {
        return Err(ValidationError("forehand changed in a running hand"));
    }

    let finish = if before.hand().result().is_none() {
        after.hand().result()
    } else {
        None
    };
    if finish.is_some()
        && !matches!(
            action,
            RecordedAction::Announce { .. } | RecordedAction::ResolveTrick
        )
    {
        return Err(ValidationError("unexpected hand completion"));
    }

    let mut remaining = PLAYERS.map(|player| before.remaining(player));
    let mut counters = PLAYERS.map(|player| before.bummerls(player));
    let mut expected_result = before.result();
    if matches!(action, RecordedAction::ResetBummerls) {
        counters = [0; 2];
    }
    if let Some(result) = finish {
        let winner = result.outcome.winner;
        let winner_index = winner as usize;
        let loser_index = winner.other() as usize;
        remaining[winner_index] =
            remaining[winner_index].saturating_sub(result.outcome.game_points);
        if remaining[winner_index] == 0 {
            // Austrian Schnapsen starts with seven points. This constant is
            // intentional as a reference check independent of the scoring code.
            let awarded =
                if before.schneider() == SchneiderRule::Double && remaining[loser_index] == 7 {
                    2_u8
                } else {
                    1
                };
            counters[loser_index] = counters[loser_index].saturating_add(u32::from(awarded));
            expected_result = Some(crate::core::match_play::MatchResult {
                winner,
                awarded_bummerls: awarded,
            });
        }
    }
    if PLAYERS.map(|player| after.remaining(player)) != remaining
        || PLAYERS.map(|player| after.bummerls(player)) != counters
        || after.result() != expected_result
        || after.completed_hands() != before.completed_hands() + u8::from(finish.is_some())
    {
        return Err(ValidationError(
            "hand, match or Bummerl was accounted incorrectly",
        ));
    }

    let table_before = before.hand().view(Player::Local).played;
    let table_after = after.hand().view(Player::Local).played;
    match *action {
        RecordedAction::ResolveTrick => validate_draws(before, after)?,
        RecordedAction::Exchange { player } => validate_exchange(before, player, after)?,
        RecordedAction::Close { player } => validate_closure(before, player, after)?,
        RecordedAction::ResetBummerls if before.hand() != after.hand() => {
            return Err(ValidationError("resetting Bummerl changed the hand"));
        }
        _ => {}
    }
    let played = match action {
        RecordedAction::Play { player, card } | RecordedAction::Announce { player, card, .. } => {
            Some((*player, *card))
        }
        _ => None,
    };
    if let Some(played) = played {
        if table_after.len() != table_before.len() + 1
            || !table_after.starts_with(table_before)
            || table_after.last() != Some(&played)
        {
            return Err(ValidationError(
                "played card did not append to the current trick",
            ));
        }
    }
    Ok(())
}

fn validate_draws(before: &Match, after: &Match) -> Result<(), ValidationError> {
    let before_view = before.hand().view(Player::Local);
    let after_view = after.hand().view(Player::Local);
    if before_view.played.len() != 2 || !after_view.played.is_empty() {
        return Err(ValidationError(
            "trick resolution did not consume exactly two cards",
        ));
    }
    let events = after
        .history()
        .entries()
        .get(before.history().entries().len()..)
        .ok_or(ValidationError("trick resolution removed history"))?;
    let Some(PublicEvent::TrickResolved { winner }) = events.first().map(|entry| &entry.public)
    else {
        return Err(ValidationError("missing trick resolution event"));
    };
    let draw = events
        .iter()
        .find(|entry| matches!(entry.public, PublicEvent::CardsDrawn { .. }));
    let must_draw = before.hand().phase() == Phase::OpenTalon && after.hand().result().is_none();
    if must_draw != draw.is_some() {
        return Err(ValidationError(
            "draw does not match the talon and finish state",
        ));
    }
    if let Some(entry) = draw {
        let order = [*winner, winner.other()];
        let (
            PublicEvent::CardsDrawn {
                order: recorded,
                exposed_trump,
            },
            Some(PrivateEvent::Draw { cards }),
        ) = (&entry.public, &entry.private)
        else {
            return Err(ValidationError("draw must record its cards"));
        };
        let expected_exposed = (before_view.hidden_talon_count == 1)
            .then_some(before_view.open_trump)
            .flatten();
        if *recorded != order
            || *exposed_trump != expected_exposed
            || expected_exposed.is_some_and(|card| card != cards[1])
        {
            return Err(ValidationError(
                "draw order or exposed trump differs from the reference rule",
            ));
        }
        for (player, card) in order.into_iter().zip(cards) {
            let old_hand = before.hand().view(player).hand;
            let new_hand = after.hand().view(player).hand;
            if new_hand.strip_prefix(old_hand) != Some(std::slice::from_ref(card)) {
                return Err(ValidationError(
                    "drawn card did not append to the correct hand",
                ));
            }
        }
    } else if PLAYERS
        .iter()
        .any(|&player| before.hand().view(player).hand != after.hand().view(player).hand)
    {
        return Err(ValidationError(
            "trick resolution changed a hand without a draw",
        ));
    }
    Ok(())
}

fn validate_exchange(before: &Match, player: Player, after: &Match) -> Result<(), ValidationError> {
    let exchange = before
        .hand()
        .trump_exchange(player)
        .map_err(|_| ValidationError("exchange was illegal in the previous state"))?;
    let mut expected = before.hand().view(player).hand.to_vec();
    let Some(slot) = expected.iter_mut().find(|card| **card == exchange.given) else {
        return Err(ValidationError("missing exchanged trump"));
    };
    *slot = exchange.received;
    let view = after.hand().view(player);
    if view.hand != expected
        || view.open_trump != Some(exchange.given)
        || view.hidden_talon_count != before.hand().view(player).hidden_talon_count
        || view.played != before.hand().view(player).played
        || after.hand().view(player.other()).hand != before.hand().view(player.other()).hand
        || after.hand().next_player() != Some(player)
    {
        return Err(ValidationError(
            "exchange changed more than the two swapped cards",
        ));
    }
    Ok(())
}

fn validate_closure(before: &Match, player: Player, after: &Match) -> Result<(), ValidationError> {
    before
        .hand()
        .check_close_talon(player)
        .map_err(|_| ValidationError("closure was illegal in the previous state"))?;
    let opponent = player.other();
    let expected = crate::core::game::TalonClosure {
        closer: player,
        opponent_at_closing: Score {
            eyes: before.hand().eyes(opponent),
            tricks: before.hand().view(opponent).own_trick_card_count / 2,
        },
    };
    if after.hand().phase() != Phase::ClosedTalon(expected)
        || after.hand().next_player() != Some(player)
        || PLAYERS
            .iter()
            .any(|&player| before.hand().view(player).hand != after.hand().view(player).hand)
    {
        return Err(ValidationError(
            "closure changed hands, turn or the frozen scoring baseline",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schnapsen_reference_awards_at_33_and_for_no_trick() {
        assert_eq!(reference_points(Score { eyes: 0, tricks: 0 }), 3);
        assert_eq!(
            reference_points(Score {
                eyes: 12,
                tricks: 1
            }),
            2
        );
        assert_eq!(
            reference_points(Score {
                eyes: 32,
                tricks: 3
            }),
            2
        );
        assert_eq!(
            reference_points(Score {
                eyes: 33,
                tricks: 3
            }),
            1
        );
    }
}
