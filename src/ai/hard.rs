use crate::core::{
    cards::{Card, Rank},
    game::{Phase, Player, Score, TalonClosure},
};

use super::{
    action_card, card_actions, marriage_eyes,
    medium::{tactic, useful_exchange},
    unplayed_opponent_candidates, Action, Observation, Strategy,
};

const SAMPLES: usize = 12;
const MAX_NODES_PER_FORECAST: usize = 256;
const SEARCH_TRICKS: u8 = 3;

pub(super) struct Hard;

impl Strategy for Hard {
    fn decide(&self, view: &Observation) -> Option<Action> {
        let candidates = unplayed_opponent_candidates(view);
        let actions: Vec<_> = card_actions(view).collect();
        if let Some(&action) = actions.iter().find(|&&action| {
            view.own_tricks > 0 && view.own_eyes + marriage_eyes(view, action) >= view.target_eyes()
        }) {
            return Some(action);
        }
        if useful_exchange(view) {
            return Some(Action::Exchange);
        }
        let fallback = || {
            actions
                .iter()
                .copied()
                .max_by_key(|&action| tactic(view, &candidates, action))
        };
        if actions.is_empty() {
            return None;
        }
        let hands = possible_hands(view, &candidates);
        if hands.is_empty() {
            return fallback();
        }

        let current_closure = if let Phase::ClosedTalon(closure) = view.phase {
            Some(closure)
        } else {
            None
        };
        let forced = view.phase != Phase::OpenTalon;
        let current = ForecastContext {
            view,
            closure: current_closure,
            forced,
        };
        let best = actions
            .iter()
            .copied()
            .map(|action| {
                let estimate = estimate(current, &hands, &candidates, action);
                // Keep the inexpensive tactical evaluation as a tie-break and
                // to account for future draw opportunities outside the horizon.
                (
                    estimate.value * 5 + tactic(view, &candidates, action),
                    action,
                )
            })
            .max_by_key(|&(value, _)| value);

        if let Some((open_value, action)) = best {
            if view.can_close && view.own_tricks > 0 {
                let closing = ForecastContext {
                    view,
                    forced: true,
                    closure: Some(TalonClosure {
                        closer: view.player,
                        opponent_at_closing: Score {
                            eyes: view.opponent_eyes,
                            tricks: view.opponent_tricks,
                        },
                    }),
                };
                let profitable = actions.iter().copied().any(|action| {
                    let estimate = estimate(closing, &hands, &candidates, action);
                    estimate.won == hands.len()
                        && estimate.value * 5 + tactic(view, &candidates, action) > open_value + 45
                });
                if profitable {
                    return Some(Action::Close);
                }
            }
            Some(action)
        } else {
            fallback()
        }
    }
}

fn possible_hands(view: &Observation, candidates: &[Card]) -> Vec<Vec<Card>> {
    let Some(unknown_count) = view
        .opponent_hand_count
        .checked_sub(view.known_opponent.len())
    else {
        return Vec::new();
    };
    if !view
        .known_opponent
        .iter()
        .all(|card| candidates.contains(card))
    {
        return Vec::new();
    }
    let mut pool: Vec<_> = candidates
        .iter()
        .copied()
        .filter(|card| !view.known_opponent.contains(card))
        .collect();
    if unknown_count > pool.len() {
        return Vec::new();
    }
    // The seed is generated from public data and the observer's own hand.
    // There is no persistent RNG state and the shuffle cannot leak hidden cards.
    let mut seed = 17_u32;
    for card in view
        .hand
        .iter()
        .chain(view.played.iter().map(|(_, card)| card))
    {
        seed = seed.wrapping_mul(16777619) ^ (1 + card.suit as u32 * 5 + card.rank as u32);
    }
    seed |= 1;
    for i in (1..pool.len()).rev() {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        pool.swap(i, seed as usize % (i + 1));
    }

    let mut indexes: Vec<_> = (0..unknown_count).collect();
    let mut combinations = Vec::new();
    loop {
        let mask = indexes
            .iter()
            .fold(0_u32, |mask, index| mask | (1 << index));
        combinations.push(mask);
        let Some(index) =
            (0..unknown_count).rfind(|&i| indexes[i] < pool.len() - unknown_count + i)
        else {
            break;
        };
        indexes[index] += 1;
        for i in index + 1..unknown_count {
            indexes[i] = indexes[i - 1] + 1;
        }
    }
    let total = combinations.len();
    let count = total.min(SAMPLES);
    (0..count)
        .map(|i| {
            let mask = combinations[(2 * i + 1) * total / (2 * count)];
            view.known_opponent
                .iter()
                .copied()
                .chain(
                    pool.iter()
                        .copied()
                        .enumerate()
                        .filter_map(|(index, card)| (mask & (1 << index) != 0).then_some(card)),
                )
                .collect()
        })
        .collect()
}

#[derive(Clone, Copy)]
struct ForecastContext<'a> {
    view: &'a Observation,
    closure: Option<TalonClosure>,
    forced: bool,
}

#[derive(Clone)]
struct Position {
    hands: [Vec<Card>; 2],
    scores: [Score; 2],
    pending: [u8; 2],
    leader: Player,
    lead: Option<Card>,
    last_draw: Option<[Card; 2]>,
}

impl Position {
    fn player(&self) -> Player {
        if self.lead.is_some() {
            self.leader.other()
        } else {
            self.leader
        }
    }
}

#[derive(Clone, Copy)]
struct Forecast {
    value: i32,
    winner: Option<Player>,
}

struct Estimate {
    value: i32,
    won: usize,
}

fn estimate(
    context: ForecastContext<'_>,
    samples: &[Vec<Card>],
    candidates: &[Card],
    action: Action,
) -> Estimate {
    let view = context.view;
    let own = view.player as usize;
    let opponent = view.player.other() as usize;
    let mut value = 0;
    let mut worst = i32::MAX;
    let mut won = 0;
    for sample in samples {
        let last_draw = if !context.forced && view.talon_count == 2 {
            candidates
                .iter()
                .copied()
                .find(|card| !sample.contains(card))
                .zip(view.open_trump)
                .map(|(hidden, trump)| [hidden, trump])
        } else {
            None
        };
        let (leader, lead) = view
            .played
            .first()
            .map_or((view.player, None), |&(player, card)| (player, Some(card)));
        let mut position = Position {
            hands: [Vec::new(), Vec::new()],
            scores: [Score { eyes: 0, tricks: 0 }; 2],
            pending: [0; 2],
            leader,
            lead,
            last_draw,
        };
        position.hands[own].clone_from(&view.hand);
        position.hands[opponent].clone_from(sample);
        position.scores[own] = Score {
            eyes: view.own_eyes,
            tricks: view.own_tricks,
        };
        position.scores[opponent] = Score {
            eyes: view.opponent_eyes,
            tricks: view.opponent_tricks,
        };
        position.pending[own] = view.own_pending_eyes;
        position.pending[opponent] = view.opponent_pending_eyes;
        let exact_tail = context.forced || last_draw.is_some();
        let mut search = Search {
            context,
            exact_tail,
            budget: MAX_NODES_PER_FORECAST,
        };
        let limit = if exact_tail {
            if context.closure.is_some() {
                view.hand.len() as u8
            } else {
                SEARCH_TRICKS
            }
        } else {
            1
        };
        let forecast = search.play(
            position,
            action_card(action),
            marriage_eyes(view, action),
            limit,
            i32::MIN,
            i32::MAX,
        );
        value += forecast.value;
        worst = worst.min(forecast.value);
        won += usize::from(forecast.winner == Some(view.player));
    }
    Estimate {
        value: (value * 4 / samples.len() as i32 + worst) / 5,
        won,
    }
}

struct Search<'a> {
    context: ForecastContext<'a>,
    exact_tail: bool,
    budget: usize,
}

impl Search<'_> {
    fn play(
        &mut self,
        mut position: Position,
        card: Card,
        announcement: u8,
        depth: u8,
        alpha: i32,
        beta: i32,
    ) -> Forecast {
        let view = self.context.view;
        let player = position.player();
        let own = player as usize;
        let index = position.hands[own]
            .iter()
            .position(|&held| held == card)
            .expect("model only evaluates cards in its hypothetical hand");
        position.hands[own].swap_remove(index);
        if position.scores[own].tricks > 0 {
            position.scores[own].eyes += announcement;
        } else {
            position.pending[own] += announcement;
        }
        if position.scores[own].eyes >= view.target_eyes() {
            return self.finish(&position, player, None);
        }
        if let Some(lead) = position.lead.take() {
            let winner = if view.beats(card, lead) {
                player
            } else {
                position.leader
            };
            let score = &mut position.scores[winner as usize];
            score.eyes += lead.points() + card.points() + position.pending[winner as usize];
            position.pending[winner as usize] = 0;
            score.tricks += 1;
            position.leader = winner;
            if score.eyes >= view.target_eyes() {
                return self.finish(&position, winner, None);
            }
            if let Some([hidden, trump]) = position.last_draw.take() {
                position.hands[winner as usize].push(hidden);
                position.hands[winner.other() as usize].push(trump);
            }
            if position.hands.iter().all(Vec::is_empty) {
                return if let Some(closure) = self.context.closure {
                    self.finish(&position, closure.closer.other(), None)
                } else {
                    self.finish(&position, winner, Some(view.last_trick_points()))
                };
            }
            self.search(position, depth - 1, alpha, beta)
        } else {
            position.lead = Some(card);
            self.search(position, depth, alpha, beta)
        }
    }

    fn search(&mut self, position: Position, depth: u8, mut alpha: i32, mut beta: i32) -> Forecast {
        if self.budget == 0 || depth == 0 {
            return self.evaluate(&position);
        }
        self.budget -= 1;
        let player = position.player();
        let hand = &position.hands[player as usize];
        let forced = self.exact_tail && position.last_draw.is_none();
        let moves = if let Some(lead) = position.lead {
            self.context.view.hypothetical_replies(hand, lead, forced)
        } else {
            hand.clone()
        };
        let maximizing = player == self.context.view.player;
        let mut best = None::<Forecast>;
        for card in moves {
            let candidate = self.play(position.clone(), card, 0, depth, alpha, beta);
            if best.is_none_or(|previous| {
                if maximizing {
                    candidate.value > previous.value
                } else {
                    candidate.value < previous.value
                }
            }) {
                best = Some(candidate);
            }
            if maximizing {
                alpha = alpha.max(candidate.value);
            } else {
                beta = beta.min(candidate.value);
            }
            if alpha >= beta {
                break;
            }
        }
        best.unwrap_or_else(|| self.evaluate(&position))
    }

    fn finish(&self, position: &Position, winner: Player, fixed_points: Option<u8>) -> Forecast {
        let view = self.context.view;
        let points = fixed_points.unwrap_or_else(|| {
            if let Some(closure) = self.context.closure {
                let (success, failure) = view.closure_points_at_end(
                    closure.opponent_at_closing,
                    position.scores[closure.closer.other() as usize],
                );
                if closure.closer == winner {
                    success
                } else {
                    failure
                }
            } else {
                view.winning_points(position.scores[winner.other() as usize])
            }
        });
        let match_award = view.match_award(winner, points);
        let utility = 260
            + 32 * i32::from(points)
            + if match_award > 0 { 110 } else { 0 }
            + if match_award > 1 { 72 } else { 0 };
        Forecast {
            value: if winner == view.player {
                utility
            } else {
                -utility
            },
            winner: Some(winner),
        }
    }

    fn evaluate(&self, position: &Position) -> Forecast {
        let own = self.context.view.player as usize;
        let other = self.context.view.player.other() as usize;
        let value = |side: usize, hand: &[Card], rival: &[Card]| {
            let eyes = i32::from(position.scores[side].eyes);
            let hand_power: i32 = hand
                .iter()
                .map(|&card| {
                    let trump = card.suit == self.context.view.trump;
                    let master = !rival
                        .iter()
                        .any(|&other| self.context.view.beats(other, card));
                    let known_control = if master { 13 } else { 0 };
                    i32::from(card.points()) / 2 + if trump { 8 } else { 0 } + known_control
                })
                .sum();
            let pair_power: i32 = hand
                .iter()
                .filter(|card| card.rank == Rank::King)
                .map(|card| {
                    if hand.contains(&Card {
                        suit: card.suit,
                        rank: Rank::Ober,
                    }) {
                        if card.suit == self.context.view.trump {
                            17
                        } else {
                            10
                        }
                    } else {
                        0
                    }
                })
                .sum();
            let no_trick_risk = if position.scores[side].tricks == 0 {
                -16
            } else {
                0
            };
            eyes * 3
                + (eyes - 35).max(0) * 2
                + i32::from(position.pending[side]) * 2
                + hand_power
                + pair_power
                + no_trick_risk
        };
        let lead_bonus = if position.leader == self.context.view.player {
            11
        } else {
            -11
        };
        Forecast {
            value: value(own, &position.hands[own], &position.hands[other])
                - value(other, &position.hands[other], &position.hands[own])
                + lead_bonus,
            winner: None,
        }
    }
}
