use crate::core::cards::Card;
use crate::core::game::{Exchange, Marriage};

pub fn wants_exchange(legal: Exchange) -> bool {
    legal.received.points() > legal.given.points()
}

pub fn choose_marriage(legal_marriages: &[Marriage]) -> Option<Marriage> {
    legal_marriages.first().copied()
}

pub fn choose_card(legal_cards: &[Card]) -> Option<Card> {
    legal_cards.first().copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::game::{Game, Player};

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
}
