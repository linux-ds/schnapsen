//! Strategy dispatch receives the same public observation at every level.

mod easy;
mod hard;
mod medium;

use crate::core::{
    cards::{Card, Suit},
    game::{
        Announcement, ClaimError, CloseError, Exchange, ExchangeError, Marriage, MarriageError,
        PlayError, Player,
    },
    match_play::Match,
    observation::Observation,
    rules::Difficulty,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Play(Card),
    Announce { suit: Suit, card: Card },
    Exchange,
    Close,
    Claim,
    ContinueClaim,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionEvent {
    Played,
    Announced(Announcement),
    Exchanged(Exchange),
    Closed,
    Claimed,
    ClaimContinued,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionError {
    Play(PlayError),
    Announce(MarriageError),
    Exchange(ExchangeError),
    Close(CloseError),
    Claim(ClaimError),
}

trait Strategy: Sync {
    fn decide(&self, view: &Observation) -> Option<Action>;
}

struct Registration {
    difficulty: Difficulty,
    id: &'static str,
    strategy: &'static dyn Strategy,
}

const STRATEGIES: [Registration; 3] = [
    Registration {
        difficulty: Difficulty::Easy,
        id: "first_legal",
        strategy: &easy::Easy,
    },
    Registration {
        difficulty: Difficulty::Medium,
        id: "tactical",
        strategy: &medium::Medium,
    },
    Registration {
        difficulty: Difficulty::Hard,
        id: "foresight",
        strategy: &hard::Hard,
    },
];

fn registration(difficulty: Difficulty) -> &'static Registration {
    STRATEGIES
        .iter()
        .find(|entry| entry.difficulty == difficulty)
        .expect("every difficulty has a registered strategy")
}

pub fn strategy_id(difficulty: Difficulty) -> &'static str {
    registration(difficulty).id
}

pub fn decide(difficulty: Difficulty, view: &Observation) -> Option<Action> {
    if let Some(player) = view.claim_player {
        return (player == view.player).then(|| {
            if view.own_eyes >= view.target_eyes() {
                Action::Claim
            } else {
                Action::ContinueClaim
            }
        });
    }
    registration(difficulty).strategy.decide(view)
}

pub fn apply(
    session: &mut Match,
    player: Player,
    action: Action,
) -> Result<ActionEvent, ActionError> {
    match action {
        Action::Claim => session
            .claim(player)
            .map(|_| ActionEvent::Claimed)
            .map_err(ActionError::Claim),
        Action::ContinueClaim => session
            .continue_claim(player)
            .map(|_| ActionEvent::ClaimContinued)
            .map_err(ActionError::Claim),
        Action::Play(card) => session
            .play_card(player, card)
            .map(|()| ActionEvent::Played)
            .map_err(ActionError::Play),
        Action::Announce { suit, card } => session
            .announce_and_play(player, suit, card)
            .map(ActionEvent::Announced)
            .map_err(ActionError::Announce),
        Action::Exchange => session
            .exchange_trump(player)
            .map(ActionEvent::Exchanged)
            .map_err(ActionError::Exchange),
        Action::Close => session
            .close_talon(player)
            .map(|()| ActionEvent::Closed)
            .map_err(ActionError::Close),
    }
}

pub fn wants_exchange(legal: Exchange) -> bool {
    legal.received.points() > legal.given.points()
}

pub fn choose_marriage(legal_marriages: &[Marriage]) -> Option<Marriage> {
    legal_marriages.first().copied()
}

pub fn choose_card(legal_cards: &[Card]) -> Option<Card> {
    legal_cards.first().copied()
}

fn card_actions(view: &Observation) -> impl Iterator<Item = Action> + '_ {
    view.legal_cards
        .iter()
        .copied()
        .map(Action::Play)
        .chain(view.legal_marriages.iter().flat_map(|marriage| {
            [
                Some(marriage.king()),
                marriage.may_play_ober.then(|| marriage.ober()),
            ]
            .into_iter()
            .flatten()
            .map(|card| Action::Announce {
                suit: marriage.suit,
                card,
            })
        }))
}

fn action_card(action: Action) -> Card {
    match action {
        Action::Play(card) | Action::Announce { card, .. } => card,
        _ => unreachable!("expected a card action"),
    }
}

fn marriage_eyes(view: &Observation, action: Action) -> u8 {
    match action {
        Action::Announce { suit, .. } => view
            .legal_marriages
            .iter()
            .find(|entry| entry.suit == suit)
            .map_or(0, |entry| entry.eyes),
        _ => 0,
    }
}

fn unplayed_opponent_candidates(view: &Observation) -> Vec<Card> {
    view.variant
        .deck()
        .into_iter()
        .filter(|card| {
            !view.hand.contains(card)
                && Some(*card) != view.known_talon_trump
                && !view.played.iter().any(|&(_, played)| played == *card)
                && !view.tricks.iter().any(|trick| trick.cards.contains(card))
                && view.could_hold(view.player.other(), *card)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{
        cards::Rank,
        game::{Game, Phase},
        rules::{RuleProfile, SchneiderRule},
    };

    #[test]
    fn each_strategy_claims_only_with_enough_eyes_in_a_manual_window() {
        use crate::core::{
            game::Player,
            rules::{MatchConfig, RuleProfile},
        };
        let mut saw_continue = false;
        let mut saw_claim = false;
        for mut seed in 1..=16_u32 {
            let mut session = Match::with_config(
                Player::Local,
                MatchConfig {
                    profile: RuleProfile::ClassicSoft,
                    ..MatchConfig::default()
                },
                |upper| {
                    seed ^= seed << 13;
                    seed ^= seed >> 17;
                    seed ^= seed << 5;
                    seed as usize % upper
                },
            );
            while session.hand().result().is_none() {
                if let Some(player) = session.claim_player() {
                    let view = session.observation(player);
                    let expected = if view.own_eyes >= 66 {
                        saw_claim = true;
                        Action::Claim
                    } else {
                        saw_continue = true;
                        Action::ContinueClaim
                    };
                    for difficulty in Difficulty::ALL {
                        assert_eq!(decide(difficulty, &view), Some(expected));
                        assert_eq!(
                            decide(difficulty, &session.observation(player.other())),
                            None
                        );
                    }
                    apply(&mut session, player, expected).unwrap();
                } else if let Some(player) = session.hand().next_player() {
                    let card = session.hand().legal_cards(player)[0];
                    session.play_card(player, card).unwrap();
                } else {
                    session.resolve_trick().unwrap();
                }
            }
            if saw_continue && saw_claim {
                break;
            }
        }
        assert!(saw_continue && saw_claim);
    }

    #[test]
    fn chooses_first_legal_card_only_on_its_turn() {
        let mut game = Game::new(Player::Local, |upper| upper - 1);
        assert_eq!(choose_card(&game.legal_cards(Player::Opponent)), None);
        let lead = game.legal_cards(Player::Local)[0];
        game.play_card(Player::Local, lead).unwrap();
        let legal = game.legal_cards(Player::Opponent);
        assert_eq!(choose_card(&legal), Some(legal[0]));
        game.play_card(Player::Opponent, legal[0]).unwrap();
        assert_eq!(choose_card(&game.legal_cards(Player::Opponent)), None);
    }

    fn observed_reply(eyes: u8) -> (Observation, Card, Card) {
        let game = Match::new(
            Player::Opponent,
            RuleProfile::Soft,
            SchneiderRule::Double,
            |_| 0,
        );
        let mut view = game.observation(Player::Local);
        let lead = Card {
            suit: Suit::Leaves,
            rank: Rank::Ace,
        };
        let low = Card {
            suit: Suit::Bells,
            rank: Rank::Unter,
        };
        let high = Card {
            suit: Suit::Bells,
            rank: Rank::Ace,
        };
        view.hand = vec![low, high];
        view.legal_cards.clone_from(&view.hand);
        view.legal_marriages.clear();
        view.phase = Phase::OpenTalon;
        view.talon_count = 4;
        view.trump = Suit::Bells;
        view.open_trump = Some(Card {
            suit: Suit::Bells,
            rank: Rank::Ober,
        });
        view.known_talon_trump = view.open_trump;
        view.played = vec![(Player::Opponent, lead)];
        view.own_eyes = eyes;
        view.own_tricks = 2;
        (view, low, high)
    }

    #[test]
    fn medium_keeps_a_high_trump_if_a_low_one_wins_the_same_trick() {
        let (view, low, _) = observed_reply(30);
        assert_eq!(decide(Difficulty::Medium, &view), Some(Action::Play(low)));
    }

    #[test]
    fn all_strategies_take_a_trick_that_reaches_the_finish_threshold() {
        let (view, _, high) = observed_reply(49);
        for difficulty in Difficulty::ALL {
            assert_eq!(decide(difficulty, &view), Some(Action::Play(high)));
        }
    }

    #[test]
    fn hard_closes_only_with_reliable_winners_at_equal_score() {
        let (mut view, _, _) = observed_reply(39);
        view.played.clear();
        view.hand = vec![
            Card {
                suit: Suit::Bells,
                rank: Rank::Ace,
            },
            Card {
                suit: Suit::Bells,
                rank: Rank::Ten,
            },
            Card {
                suit: Suit::Bells,
                rank: Rank::King,
            },
            Card {
                suit: Suit::Leaves,
                rank: Rank::Ace,
            },
            Card {
                suit: Suit::Hearts,
                rank: Rank::Ace,
            },
        ];
        view.legal_cards.clone_from(&view.hand);
        view.open_trump = Some(Card {
            suit: Suit::Bells,
            rank: Rank::Unter,
        });
        view.known_talon_trump = view.open_trump;
        view.opponent_eyes = 20;
        view.opponent_tricks = 1;
        view.can_close = true;
        assert_eq!(decide(Difficulty::Hard, &view), Some(Action::Close));

        view.hand = vec![
            Card {
                suit: Suit::Leaves,
                rank: Rank::Ober,
            },
            Card {
                suit: Suit::Leaves,
                rank: Rank::Unter,
            },
            Card {
                suit: Suit::Hearts,
                rank: Rank::Ober,
            },
            Card {
                suit: Suit::Hearts,
                rank: Rank::Unter,
            },
            Card {
                suit: Suit::Acorns,
                rank: Rank::Unter,
            },
        ];
        view.legal_cards.clone_from(&view.hand);
        assert!(matches!(
            decide(Difficulty::Hard, &view),
            Some(Action::Play(_))
        ));
    }

    #[test]
    fn each_difficulty_resolves_to_a_distinct_stable_strategy() {
        assert_eq!(
            Difficulty::ALL.map(strategy_id),
            ["first_legal", "tactical", "foresight"]
        );
    }
}
