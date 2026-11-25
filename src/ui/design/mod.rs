//! Renderer descriptors keyed by stable presentation IDs.

use adw::gtk::cairo::{Context, Error};
use schnapsen::core::cards::{Card, Rank, Suit};
use std::f64::consts::TAU;

mod backs;
mod fronts;

type Color = (f64, f64, f64);

pub(crate) struct FrontTheme {
    pub id: &'static str,
    pub label: &'static str,
    rank: fn(Rank) -> (&'static str, &'static str),
    suit: fn(Suit) -> (&'static str, Color),
    symbol: fn(&Context, Suit),
    render: fn(&Context, Card, &Self) -> Result<(), Error>,
}

impl FrontTheme {
    pub fn suit_name(&self, suit: Suit) -> &'static str {
        self.suit(suit).0
    }

    pub fn rank_name(&self, rank: Rank) -> &'static str {
        self.rank(rank).1
    }

    pub fn card_name(&self, card: Card) -> String {
        format!(
            "{} {}",
            self.suit_name(card.suit),
            self.rank_name(card.rank)
        )
    }

    pub(super) fn suit(&self, suit: Suit) -> (&'static str, Color) {
        (self.suit)(suit)
    }

    fn rank(&self, rank: Rank) -> (&'static str, &'static str) {
        (self.rank)(rank)
    }

    pub(super) fn draw(&self, cr: &Context, card: Card) -> Result<(), Error> {
        (self.render)(cr, card, self)
    }

    pub(super) fn draw_symbol(
        &self,
        cr: &Context,
        suit: Suit,
        x: f64,
        y: f64,
        size: f64,
    ) -> Result<(), Error> {
        cr.save()?;
        cr.translate(x - size / 2.0, y - size / 2.0);
        cr.scale(size, size);
        let (r, g, b) = self.suit(suit).1;
        cr.set_source_rgb(r, g, b);
        cr.new_path();
        (self.symbol)(cr, suit);
        cr.fill()?;
        cr.restore()
    }
}

pub(crate) struct CardBack {
    pub id: &'static str,
    pub label: &'static str,
    render: fn(&Context) -> Result<(), Error>,
}

impl CardBack {
    pub(super) fn draw(&self, cr: &Context) -> Result<(), Error> {
        cr.save()?;
        (self.render)(cr)?;
        cr.restore()
    }
}

pub(crate) const FRONTS: &[FrontTheme] = &[fronts::DOUBLE_GERMAN, fronts::FRENCH];
pub(crate) const BACKS: &[CardBack] = &[
    backs::LEGACY,
    backs::CLASSIC,
    backs::VIENNESE,
    backs::ALPINE,
    backs::GEOMETRIC,
    backs::ORNAMENT,
    backs::MINIMAL,
];

#[derive(Clone, Copy)]
pub(crate) struct CardDesign {
    pub front: &'static FrontTheme,
    pub back: &'static CardBack,
}

impl Default for CardDesign {
    fn default() -> Self {
        Self {
            front: &FRONTS[0],
            back: &BACKS[0],
        }
    }
}

impl CardDesign {
    pub fn from_ids(front: &str, back: &str) -> Self {
        let default = Self::default();
        Self {
            front: FRONTS
                .iter()
                .find(|theme| theme.id == front)
                .unwrap_or(default.front),
            back: BACKS
                .iter()
                .find(|design| design.id == back)
                .unwrap_or(default.back),
        }
    }
}

pub(super) fn rounded_rect(cr: &Context, x: f64, y: f64, width: f64, height: f64, radius: f64) {
    let quarter = TAU / 4.0;
    cr.new_sub_path();
    cr.arc(x + width - radius, y + radius, radius, -quarter, 0.0);
    cr.arc(
        x + width - radius,
        y + height - radius,
        radius,
        0.0,
        quarter,
    );
    cr.arc(
        x + radius,
        y + height - radius,
        radius,
        quarter,
        2.0 * quarter,
    );
    cr.arc(x + radius, y + radius, radius, 2.0 * quarter, 3.0 * quarter);
    cr.close_path();
}

#[cfg(test)]
mod tests {
    use super::*;
    use schnapsen::core::cards::full_deck;
    use std::collections::HashSet;

    #[test]
    fn themes_map_all_logical_cards_and_keep_stable_ids() {
        let [german, french] = FRONTS else {
            panic!("two built-in themes")
        };
        let suits = [Suit::Hearts, Suit::Bells, Suit::Leaves, Suit::Acorns];
        let ranks = [Rank::Ace, Rank::Ten, Rank::King, Rank::Ober, Rank::Unter];
        assert_eq!(
            suits.map(|suit| german.suit_name(suit)),
            ["Herz", "Schellen", "Grün", "Eichel"]
        );
        assert_eq!(
            suits.map(|suit| french.suit_name(suit)),
            ["Herz", "Karo", "Pik", "Kreuz"]
        );
        assert_eq!(
            ranks.map(|rank| german.rank(rank)),
            [
                ("A", "Ass"),
                ("10", "Zehn"),
                ("K", "König"),
                ("O", "Ober"),
                ("U", "Unter")
            ]
        );
        assert_eq!(
            ranks.map(|rank| french.rank(rank)),
            [
                ("A", "Ass"),
                ("10", "Zehn"),
                ("K", "König"),
                ("D", "Dame"),
                ("B", "Bube")
            ]
        );
        for theme in FRONTS {
            assert_eq!(
                full_deck()
                    .into_iter()
                    .map(|card| theme.card_name(card))
                    .collect::<HashSet<_>>()
                    .len(),
                20
            );
        }
        assert_eq!(
            FRONTS
                .iter()
                .map(|item| item.id)
                .collect::<HashSet<_>>()
                .len(),
            FRONTS.len()
        );
        assert_eq!(
            BACKS
                .iter()
                .map(|item| item.id)
                .collect::<HashSet<_>>()
                .len(),
            BACKS.len()
        );
    }

    #[test]
    fn unknown_design_ids_fall_back_independently() {
        let front_fallback = CardDesign::from_ids("unknown", "alpine");
        assert_eq!(front_fallback.front.id, "double-german");
        assert_eq!(front_fallback.back.id, "alpine");
        let back_fallback = CardDesign::from_ids("french", "unknown");
        assert_eq!(back_fallback.front.id, "french");
        assert_eq!(back_fallback.back.id, "legacy");
    }
}
