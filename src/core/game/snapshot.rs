use std::collections::HashSet;

use super::*;

impl Game {
    pub(in crate::core) fn validate_snapshot(&self, forehand: Player) -> bool {
        let deck = self.game_variant.deck();
        let cards: Vec<_> = self
            .hands
            .iter()
            .flatten()
            .chain(&self.talon)
            .chain(self.played.iter().map(|(_, card)| card))
            .chain(self.tricks.iter().flatten().flatten())
            .copied()
            .collect();
        if cards.len() != deck.len()
            || cards.into_iter().collect::<HashSet<_>>() != deck.iter().copied().collect()
            || self.played.len() > 2
            || self.trick_winners.len() != self.trick_index()
            || self
                .talon
                .last()
                .is_some_and(|card| card.suit != self.trump_suit)
        {
            return false;
        }

        let hand_size = self.game_variant.hand_size();
        let initial_talon = deck.len() - hand_size * self.game_variant.player_count();
        let completed = self.trick_index();
        let closure = self.talon_closure();
        if self.talon.len() > initial_talon || !self.talon.len().is_multiple_of(2) {
            return false;
        }
        let drawn = (initial_talon - self.talon.len()) / 2;
        if closure.is_none() {
            // An open-talon hand that finishes on a full trick awards it before drawing.
            let without_final_draw = matches!(self.phase, Phase::Finished(_))
                && !self.talon.is_empty()
                && self.played.is_empty();
            if drawn
                != (completed.saturating_sub(usize::from(without_final_draw)))
                    .min(initial_talon / 2)
            {
                return false;
            }
        }
        let mut pending = [0; 2];
        for &(player, _) in &self.played {
            pending[player as usize] += 1;
        }
        if drawn > completed
            || [Player::Local, Player::Opponent].into_iter().any(|player| {
                let index = player as usize;
                self.hands[index].len() + pending[index] + completed != hand_size + drawn
            })
        {
            return false;
        }

        let Some(plays) = self.snapshot_plays(forehand) else {
            return false;
        };
        self.validate_public_memory(forehand, &plays, drawn)
            && self.validate_snapshot_timeline(forehand, &plays, drawn)
    }

    // Per-winner piles preserve the pair order but rely on trick_winners to
    // recover the chronological order. The opening player comes from Match.
    fn snapshot_plays(&self, forehand: Player) -> Option<Vec<(Player, Card)>> {
        let mut piles = self.tricks.each_ref().map(|tricks| tricks.iter());
        let mut leader = forehand;
        let mut plays = Vec::with_capacity(2 * self.trick_index() + self.played.len());
        for &winner in &self.trick_winners {
            let &[lead, reply] = piles[winner as usize].next()?;
            let computed_winner = if self.game_variant.beats(reply, lead, self.trump_suit) {
                leader.other()
            } else {
                leader
            };
            if winner != computed_winner {
                return None;
            }
            plays.extend([(leader, lead), (leader.other(), reply)]);
            leader = winner;
        }
        if self.leader != leader
            || self
                .played
                .first()
                .is_some_and(|&(player, _)| player != leader)
            || self
                .played
                .get(1)
                .is_some_and(|&(player, _)| player != leader.other())
        {
            return None;
        }
        plays.extend_from_slice(&self.played);
        Some(plays)
    }

    fn validate_snapshot_timeline(
        &self,
        forehand: Player,
        plays: &[(Player, Card)],
        drawn: usize,
    ) -> bool {
        let mut history = Game {
            hands: [Vec::new(), Vec::new()],
            talon: self.talon.clone(),
            trump_suit: self.trump_suit,
            leader: forehand,
            played: Vec::new(),
            tricks: [Vec::new(), Vec::new()],
            trick_winners: Vec::new(),
            game_variant: self.game_variant,
            profile: self.profile,
            announcements: Vec::new(),
            phase: Phase::OpenTalon,
            public_memory: PublicMemory::default(),
        };
        let completed = self.trick_index();
        for index in 0..=completed {
            if index == drawn {
                if let Some(closure) = self.talon_closure() {
                    if history.close_talon(closure.closer).is_err()
                        || history.talon_closure() != Some(closure)
                    {
                        return false;
                    }
                } else if self.talon.is_empty() {
                    history.phase = Phase::EmptyTalon;
                }
            }

            let Some(&(leader, lead)) = plays.get(index * 2) else {
                break;
            };
            let event = self
                .announcements
                .get(history.announcements.len())
                .filter(|event| event.played == lead);
            if let Some(&event) = event {
                if !self.validate_snapshot_announcement(
                    &mut history,
                    event,
                    leader,
                    &plays[index * 2 + 1..],
                ) {
                    return false;
                }
            }
            history.played.push((leader, lead));
            if event.is_some() && history.finish_at_66(leader) {
                return index == completed
                    && self.played.len() == 1
                    && history.phase == self.phase
                    && history.announcements == self.announcements;
            }

            let Some(&(follower, reply)) = plays.get(index * 2 + 1) else {
                break;
            };
            if self.game_variant.enforce_reply_obligations(history.phase) {
                // Once there are no further draws or exchanges, the current
                // hand and the player's subsequent cards reconstruct the hand
                // at the time of this reply.
                history.hands[follower as usize] = self
                    .snapshot_hand(follower, &plays[index * 2 + 1..])
                    .collect();
                if !history.legal_cards(follower).contains(&reply) {
                    return false;
                }
            }
            history.played.clear();
            if index == completed {
                break;
            }
            let winner = self.trick_winners[index];
            history.tricks[winner as usize].push([lead, reply]);
            history.leader = winner;
            if history.finish_at_66(winner) || history.finish_at_66(winner.other()) {
                return index + 1 == completed
                    && self.played.is_empty()
                    && history.phase == self.phase
                    && history.announcements == self.announcements;
            }
        }

        if self.hands.iter().all(Vec::is_empty) && self.played.is_empty() {
            if let Some(closure) = history.talon_closure() {
                history.finish_failed_closure(closure);
            } else {
                history.phase = Phase::Finished(Outcome {
                    winner: history.leader,
                    reason: FinishReason::LastTrick,
                    game_points: history.game_variant.last_trick_points(),
                    closure: None,
                });
            }
        }
        history.phase == self.phase && history.announcements == self.announcements
    }

    fn snapshot_hand<'a>(
        &'a self,
        player: Player,
        plays: &'a [(Player, Card)],
    ) -> impl Iterator<Item = Card> + 'a {
        self.hands[player as usize].iter().copied().chain(
            plays
                .iter()
                .filter_map(move |&(owner, card)| (player == owner).then_some(card)),
        )
    }

    fn validate_snapshot_announcement(
        &self,
        history: &mut Game,
        event: Announcement,
        leader: Player,
        later_plays: &[(Player, Card)],
    ) -> bool {
        if event.player != leader
            || !self
                .snapshot_hand(leader, later_plays)
                .any(|card| card == event.partner())
        {
            return false;
        }
        history.hands[leader as usize] = vec![event.played, event.partner()];
        if history.marriage(leader, event.marriage.suit) != Ok(event.marriage)
            || (event.played != event.marriage.king()
                && !(event.marriage.may_play_ober && event.played == event.marriage.ober()))
        {
            return false;
        }
        history.announcements.push(event);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_rejects_duplicate_cards_and_inconsistent_trick_or_closure() {
        let player = Player::Local;
        let mut game = Game::new(player, |upper| upper - 1);
        let mut duplicate = game.clone();
        duplicate.hands[0][0] = duplicate.hands[1][0];
        assert!(!duplicate.validate_snapshot(player));

        let lead = game.hands[0][0];
        let reply = game.hands[1][0];
        game.play_card(player, lead).unwrap();
        game.play_card(player.other(), reply).unwrap();
        let mut wrong_turn = game.clone();
        wrong_turn.played[1].0 = player;
        assert!(!wrong_turn.validate_snapshot(player));

        let winner = game.resolve_trick().unwrap();
        let mut wrong_winner = game.clone();
        wrong_winner.trick_winners[0] = winner.other();
        assert!(!wrong_winner.validate_snapshot(player));

        game.close_talon(winner).unwrap();
        assert!(game.validate_snapshot(player));
        let Phase::ClosedTalon(mut forged) = game.phase else {
            unreachable!();
        };
        forged.opponent_at_closing.eyes += 1;
        game.phase = Phase::ClosedTalon(forged);
        assert!(!game.validate_snapshot(player));
    }
}
