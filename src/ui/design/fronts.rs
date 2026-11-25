use super::{Color, FrontTheme};
use adw::gtk::cairo::{Context, Error, FontSlant, FontWeight};
use schnapsen::core::cards::{Card, Rank, Suit};
use std::f64::consts::TAU;

fn draw_front(cr: &Context, card: Card, theme: &FrontTheme) -> Result<(), Error> {
    let value = crate::i18n::gettext(theme.rank(card.rank).0);
    let (r, g, b) = theme.suit(card.suit).1;
    cr.set_source_rgb(r, g, b);
    cr.select_font_face("Sans", FontSlant::Normal, FontWeight::Bold);
    cr.set_font_size(0.255);
    cr.move_to(0.105, 0.315);
    cr.show_text(&value)?;
    theme.draw_symbol(cr, card.suit, 0.19, 0.47, 0.19)?;
    theme.draw_symbol(cr, card.suit, 0.50, 0.81, 0.53)
}

fn rank_german(rank: Rank) -> (&'static str, &'static str) {
    match rank {
        Rank::Ace => (msgid!("A"), msgid!("Ace")),
        Rank::Ten => ("10", msgid!("Ten")),
        Rank::King => (msgid!("K"), msgid!("King")),
        Rank::Ober => (msgid!("O"), msgid!("Ober")),
        Rank::Unter => (msgid!("U"), msgid!("Unter")),
    }
}

fn suit_german(suit: Suit) -> (&'static str, Color) {
    match suit {
        Suit::Hearts => (msgid!("Hearts"), (0.67, 0.15, 0.21)),
        Suit::Bells => (msgid!("Bells"), (0.56, 0.32, 0.04)),
        Suit::Leaves => (msgid!("Leaves"), (0.12, 0.37, 0.24)),
        Suit::Acorns => (msgid!("Acorns"), (0.29, 0.19, 0.14)),
    }
}

fn german_symbol(cr: &Context, suit: Suit) {
    match suit {
        Suit::Hearts => {
            cr.move_to(0.50, 0.93);
            cr.curve_to(-0.48, 0.17, 0.27, -0.19, 0.50, 0.30);
            cr.curve_to(0.73, -0.19, 1.48, 0.17, 0.50, 0.93);
        }
        Suit::Bells => {
            cr.move_to(0.12, 0.72);
            cr.curve_to(0.34, 0.65, 0.17, 0.10, 0.50, 0.08);
            cr.curve_to(0.83, 0.10, 0.66, 0.65, 0.88, 0.72);
            cr.line_to(0.88, 0.81);
            cr.line_to(0.12, 0.81);
            cr.close_path();
            cr.new_sub_path();
            cr.arc(0.50, 0.88, 0.088, 0.0, TAU);
        }
        Suit::Leaves => {
            cr.move_to(0.14, 0.92);
            cr.line_to(0.07, 0.85);
            cr.line_to(0.24, 0.69);
            cr.curve_to(0.04, 0.34, 0.32, 0.13, 0.94, 0.06);
            cr.curve_to(0.86, 0.68, 0.65, 0.95, 0.31, 0.76);
        }
        Suit::Acorns => {
            cr.move_to(0.16, 0.47);
            cr.curve_to(0.13, 0.72, 0.30, 0.88, 0.50, 0.96);
            cr.curve_to(0.70, 0.88, 0.87, 0.72, 0.84, 0.47);
            cr.close_path();
            cr.move_to(0.09, 0.40);
            cr.curve_to(0.09, 0.22, 0.29, 0.14, 0.47, 0.15);
            cr.line_to(0.54, 0.04);
            cr.line_to(0.66, 0.07);
            cr.line_to(0.60, 0.16);
            cr.curve_to(0.78, 0.18, 0.91, 0.26, 0.91, 0.40);
        }
    }
}

fn rank_french(rank: Rank) -> (&'static str, &'static str) {
    match rank {
        // Translators: corner index for a queen in the French deck.
        Rank::Ober => (msgid!("Q"), msgid!("Queen")),
        // Translators: corner index for a jack in the French deck.
        Rank::Unter => (msgid!("J"), msgid!("Jack")),
        _ => rank_german(rank),
    }
}

fn suit_french(suit: Suit) -> (&'static str, Color) {
    let red = (0.70, 0.11, 0.18);
    let black = (0.08, 0.09, 0.11);
    match suit {
        Suit::Hearts => (msgid!("Hearts"), red),
        Suit::Bells => (msgid!("Diamonds"), red),
        Suit::Leaves => (msgid!("Spades"), black),
        Suit::Acorns => (msgid!("Clubs"), black),
    }
}

fn stem(cr: &Context) {
    cr.move_to(0.42, 0.61);
    cr.curve_to(0.44, 0.77, 0.38, 0.86, 0.30, 0.92);
    cr.line_to(0.70, 0.92);
    cr.curve_to(0.62, 0.86, 0.56, 0.77, 0.58, 0.61);
    cr.close_path();
}

fn french_symbol(cr: &Context, suit: Suit) {
    match suit {
        Suit::Hearts => german_symbol(cr, suit),
        Suit::Bells => {
            cr.move_to(0.50, 0.04);
            cr.line_to(0.88, 0.50);
            cr.line_to(0.50, 0.96);
            cr.line_to(0.12, 0.50);
            cr.close_path();
        }
        Suit::Leaves => {
            cr.move_to(0.50, 0.04);
            cr.curve_to(-0.40, 0.71, 0.20, 0.92, 0.50, 0.61);
            cr.curve_to(0.80, 0.92, 1.40, 0.71, 0.50, 0.04);
            cr.close_path();
            cr.new_sub_path();
            stem(cr);
        }
        Suit::Acorns => {
            for (x, y) in [(0.50, 0.29), (0.27, 0.55), (0.73, 0.55)] {
                cr.new_sub_path();
                cr.arc(x, y, 0.235, 0.0, TAU);
            }
            cr.new_sub_path();
            stem(cr);
        }
    }
}

pub(super) const DOUBLE_GERMAN: FrontTheme = FrontTheme {
    id: "double-german",
    label: msgid!("Double German"),
    rank: rank_german,
    suit: suit_german,
    symbol: german_symbol,
    render: draw_front,
};

pub(super) const FRENCH: FrontTheme = FrontTheme {
    id: "french",
    label: msgid!("French"),
    rank: rank_french,
    suit: suit_french,
    symbol: french_symbol,
    render: draw_front,
};
