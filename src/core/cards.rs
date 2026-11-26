use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Suit {
    Hearts,
    Bells,
    Leaves,
    Acorns,
}

impl Suit {
    const ALL: [Self; 4] = [Self::Hearts, Self::Bells, Self::Leaves, Self::Acorns];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Rank {
    Ace,
    Ten,
    King,
    Ober,
    Unter,
}

impl Rank {
    const ALL: [Self; 5] = [Self::Ace, Self::Ten, Self::King, Self::Ober, Self::Unter];

    pub const fn points(self) -> u8 {
        match self {
            Self::Ace => 11,
            Self::Ten => 10,
            Self::King => 4,
            Self::Ober => 3,
            Self::Unter => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Card {
    pub suit: Suit,
    pub rank: Rank,
}

impl Card {
    pub const fn points(self) -> u8 {
        self.rank.points()
    }
}

pub fn full_deck() -> [Card; 20] {
    std::array::from_fn(|index| Card {
        suit: Suit::ALL[index / Rank::ALL.len()],
        rank: Rank::ALL[index % Rank::ALL.len()],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn deck_contains_each_suit_and_rank_combination_once() {
        let cards = HashSet::from(full_deck());
        assert_eq!(cards.len(), 20);
        for suit in [Suit::Hearts, Suit::Bells, Suit::Leaves, Suit::Acorns] {
            for rank in [Rank::Ace, Rank::Ten, Rank::King, Rank::Ober, Rank::Unter] {
                assert!(cards.contains(&Card { suit, rank }));
            }
        }
    }

    #[test]
    fn cards_have_the_correct_points() {
        for (rank, expected) in [
            (Rank::Ace, 11),
            (Rank::Ten, 10),
            (Rank::King, 4),
            (Rank::Ober, 3),
            (Rank::Unter, 2),
        ] {
            let card = Card {
                suit: Suit::Hearts,
                rank,
            };
            assert_eq!(card.points(), expected);
        }
    }
}
