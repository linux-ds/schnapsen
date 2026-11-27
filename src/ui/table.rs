use schnapsen::{
    ai,
    core::{
        cards::{Card, Suit},
        game::{
            Announcement, CloseError, Exchange, ExchangeError, HandResult, Marriage, PlayError,
            Player, View,
        },
        match_play::{DealError, Match},
        rules::MatchConfig,
    },
};

use super::design::CardDesign;

pub(super) struct Table {
    session: Match,
    design: CardDesign,
    selected: Vec<Card>,
    focus: Option<Card>,
    show_legal: bool,
    show_opponent_eyes: bool,
    first_click: Option<(Card, Option<[Card; 2]>)>,
    play_error: Option<PlayError>,
    visible_announcement: Option<Announcement>,
    visible_exchange: Option<Exchange>,
    exchange_error: Option<ExchangeError>,
}

impl Table {
    pub fn new(session: Match) -> Self {
        let focus = session.hand().view(Player::Local).hand.first().copied();
        Self {
            session,
            design: CardDesign::default(),
            selected: Vec::new(),
            focus,
            show_legal: false,
            show_opponent_eyes: false,
            first_click: None,
            play_error: None,
            visible_announcement: None,
            visible_exchange: None,
            exchange_error: None,
        }
    }

    pub fn selection(&self) -> &[Card] {
        &self.selected
    }

    pub fn restore_selection(&mut self, selection: Vec<Card>) {
        self.focus = selection.first().copied().or(self.focus);
        self.selected = selection;
    }

    #[cfg(test)]
    pub fn into_session(self) -> Match {
        self.session
    }

    pub fn session(&self) -> &Match {
        &self.session
    }

    pub fn next_hand(&mut self, choose_index: impl FnMut(usize) -> usize) -> Result<(), DealError> {
        self.session.next_hand(choose_index)?;
        self.reset_hand_ui();
        Ok(())
    }

    pub fn new_match(
        &mut self,
        config: MatchConfig,
        choose_index: impl FnMut(usize) -> usize,
    ) -> Result<(), DealError> {
        self.session.new_match_with_config(config, choose_index)?;
        self.reset_hand_ui();
        Ok(())
    }

    pub fn reset_bummerls(&mut self) {
        self.session.reset_bummerls();
    }

    fn reset_hand_ui(&mut self) {
        self.clear_selection();
        self.focus = self.view().hand.first().copied();
        self.play_error = None;
        self.exchange_error = None;
        self.hide_card_effects();
    }

    pub fn design(&self) -> CardDesign {
        self.design
    }

    pub fn set_design(&mut self, design: CardDesign) {
        self.design = design;
    }

    pub fn set_show_legal(&mut self, enabled: bool) {
        self.show_legal = enabled;
    }

    pub fn set_show_opponent_eyes(&mut self, enabled: bool) {
        self.show_opponent_eyes = enabled;
    }

    pub fn opponent_eyes(&self) -> Option<u8> {
        (self.show_opponent_eyes && self.session.config().profile.allows_opponent_eyes_help())
            .then(|| self.session.hand().eyes(Player::Opponent))
    }

    pub fn highlighted_cards(&self) -> Vec<Card> {
        if self.show_legal && self.session.config().profile.allows_legal_card_help() {
            self.session.hand().legal_cards(Player::Local)
        } else {
            Vec::new()
        }
    }

    pub fn take_play_error(&mut self) -> Option<PlayError> {
        self.play_error.take()
    }

    pub fn take_exchange_error(&mut self) -> Option<ExchangeError> {
        self.exchange_error.take()
    }

    pub fn exchange_available(&self) -> bool {
        self.selected_card() == Some(self.session.hand().trump_under())
            && self.session.hand().trump_exchange(Player::Local).is_ok()
    }

    pub fn exchange_selected(&mut self) {
        let result = if self.selected_card() == Some(self.session.hand().trump_under()) {
            self.session.exchange_trump(Player::Local)
        } else {
            Err(ExchangeError::MissingTrumpUnder)
        };
        match result {
            Ok(event) => {
                self.clear_selection();
                self.focus = Some(event.received);
                self.visible_exchange = Some(event);
            }
            Err(error) => self.exchange_error = Some(error),
        }
    }

    pub fn exchange(&self) -> Option<Exchange> {
        self.visible_exchange
    }

    pub fn view(&self) -> View<'_> {
        self.session.hand().view(Player::Local)
    }

    pub fn trump_indicator(&self) -> Option<Suit> {
        let view = self.view();
        (view.hidden_talon_count == 0 && view.open_trump.is_none() && !view.talon_closed)
            .then(|| self.session.hand().trump_suit())
    }

    pub fn is_selected(&self, card: Card) -> bool {
        self.selected.contains(&card)
    }

    pub fn focus(&self) -> Option<Card> {
        self.focus
    }

    pub fn move_focus(&mut self, step: isize) {
        let hand = self.view().hand;
        if let Some(index) = hand.iter().position(|card| Some(*card) == self.focus) {
            self.focus = Some(hand[index.saturating_add_signed(step).min(hand.len() - 1)]);
        }
    }

    pub fn toggle(&mut self, card: Card) {
        self.cancel_mouse_click();
        if !self.view().hand.contains(&card) {
            return;
        }
        self.focus = Some(card);
        if let Some(index) = self.selected.iter().position(|&selected| selected == card) {
            self.selected.remove(index);
        } else if self.selected.len() < 2 {
            self.selected.push(card);
        }
    }

    pub fn clear_selection(&mut self) {
        self.selected.clear();
        self.cancel_mouse_click();
    }

    pub fn cancel_mouse_click(&mut self) {
        self.first_click = None;
    }

    pub fn mouse_click(&mut self, card: Card) {
        let prepared_pair = match self.selected.as_slice() {
            &[first, second] => Some([first, second]),
            _ => None,
        };
        self.toggle(card);
        self.first_click = Some((card, prepared_pair));
    }

    pub fn mouse_double_click(&mut self) -> bool {
        let Some((card, prepared_pair)) = self.first_click.take() else {
            return false;
        };

        // The first click can deselect a card and move it down. Reuse its
        // identity and restore a pair that was prepared before the gesture.
        if let Some(pair) = prepared_pair {
            self.selected = pair.into();
        }
        if self.selected.len() == 2 {
            return self.selected_marriage().is_some_and(|marriage| {
                if self.selected.contains(&card) {
                    self.announce_selected(card == marriage.ober() && marriage.may_play_ober)
                } else {
                    false
                }
            });
        }

        let played = self.play_local(card);
        if played {
            self.focus = self.view().hand.first().copied();
        }
        played
    }

    pub fn selected_card(&self) -> Option<Card> {
        match self.selected.as_slice() {
            &[card] => Some(card),
            _ => None,
        }
    }

    pub fn selected_marriage(&self) -> Option<Marriage> {
        let &[first, second] = self.selected.as_slice() else {
            return None;
        };
        let marriage = self
            .session
            .hand()
            .marriage(Player::Local, first.suit)
            .ok()?;
        ((first == marriage.king() && second == marriage.ober())
            || (second == marriage.king() && first == marriage.ober()))
        .then_some(marriage)
    }

    pub fn announce_selected(&mut self, play_ober: bool) -> bool {
        let Some(marriage) = self.selected_marriage() else {
            return false;
        };
        let card = if play_ober {
            marriage.ober()
        } else {
            marriage.king()
        };
        let Ok(event) = self
            .session
            .announce_and_play(Player::Local, marriage.suit, card)
        else {
            return false;
        };
        self.visible_announcement = Some(event);
        self.after_local_play(card);
        true
    }

    pub fn announcement(&self) -> Option<Announcement> {
        self.visible_announcement
    }

    pub fn hide_card_effects(&mut self) {
        self.visible_announcement = None;
        self.visible_exchange = None;
    }

    pub fn check_close_talon(&self) -> Result<(), CloseError> {
        self.session.hand().check_close_talon(Player::Local)
    }

    pub fn close_talon(&mut self) -> Result<(), CloseError> {
        self.session.close_talon(Player::Local)
    }

    pub fn can_play_selected(&self) -> bool {
        self.selected_card().is_some_and(|card| {
            self.session
                .hand()
                .legal_cards(Player::Local)
                .contains(&card)
        })
    }

    pub fn play_selected(&mut self) -> bool {
        self.selected_card()
            .is_some_and(|card| self.play_local(card))
    }

    pub fn swipe(&mut self, card: Card) -> bool {
        (self.selected.is_empty() || self.selected_card() == Some(card)) && self.play_local(card)
    }

    fn play_local(&mut self, card: Card) -> bool {
        self.play_error = self.session.play_card(Player::Local, card).err();
        if self.play_error.is_some() {
            return false;
        }
        self.after_local_play(card);
        true
    }

    fn after_local_play(&mut self, card: Card) {
        self.clear_selection();
        if self.focus == Some(card) {
            self.focus = self.view().hand.first().copied();
        }
    }

    pub fn play_opponent(&mut self) {
        let player = Player::Opponent;
        let difficulty = self.session.config().difficulty;
        // Exchange and closure retain the lead. Re-evaluate from the new
        // observation before playing, with at most two preparatory actions.
        for _ in 0..3 {
            let Some(action) = ai::decide(difficulty, &self.session.observation(player)) else {
                break;
            };
            match ai::apply(&mut self.session, player, action) {
                Ok(
                    ai::ActionEvent::Played
                    | ai::ActionEvent::Claimed
                    | ai::ActionEvent::ClaimContinued,
                ) => return,
                Ok(ai::ActionEvent::Announced(event)) => {
                    self.visible_announcement = Some(event);
                    return;
                }
                Ok(ai::ActionEvent::Exchanged(event)) => self.visible_exchange = Some(event),
                Ok(ai::ActionEvent::Closed) => {}
                Err(error) => {
                    eprintln!("Computer action was rejected: {error:?}");
                    break;
                }
            }
        }
        // A strategy failure must not leave a turn queued indefinitely.
        if let Some(card) = self.session.hand().legal_cards(player).first().copied() {
            self.session
                .play_card(player, card)
                .expect("fallback card is legal");
        }
    }

    pub fn resolve_trick(&mut self) {
        self.hide_card_effects();
        self.session
            .resolve_trick()
            .expect("a complete trick was scheduled");
    }

    pub fn decide_claim(&mut self, claim: bool) {
        let result = if claim {
            self.session.claim(Player::Local)
        } else {
            self.session.continue_claim(Player::Local)
        };
        debug_assert!(result.is_ok());
    }

    pub fn opponent_to_play(&self) -> bool {
        self.session.claim_player() == Some(Player::Opponent)
            || self.session.hand().next_player() == Some(Player::Opponent)
    }

    pub fn local_eyes(&self) -> u8 {
        self.session.hand().eyes(Player::Local)
    }

    pub fn result(&self) -> Option<HandResult> {
        self.session.hand().result()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use schnapsen::core::{
        cards::{Rank, Suit},
        game::{FinishReason, Variant},
        rules::SchneiderRule,
    };

    fn replace_for_test(table: &mut Table, session: Match) {
        table.session = session;
        table.reset_hand_ui();
    }

    #[test]
    fn changing_front_and_back_keeps_selection_geometry_and_the_running_trick() {
        let mut table = Table::new(Match::new(
            Player::Local,
            Variant::Soft,
            SchneiderRule::Double,
            |upper| upper - 1,
        ));
        let lead = table.view().hand[0];
        assert!(table.swipe(lead));
        let selected = table.view().hand[0];
        table.toggle(selected);
        let hand = table.session().hand().clone();
        let config = table.session().config();
        let score = table.session().remaining(Player::Local);
        let focus = table.focus();
        let positions = |table: &Table| {
            crate::ui::layout::arrange(table, 680.0, 560.0)
                .into_iter()
                .map(|placement| placement.rect)
                .collect::<Vec<_>>()
        };
        let before = positions(&table);
        table.set_design(CardDesign::from_ids("french", "legacy"));
        assert_eq!(table.design().front.id, "french");
        assert_eq!(table.design().back.id, "legacy");
        table.set_design(CardDesign::from_ids("french", "alpine"));
        assert_eq!(table.design().front.id, "french");
        assert_eq!(table.design().back.id, "alpine");
        assert_eq!(*table.session().hand(), hand);
        assert_eq!(table.session().config(), config);
        assert_eq!(table.session().remaining(Player::Local), score);
        assert_eq!(table.view().played, [(Player::Local, lead)]);
        assert!(table.is_selected(selected));
        assert_eq!(table.focus(), focus);
        assert_eq!(positions(&table), before);
    }

    #[test]
    fn complete_match_advances_hands_and_the_computer_opens_its_deals() {
        let ordered = |upper| upper - 1;
        let mut table = Table::new(Match::new(
            Player::Local,
            Variant::Soft,
            SchneiderRule::Double,
            ordered,
        ));
        loop {
            let before = table.session.completed_hands();
            while let Some(player) = table.session.hand().next_player() {
                if player == Player::Opponent {
                    table.play_opponent();
                } else {
                    let card = table.session.hand().legal_cards(player)[0];
                    assert!(table.swipe(card));
                }
                if table.view().played.len() == 2 {
                    table.resolve_trick();
                }
            }
            assert!(table.result().is_some());
            assert_eq!(table.session.completed_hands(), before + 1);
            if let Some(result) = table.session.result() {
                assert_eq!(table.session.remaining(result.winner), 0);
                assert!(table.next_hand(ordered).is_err());
                let loser = result.winner.other();
                let total = table.session.bummerls(loser);
                table
                    .new_match(
                        MatchConfig {
                            profile: Variant::Sharp,
                            schneider: SchneiderRule::Single,
                            ..MatchConfig::default()
                        },
                        ordered,
                    )
                    .unwrap();
                assert_eq!(table.session.remaining(Player::Local), 7);
                assert_eq!(table.session.remaining(Player::Opponent), 7);
                assert_eq!(table.session.bummerls(loser), total);
                assert!(table.result().is_none());
                break;
            }
            let before = table.session.forehand();
            table.next_hand(ordered).unwrap();
            assert_eq!(table.session.forehand(), before.other());
            assert!(table.view().played.is_empty());
            assert!(table.announcement().is_none());
            assert_eq!(table.local_eyes(), 0);
            assert_eq!(table.selected_card(), None);
            if table.session.forehand() == Player::Opponent {
                assert!(table.opponent_to_play());
                table.play_opponent();
                assert_eq!(table.view().played[0].0, Player::Opponent);
            }
        }
    }

    fn after_opening_trick(mut session: Match) -> Match {
        let leader = session.forehand();
        for player in [leader, leader.other()] {
            let card = session.hand().legal_cards(player)[0];
            session.play_card(player, card).unwrap();
        }
        assert_eq!(session.resolve_trick(), Ok(leader));
        session
    }

    #[test]
    fn restart_reopens_talon_after_a_finished_closed_hand() {
        let deal = || {
            Match::new(
                Player::Local,
                Variant::Soft,
                SchneiderRule::Double,
                |upper| upper - 1,
            )
        };
        let mut table = Table::new(after_opening_trick(deal()));
        table.close_talon().unwrap();
        while let Some(player) = table.session.hand().next_player() {
            let card = table.session.hand().legal_cards(player)[0];
            table.session.play_card(player, card).unwrap();
            if table.view().played.len() == 2 {
                table.resolve_trick();
            }
        }
        assert!(table.result().is_some());
        assert!(table.view().talon_closed);
        replace_for_test(&mut table, deal());
        assert!(table.result().is_none());
        assert!(!table.view().talon_closed);
        assert!(table.view().open_trump.is_some());
        assert_eq!(
            table.check_close_talon(),
            Err(CloseError::FirstTrickNotCompleted)
        );
    }

    #[test]
    fn exchange_keeps_hand_order_and_focus_and_computer_can_continue() {
        let deal = |leader| {
            Match::new(leader, Variant::Soft, SchneiderRule::Double, |upper| {
                if upper == 10 {
                    7
                } else {
                    upper - 1
                }
            })
        };
        let mut table = Table::new(after_opening_trick(deal(Player::Local)));
        let under = table.session.hand().trump_under();
        let mut hand = table.view().hand.to_vec();
        let index = hand.iter().position(|&card| card == under).unwrap();
        let trump = table.view().open_trump.unwrap();
        hand[index] = trump;
        table.exchange_selected();
        assert_eq!(
            table.take_exchange_error(),
            Some(ExchangeError::MissingTrumpUnder)
        );
        table.toggle(under);
        assert!(table.exchange_available());
        table.exchange_selected();
        assert_eq!(table.take_exchange_error(), None);
        assert_eq!(table.focus(), Some(trump));
        assert_eq!(table.selected_card(), None);
        assert_eq!(table.view().hand, hand);
        assert_eq!(table.view().open_trump, Some(under));
        table.toggle(trump);
        assert!(table.play_selected());
        assert_eq!(table.view().played, &[(Player::Local, trump)]);

        replace_for_test(&mut table, after_opening_trick(deal(Player::Opponent)));
        assert!(table.exchange().is_none());
        table.play_opponent();
        assert_eq!(table.exchange().unwrap().player, Player::Opponent);
        assert_eq!(table.view().open_trump, Some(under));
        assert_eq!(table.view().played.len(), 1);
        assert_eq!(table.view().played[0].0, Player::Opponent);
    }

    #[test]
    fn ai_announces_only_when_allowed_and_the_variant_changes_with_the_deal() {
        let deal = |variant| {
            Match::new(Player::Opponent, variant, SchneiderRule::Double, |upper| {
                upper - 1
            })
        };
        let mut table = Table::new(deal(Variant::Soft));
        table.play_opponent();
        let event = table.announcement().unwrap();
        assert_eq!(event.player, Player::Opponent);
        assert_eq!(event.marriage.suit, Suit::Bells);
        assert_eq!(event.played.rank, Rank::King);
        assert_eq!(table.session.hand().variant(), Variant::Soft);
        replace_for_test(&mut table, deal(Variant::Sharp));
        assert!(table.announcement().is_none());
        table.play_opponent();
        assert!(table.announcement().is_none());
        assert_eq!(table.session.hand().variant(), Variant::Sharp);
        assert_eq!(table.view().played.len(), 1);
    }

    #[test]
    fn double_click_plays_the_original_card_even_after_it_moves() {
        let mut table = Table::new(Match::new(
            Player::Local,
            Variant::Soft,
            SchneiderRule::Double,
            |upper| upper - 1,
        ));
        let card = table.view().hand[0];
        table.mouse_click(card);
        assert!(table.can_play_selected());
        table.mouse_click(card);
        assert_eq!(table.selected_card(), None);
        assert!(table.mouse_double_click());
        assert_eq!(table.view().played, [(Player::Local, card)]);
        assert!(!table.mouse_double_click());
    }

    #[test]
    fn double_click_respects_prepared_pairs_and_invalid_turns() {
        let deal = |leader| {
            Match::new(leader, Variant::Soft, SchneiderRule::Double, |upper| {
                upper - 1
            })
        };
        let mut table = Table::new(deal(Player::Local));
        let first = table.view().hand[0];
        let second = table.view().hand[1];
        table.toggle(first);
        table.toggle(second);
        table.mouse_click(first);
        assert!(!table.mouse_double_click());
        assert_eq!(table.selected, [first, second]);
        assert!(table.view().played.is_empty());

        table.clear_selection();
        let marriage = table
            .session
            .hand()
            .marriage(Player::Local, Suit::Bells)
            .unwrap();
        let king = marriage.king();
        let ober = marriage.ober();
        table.toggle(king);
        table.toggle(ober);
        table.mouse_click(ober);
        assert!(table.mouse_double_click());
        assert_eq!(table.announcement().unwrap().played, ober);
        assert_eq!(table.view().played, [(Player::Local, ober)]);

        replace_for_test(&mut table, deal(Player::Opponent));
        let card = table.view().hand[0];
        table.mouse_click(card);
        assert!(!table.can_play_selected());
        assert!(!table.mouse_double_click());
        assert!(table.take_play_error().is_some());
        assert!(table.view().played.is_empty());
        table.mouse_click(card);
        replace_for_test(&mut table, deal(Player::Local));
        assert!(!table.mouse_double_click());
        assert!(table.view().played.is_empty());
    }

    #[test]
    fn eyes_show_effective_announcements_and_independent_opponent_help() {
        let deal = |leader| {
            Match::new(leader, Variant::Soft, SchneiderRule::Double, |upper| {
                upper - 1
            })
        };
        let mut table = Table::new(deal(Player::Local));
        let marriage = table
            .session
            .hand()
            .marriage(Player::Local, Suit::Bells)
            .unwrap();
        table.toggle(marriage.king());
        table.toggle(marriage.ober());
        assert!(table.announce_selected(false));
        assert_eq!(table.local_eyes(), 0);
        let reply = table.session.hand().legal_cards(Player::Opponent)[0];
        table.session.play_card(Player::Opponent, reply).unwrap();
        assert_eq!(table.session.resolve_trick(), Ok(Player::Local));
        assert_eq!(
            table.local_eyes(),
            marriage.eyes + marriage.king().points() + reply.points()
        );

        replace_for_test(&mut table, after_opening_trick(deal(Player::Opponent)));
        let eyes = table.session.hand().eyes(Player::Opponent);
        assert!(eyes > 0);
        table.set_show_legal(true);
        assert_eq!(table.opponent_eyes(), None);
        table.set_show_legal(false);
        table.set_show_opponent_eyes(true);
        assert_eq!(table.opponent_eyes(), Some(eyes));
        assert!(table.highlighted_cards().is_empty());
        replace_for_test(&mut table, deal(Player::Local));
        assert_eq!(table.opponent_eyes(), Some(0));
    }

    #[test]
    fn selection_is_limited_and_leaves_game_unchanged() {
        let mut table = Table::new(Match::new(
            Player::Local,
            Variant::Soft,
            SchneiderRule::Double,
            |upper| upper - 1,
        ));
        let hand = table.view().hand.to_vec();
        table.toggle(hand[0]);
        assert!(table.is_selected(hand[0]));
        table.toggle(hand[0]);
        assert!(!table.is_selected(hand[0]));
        for &card in &hand[..3] {
            table.toggle(card);
        }
        assert_eq!(table.selected, hand[..2]);
        assert_eq!(table.view().hand, hand);
        assert!(table.view().played.is_empty());
        table.clear_selection();
        assert!(table.selected.is_empty());
    }

    #[test]
    fn new_game_resets_selection_and_focus_even_for_identical_deals() {
        let deal = || {
            Match::new(
                Player::Local,
                Variant::Soft,
                SchneiderRule::Double,
                |upper| upper - 1,
            )
        };
        let mut table = Table::new(deal());
        table.move_focus(-1);
        assert_eq!(table.focus(), Some(table.view().hand[0]));
        table.move_focus(1);
        table.toggle(table.focus().unwrap());
        assert_eq!(table.selected, [table.view().hand[1]]);
        replace_for_test(&mut table, deal());
        assert!(table.selected.is_empty());
        assert_eq!(table.focus(), Some(table.view().hand[0]));
    }

    #[test]
    fn inputs_only_play_a_single_card_on_the_local_turn() {
        let mut table = Table::new(Match::new(
            Player::Local,
            Variant::Soft,
            SchneiderRule::Double,
            |upper| upper - 1,
        ));
        let first = table.view().hand[0];
        let second = table.view().hand[1];
        table.toggle(first);
        table.toggle(second);
        assert!(!table.play_selected());
        assert!(!table.swipe(first));
        table.toggle(second);
        assert!(table.play_selected());
        assert_eq!(table.view().played, [(Player::Local, first)]);
        assert!(!table.view().hand.contains(&first));
        assert_eq!(table.selected_card(), None);
        assert!(!table.swipe(second));
        table.play_opponent();
        assert_eq!(table.view().played.len(), 2);
        assert!(!table.swipe(second));
        table.resolve_trick();
        assert!(table.view().played.is_empty());
        assert_eq!(table.view().hand.len(), 5);
        assert_eq!(table.view().hidden_talon_count, 7);
    }

    #[test]
    fn play_help_is_optional_and_survives_a_new_deal() {
        let deal = || {
            Match::new(
                Player::Local,
                Variant::Soft,
                SchneiderRule::Double,
                |upper| upper - 1,
            )
        };
        let mut table = Table::new(deal());
        assert!(table.highlighted_cards().is_empty());
        table.set_show_legal(true);
        assert_eq!(table.highlighted_cards(), table.view().hand);
        replace_for_test(&mut table, deal());
        assert_eq!(table.highlighted_cards(), table.view().hand);
        table.set_show_legal(false);
        assert!(table.highlighted_cards().is_empty());
        table.toggle(table.view().hand[0]);
        assert!(table.play_selected());
    }

    #[test]
    fn both_players_win_automatically_and_hand_reset_clears_the_result() {
        let mut winners = [false; 2];
        for seed in 0..32 {
            let deal = || {
                Match::new(
                    Player::Local,
                    Variant::Soft,
                    SchneiderRule::Double,
                    |upper| (seed + upper / 2) % upper,
                )
            };
            let mut table = Table::new(deal());
            while let Some(player) = table.session.hand().next_player() {
                if player == Player::Opponent {
                    table.play_opponent();
                } else {
                    let card = table.session.hand().legal_cards(player)[0];
                    assert!(table.swipe(card));
                }
                if table.view().played.len() == 2 {
                    table.resolve_trick();
                }
            }
            let result = table.result().unwrap();
            if result.outcome.reason == FinishReason::Reached66 {
                assert!(result.score(result.outcome.winner).eyes >= 66);
                winners[result.outcome.winner as usize] = true;
            }
            replace_for_test(&mut table, deal());
            assert!(table.result().is_none());
            assert!(table.view().played.is_empty());
            assert_eq!(table.view().hand.len(), 5);
            assert_eq!(table.view().hidden_talon_count, 9);
            assert_eq!(table.local_eyes(), 0);
        }
        assert_eq!(winners, [true, true]);
    }
}
