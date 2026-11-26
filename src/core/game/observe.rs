use super::*;
use crate::core::{
    observation::{Observation, ObservedTrick},
    rules::MatchConfig,
};

impl Game {
    pub(in crate::core) fn observation(
        &self,
        player: Player,
        forehand: Player,
        remaining: [u8; 2],
        config: MatchConfig,
    ) -> Observation {
        let view = self.view(player);
        let tricks = self.observed_tricks(forehand);
        let opponent = player.other();
        let known_talon_trump = self.talon.last().copied();
        let known_opponent = self.known_cards(opponent, &tricks, view.hand, known_talon_trump);
        let own = self.eye_breakdown(player);
        let opposing = self.eye_breakdown(opponent);
        Observation {
            player,
            variant: self.game_variant,
            profile: self.profile,
            phase: self.phase,
            hand: view.hand.to_vec(),
            trump: self.trump_suit,
            open_trump: view.open_trump,
            known_talon_trump,
            played: self.played.clone(),
            tricks,
            announcements: self.announcements.clone(),
            exchanges: self.public_memory.exchanges.clone(),
            known_opponent,
            opponent_hand_count: view.opponent_hand_count,
            talon_count: self.talon.len(),
            own_eyes: own.tricks + own.effective_announcements,
            opponent_eyes: opposing.tricks + opposing.effective_announcements,
            own_pending_eyes: own.pending_announcements,
            opponent_pending_eyes: opposing.pending_announcements,
            own_tricks: self.tricks[player as usize].len(),
            opponent_tricks: self.tricks[opponent as usize].len(),
            remaining,
            schneider: config.schneider,
            legal_cards: self.legal_cards(player),
            legal_marriages: self.legal_marriages(player),
            exchange: self.trump_exchange(player).ok(),
            can_close: self.check_close_talon(player).is_ok(),
        }
    }

    fn observed_tricks(&self, forehand: Player) -> Vec<ObservedTrick> {
        let mut lead_player = forehand;
        let mut piles = self.tricks.each_ref().map(|tricks| tricks.iter());
        let first_forced = self.tricks_with_draws();
        self.trick_winners
            .iter()
            .enumerate()
            .map(|(index, &winner)| {
                let cards = *piles[winner as usize]
                    .next()
                    .expect("each recorded winner has a trick");
                let trick = ObservedTrick {
                    lead_player,
                    winner,
                    cards,
                    forced: index >= first_forced,
                };
                lead_player = winner;
                trick
            })
            .collect()
    }

    fn initial_talon_count(&self) -> usize {
        self.game_variant.deck().len()
            - self.game_variant.hand_size() * self.game_variant.player_count()
    }

    fn tricks_with_draws(&self) -> usize {
        if self.talon_closure().is_some() || self.talon.is_empty() {
            (self.initial_talon_count() - self.talon.len()) / 2
        } else {
            // If an open-talon hand ends before a draw, that final trick
            // retains open rules.
            self.trick_winners.len()
        }
    }

    fn known_cards(
        &self,
        owner: Player,
        tricks: &[ObservedTrick],
        own_hand: &[Card],
        known_talon_trump: Option<Card>,
    ) -> Vec<Card> {
        let from_announcements = self
            .announcements
            .iter()
            .filter(|event| event.player == owner)
            .map(|event| event.partner());
        let from_exchanges = self
            .public_memory
            .exchanges
            .iter()
            .filter(|event| event.exchange.player == owner)
            .map(|event| event.exchange.received);
        let final_draw_index = self.initial_talon_count() / 2 - 1;
        let public_draw = self
            .public_memory
            .last_trump
            .filter(|_| self.trick_winners.get(final_draw_index) == Some(&owner.other()));

        let mut known = Vec::new();
        for card in from_announcements.chain(from_exchanges).chain(public_draw) {
            if known_talon_trump != Some(card)
                && !own_hand.contains(&card)
                && !self.played.iter().any(|&(_, played)| played == card)
                && !tricks.iter().any(|trick| trick.cards.contains(&card))
                && !known.contains(&card)
            {
                known.push(card);
            }
        }
        known
    }

    pub(super) fn validate_public_memory(
        &self,
        forehand: Player,
        plays: &[(Player, Card)],
        drawn: usize,
    ) -> bool {
        fn holds_or_played_since(
            game: &Game,
            player: Player,
            card: Card,
            plays: &[(Player, Card)],
            index: usize,
        ) -> bool {
            game.hands[player as usize].contains(&card)
                || plays[index * 2..].contains(&(player, card))
        }

        let full_draws = self.initial_talon_count() / 2;
        if let Some(last) = self.public_memory.last_trump {
            if drawn != full_draws
                || last.suit != self.trump_suit
                || !holds_or_played_since(
                    self,
                    self.trick_winners[full_draws - 1].other(),
                    last,
                    plays,
                    full_draws,
                )
            {
                return false;
            }
        }

        match self.public_memory.exchanges.as_slice() {
            [] => true,
            [event] => {
                // The exchanged Unter stays at the bottom until the final draw,
                // so a physical deck permits at most one exchange per hand.
                let index = event.completed_tricks;
                let exchange = event.exchange;
                let rules = self.rules();
                if index < rules.minimum_completed_tricks_for_talon_action
                    || index > drawn
                    || self.initial_talon_count() - 2 * index < rules.minimum_talon_cards_for_action
                {
                    return false;
                }
                let leader = index
                    .checked_sub(1)
                    .map_or(forehand, |index| self.trick_winners[index]);
                exchange.player == leader
                    && exchange.given == self.trump_under()
                    && exchange.received.suit == self.trump_suit
                    && exchange.received != exchange.given
                    && holds_or_played_since(self, leader, exchange.received, plays, index)
                    && if drawn < full_draws {
                        self.talon.last() == Some(&exchange.given)
                    } else {
                        self.public_memory.last_trump == Some(exchange.given)
                    }
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn covered_card_permutations_do_not_change_observation() {
        let game = Game::new(Player::Local, |upper| upper - 1);
        let mut altered = game.clone();
        std::mem::swap(&mut altered.hands[1][0], &mut altered.talon[0]);
        altered.talon.swap(1, 2);
        let observe = |game: &Game| {
            game.observation(Player::Local, Player::Local, [7, 7], MatchConfig::default())
        };
        assert_ne!(game, altered);
        assert_eq!(observe(&game), observe(&altered));
        let observation = observe(&game);
        assert_eq!(observation.known_opponent, []);
        assert!(observation.could_hold(Player::Opponent, game.hands[1][0]));
        assert!(observation.could_hold(Player::Opponent, game.talon[0]));
        assert!(!observation.could_hold(Player::Opponent, *game.talon.last().unwrap()));
    }

    #[test]
    fn exchange_is_remembered_after_restoring_a_snapshot() {
        let mut game = Game::new(
            Player::Local,
            |upper| if upper == 10 { 7 } else { upper - 1 },
        );
        let cards = crate::core::cards::full_deck();
        game.play_card(Player::Local, cards[0]).unwrap();
        game.play_card(Player::Opponent, cards[3]).unwrap();
        game.resolve_trick().unwrap();
        let exchange = game.exchange_trump(Player::Local).unwrap();
        let bytes = serde_json::to_vec(&game).unwrap();
        let mut restored: Game = serde_json::from_slice(&bytes).unwrap();
        assert!(restored.validate_snapshot(Player::Local));
        let seen = restored.observation(
            Player::Opponent,
            Player::Local,
            [7, 7],
            MatchConfig::default(),
        );
        assert_eq!(seen.known_opponent, [exchange.received]);
        assert_eq!(seen.exchanges[0].exchange, exchange);
        assert_eq!(seen.known_talon_trump, Some(exchange.given));

        restored.public_memory.exchanges[0].completed_tricks = 3;
        assert!(!restored.validate_snapshot(Player::Local));
    }

    #[test]
    fn public_deductions_keep_the_actual_opponent_cards_possible() {
        for profile in [RuleProfile::Soft, RuleProfile::Sharp] {
            for seed in 1..=16 {
                let mut seed_state: u32 = seed;
                let mut game = Game::with_variant(Player::Local, profile, |upper| {
                    seed_state ^= seed_state << 13;
                    seed_state ^= seed_state >> 17;
                    seed_state ^= seed_state << 5;
                    seed_state as usize % upper
                });
                while let Some(player) = game.next_player() {
                    let seen = game.observation(
                        player,
                        Player::Local,
                        [7, 7],
                        MatchConfig {
                            profile,
                            ..MatchConfig::default()
                        },
                    );
                    let opposing = &game.hands[player.other() as usize];
                    assert!(opposing
                        .iter()
                        .all(|&card| seen.could_hold(player.other(), card)));
                    assert!(seen
                        .known_opponent
                        .iter()
                        .all(|card| opposing.contains(card)));
                    if seen.can_close && seed % 2 == 0 {
                        game.close_talon(player).unwrap();
                    }
                    if game.trump_exchange(player).is_ok() {
                        game.exchange_trump(player).unwrap();
                    }
                    if let Some(marriage) = game.legal_marriages(player).first() {
                        game.announce_and_play(player, marriage.suit, marriage.king())
                            .unwrap();
                    } else {
                        let card = game.legal_cards(player)[0];
                        game.play_card(player, card).unwrap();
                    }
                    if game.played.len() == 2 {
                        game.resolve_trick().unwrap();
                    }
                }
            }
        }
    }
}
