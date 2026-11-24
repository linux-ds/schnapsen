use schnapsen::{
    ai,
    core::{
        cards::Card,
        game::{
            Announcement, CloseError, DeclareError, Exchange, ExchangeError, HandResult, Marriage,
            PlayError, Player, Variant, View,
        },
        match_play::{DealError, Match, SchneiderRule},
    },
};

pub(super) struct Table {
    session: Match,
    selected: Vec<Card>,
    focus: Option<Card>,
    show_legal: bool,
    play_error: Option<PlayError>,
    visible_announcement: Option<Announcement>,
    visible_exchange: Option<Exchange>,
    exchange_error: Option<ExchangeError>,
    continue_after_announcement: bool,
}

impl Table {
    pub fn new(session: Match) -> Self {
        let focus = session.hand().view(Player::Local).hand.first().copied();
        Self {
            session,
            selected: Vec::new(),
            focus,
            show_legal: false,
            play_error: None,
            visible_announcement: None,
            visible_exchange: None,
            exchange_error: None,
            continue_after_announcement: false,
        }
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
        variant: Variant,
        schneider: SchneiderRule,
        choose_index: impl FnMut(usize) -> usize,
    ) -> Result<(), DealError> {
        self.session.new_match(variant, schneider, choose_index)?;
        self.reset_hand_ui();
        Ok(())
    }

    pub fn reset_bummerls(&mut self) {
        self.session.reset_bummerls();
    }

    fn reset_hand_ui(&mut self) {
        self.selected.clear();
        self.focus = self.view().hand.first().copied();
        self.play_error = None;
        self.exchange_error = None;
        self.hide_card_effects();
        self.continue_after_announcement = false;
    }

    pub fn set_show_legal(&mut self, enabled: bool) {
        self.show_legal = enabled;
    }

    pub fn highlighted_cards(&self) -> Vec<Card> {
        if self.show_legal {
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
        self.continue_after_announcement = false;
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

    pub fn waiting_for_declaration(&self) -> bool {
        !self.continue_after_announcement && self.session.hand().announcement_lead(Player::Local)
    }

    pub fn continue_play(&mut self) {
        if !self.session.decline_final_declaration(Player::Local) {
            self.continue_after_announcement = true;
        }
    }

    pub fn awaiting_final_declaration(&self) -> bool {
        self.session.hand().awaiting_final_declaration()
    }

    pub fn check_close_talon(&self) -> Result<(), CloseError> {
        self.session.hand().check_close_talon(Player::Local)
    }

    pub fn close_talon(&mut self) -> Result<(), CloseError> {
        self.session.close_talon(Player::Local)
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
        if let Ok(exchange) = self.session.hand().trump_exchange(Player::Opponent) {
            if ai::wants_exchange(exchange) {
                self.visible_exchange = Some(
                    self.session
                        .exchange_trump(Player::Opponent)
                        .expect("AI chose a legal exchange"),
                );
            }
        }
        if let Some(marriage) =
            ai::choose_marriage(&self.session.hand().legal_marriages(Player::Opponent))
        {
            let event = self
                .session
                .announce_and_play(Player::Opponent, marriage.suit, marriage.king())
                .expect("AI chose a legal marriage");
            self.visible_announcement = Some(event);
            if self
                .session
                .hand()
                .can_declare_after_announcement(Player::Opponent)
            {
                self.session
                    .declare(Player::Opponent)
                    .expect("the computer may declare");
            }
        } else if let Some(card) =
            ai::choose_card(&self.session.hand().legal_cards(Player::Opponent))
        {
            self.session
                .play_card(Player::Opponent, card)
                .expect("AI chose a legal card");
        }
    }

    pub fn resolve_trick(&mut self) {
        self.hide_card_effects();
        let winner = self
            .session
            .resolve_trick()
            .expect("a complete trick was scheduled");
        if winner == Player::Opponent && self.session.hand().can_declare(winner) {
            self.session
                .declare(winner)
                .expect("the computer may declare");
        }
    }

    pub fn opponent_to_play(&self) -> bool {
        self.session.hand().next_player() == Some(Player::Opponent)
    }

    pub fn local_eyes(&self) -> u8 {
        self.session.hand().eyes(Player::Local)
    }

    pub fn may_declare(&self) -> bool {
        let hand = self.session.hand();
        hand.may_declare(Player::Local)
            && !(self.continue_after_announcement && hand.announcement_lead(Player::Local))
    }

    #[cfg(test)]
    pub fn can_declare(&self) -> bool {
        self.session.hand().can_declare(Player::Local)
    }

    pub fn declare(&mut self) -> Result<(), DeclareError> {
        self.session.declare(Player::Local)
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
    };

    fn replace_for_test(table: &mut Table, session: Match) {
        table.session = session;
        table.reset_hand_ui();
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
                } else if table.can_declare() {
                    table.declare().unwrap();
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
                    .new_match(Variant::Sharp, SchneiderRule::Single, ordered)
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
        if table.awaiting_final_declaration() {
            table.continue_play();
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
    fn computer_declares_after_a_trick_and_player_can_declare_before_leading() {
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
                } else if table.can_declare() {
                    assert!(table.view().played.is_empty());
                    assert!(table.local_eyes() >= 66);
                    table.declare().unwrap();
                } else {
                    let card = table.session.hand().legal_cards(player)[0];
                    assert!(table.swipe(card));
                }
                if table.view().played.len() == 2 {
                    table.resolve_trick();
                }
            }
            let result = table.result().unwrap();
            if result.outcome.reason == FinishReason::Declared66 {
                assert!(result.score(result.outcome.winner).eyes >= 66);
                winners[result.outcome.winner as usize] = true;
            }
            replace_for_test(&mut table, deal());
            assert!(table.result().is_none());
            assert!(table.view().played.is_empty());
            assert_eq!(table.view().hand.len(), 5);
            assert_eq!(table.view().hidden_talon_count, 9);
            assert_eq!(table.local_eyes(), 0);
            assert!(!table.can_declare());
        }
        assert_eq!(winners, [true, true]);
    }
}
