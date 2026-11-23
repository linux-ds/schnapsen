use schnapsen::core::cards::Card;
use schnapsen::core::game::Player;

use super::table::Table;

pub(super) const MIN_WIDTH: i32 = 560;
pub(super) const MIN_HEIGHT: i32 = 400;
pub(super) const ASPECT: f64 = 1.4;
pub(super) const CORNER_RADIUS: f64 = 0.085;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
}

impl Rect {
    pub fn new(x: f64, y: f64, width: f64) -> Self {
        Self { x, y, width }
    }

    pub fn height(self) -> f64 {
        self.width * ASPECT
    }

    fn contains(self, x: f64, y: f64) -> bool {
        let x = (x - self.x) / self.width;
        let y = (y - self.y) / self.width;
        let radius = CORNER_RADIUS;
        let corner_x = x.clamp(radius, 1.0 - radius);
        let corner_y = y.clamp(radius, ASPECT - radius);
        (x - corner_x).hypot(y - corner_y) <= radius
    }
}

#[derive(Clone, Copy)]
pub(super) enum Face {
    Front(Card),
    Back,
}

pub(super) struct Placement {
    pub face: Face,
    pub rect: Rect,
    pub in_hand: bool,
    pub open_trump: bool,
    pub talon_stack: bool,
    pub trick_owner: Option<Player>,
    pub sideways: bool,
    pub selected: bool,
    pub highlighted: bool,
}

fn placement_at(placements: &[Placement], x: f64, y: f64) -> Option<&Placement> {
    placements.iter().rev().find(|entry| {
        let rect = entry.rect;
        if entry.sideways {
            rect.contains(rect.x + y - rect.y, rect.y + rect.height() - (x - rect.x))
        } else {
            rect.contains(x, y)
        }
    })
}

pub(super) fn talon_at(placements: &[Placement], x: f64, y: f64) -> Option<f64> {
    placement_at(placements, x, y)
        .filter(|entry| entry.talon_stack)
        .map(|entry| entry.rect.height())
}

pub(super) fn trick_at(placements: &[Placement], x: f64, y: f64) -> Option<(Player, f64)> {
    let entry = placement_at(placements, x, y)?;
    Some((entry.trick_owner?, entry.rect.height()))
}

pub(super) fn open_trump_at(placements: &[Placement], x: f64, y: f64) -> Option<f64> {
    placement_at(placements, x, y)
        .filter(|entry| entry.open_trump)
        .map(|entry| entry.rect.height())
}

pub(super) fn hand_at(placements: &[Placement], x: f64, y: f64) -> Option<(Card, f64)> {
    let entry = placement_at(placements, x, y).filter(|entry| entry.in_hand)?;
    match entry.face {
        Face::Front(card) => Some((card, entry.rect.height())),
        Face::Back => None,
    }
}

fn stack_positions(base: Rect, count: usize) -> impl Iterator<Item = Rect> {
    // Start at the bottom card and limit the entire stack's footprint.
    let steps = count.saturating_sub(1).max(1) as f64;
    let step = 0.048_f64.min(0.19 / steps) * base.width;
    (0..count).map(move |index| {
        let offset = index as f64 * step;
        Rect::new(base.x + offset, base.y - offset, base.width)
    })
}

fn row_positions(
    center: f64,
    y: f64,
    width: f64,
    step: f64,
    count: usize,
) -> impl Iterator<Item = Rect> {
    let span = width + step * count.saturating_sub(1) as f64;
    let left = center - span / 2.0;
    (0..count).map(move |index| Rect::new(left + step * index as f64, y, width))
}

pub(super) fn arrange(table: &Table, width: f64, height: f64) -> Vec<Placement> {
    if width <= 0.0 || height <= 0.0 {
        return Vec::new();
    }

    let size = (width / 6.45).min(height / 4.55).min(138.0);
    let center = width / 2.0;
    let top = size * 0.28;
    let own_y = height - size * (ASPECT + 0.25);
    let opponent_size = size * 0.55;
    let middle = (top + opponent_size * ASPECT + own_y) / 2.0;
    let played_size = size * 0.95;
    let hand_step = (width / size / 4.0 - 0.78).clamp(0.82, 1.06) * size;
    let hand_right = center + size / 2.0 + 2.0 * hand_step;
    let stack_size = size * 0.60;
    let stack_x = hand_right + size * 0.20;
    let talon_size = size * 0.70;
    let talon_y = middle - talon_size * ASPECT / 2.0;
    let trump_x = center - 2.73 * size;
    let view = table.view();
    let legal = table.highlighted_cards();
    let announcement = table.announcement();
    let exchange = table.exchange();
    let exchange_available = table.exchange_available();
    let mut placements = Vec::new();
    let mut add = |face, mut rect: Rect, in_hand, talon_stack, trick_owner| {
        let selected = in_hand && matches!(face, Face::Front(card) if table.is_selected(card));
        let highlighted = in_hand && matches!(face, Face::Front(card) if legal.contains(&card));
        let open_trump = matches!(face, Face::Front(card) if Some(card) == view.open_trump);
        if selected {
            rect.y -= rect.height() * 0.12;
        }
        placements.push(Placement {
            face,
            rect,
            in_hand,
            open_trump,
            talon_stack,
            trick_owner,
            sideways: false,
            selected: selected
                || matches!(face, Face::Front(card) if
                announcement.is_some_and(|event| card == event.played || card == event.partner())
                || exchange.is_some_and(|event| card == event.given || (in_hand && card == event.received))),
            highlighted: highlighted || (open_trump && exchange_available),
        });
    };

    for (index, rect) in row_positions(
        center,
        top,
        opponent_size,
        opponent_size * 0.86,
        view.opponent_hand_count,
    )
    .enumerate()
    {
        let revealed = announcement
            .filter(|event| event.player == schnapsen::core::game::Player::Opponent && index == 0);
        add(
            revealed.map_or(Face::Back, |event| Face::Front(event.partner())),
            rect,
            false,
            false,
            None,
        );
    }
    for rect in stack_positions(
        Rect::new(stack_x, top + stack_size * 0.19, stack_size),
        view.opponent_trick_card_count,
    ) {
        add(Face::Back, rect, false, false, Some(Player::Opponent));
    }
    if let Some(trump) = view.open_trump {
        add(
            Face::Front(trump),
            Rect::new(trump_x, talon_y, talon_size),
            false,
            false,
            None,
        );
    }
    let stock = Rect::new(trump_x + talon_size * 0.53, talon_y, talon_size);
    for rect in stack_positions(stock, view.hidden_talon_count) {
        add(Face::Back, rect, false, !view.talon_closed, None);
    }
    for (&(_, card), rect) in view.played.iter().zip(row_positions(
        center,
        middle - played_size * ASPECT / 2.0,
        played_size,
        played_size * 1.18,
        view.played.len(),
    )) {
        add(Face::Front(card), rect, false, false, None);
    }
    for (&card, rect) in view.hand.iter().zip(row_positions(
        center,
        own_y,
        size,
        hand_step,
        view.hand.len(),
    )) {
        add(Face::Front(card), rect, true, false, None);
    }
    for rect in stack_positions(
        Rect::new(
            stack_x,
            height - size * 0.25 - stack_size * ASPECT,
            stack_size,
        ),
        view.own_trick_card_count,
    ) {
        add(Face::Back, rect, false, false, Some(Player::Local));
    }
    if view.talon_closed {
        placements.push(Placement {
            face: Face::Back,
            rect: Rect::new(
                stock.x - talon_size * 0.15,
                middle - talon_size * 0.56,
                talon_size,
            ),
            in_hand: false,
            open_trump: false,
            talon_stack: false,
            trick_owner: None,
            sideways: true,
            selected: false,
            highlighted: false,
        });
    }
    placements
}

#[cfg(test)]
mod tests {
    use super::*;
    use schnapsen::core::{
        game::{Player, Variant},
        match_play::{Match, SchneiderRule},
    };

    fn table() -> Table {
        Table::new(Match::new(
            Player::Local,
            Variant::Soft,
            SchneiderRule::Double,
            |upper| upper - 1,
        ))
    }

    #[test]
    fn closed_talon_has_one_sideways_back_and_keeps_all_cards_at_minimum_size() {
        let mut table = table();
        table.close_talon().unwrap();
        let placements = arrange(&table, MIN_WIDTH.into(), MIN_HEIGHT.into());
        assert_eq!(placements.len(), 20);
        assert!(placements
            .iter()
            .all(|card| !card.open_trump && !card.talon_stack));
        let covers: Vec<_> = placements.iter().filter(|card| card.sideways).collect();
        assert_eq!(covers.len(), 1);
        let cover = covers[0];
        assert!(matches!(cover.face, Face::Back));
        assert!(cover.rect.x >= 0.0 && cover.rect.y >= 0.0);
        assert!(cover.rect.x + cover.rect.height() <= f64::from(MIN_WIDTH));
        assert!(cover.rect.y + cover.rect.width <= f64::from(MIN_HEIGHT));
    }

    #[test]
    fn initial_layout_matches_view_and_only_exposes_public_cards() {
        let table = table();
        let view = table.view();
        let cards = arrange(&table, 920.0, 600.0);
        let trump = cards.iter().find(|card| card.open_trump).unwrap().rect;
        let y = trump.y + trump.height() * 0.5;
        assert!(open_trump_at(&cards, trump.x + trump.width * 0.2, y).is_some());
        assert!(open_trump_at(&cards, trump.x + trump.width * 0.8, y).is_none());
        assert_eq!(cards.len(), 20);
        let visible: Vec<_> = cards
            .iter()
            .filter_map(|entry| match entry.face {
                Face::Front(card) => Some(card),
                Face::Back => None,
            })
            .collect();
        assert_eq!(
            visible,
            view.open_trump
                .into_iter()
                .chain(view.hand.iter().copied())
                .collect::<Vec<_>>()
        );
        assert_eq!(cards.iter().filter(|entry| entry.in_hand).count(), 5);
        assert_eq!(
            cards[..5]
                .iter()
                .filter(|entry| matches!(entry.face, Face::Back))
                .count(),
            5
        );
        assert_eq!(
            cards[5..15]
                .iter()
                .filter(|entry| matches!(entry.face, Face::Back))
                .count(),
            9
        );
    }

    #[test]
    fn stack_keeps_each_card_and_caps_its_span() {
        let base = Rect::new(10.0, 20.0, 50.0);
        assert_eq!(stack_positions(base, 0).count(), 0);
        assert_eq!(stack_positions(base, 1).collect::<Vec<_>>(), [base]);
        for count in [2, 3, 20] {
            let positions = stack_positions(base, count).collect::<Vec<_>>();
            assert_eq!(positions.len(), count);
            assert_eq!(positions[0], base);
            assert!(positions
                .windows(2)
                .all(|pair| { pair[0].x < pair[1].x && pair[0].y > pair[1].y }));
            let top = positions[count - 1];
            assert!(top.x - base.x <= 0.19 * base.width + 1e-9);
            assert!(base.y - top.y <= 0.19 * base.width + 1e-9);
        }
    }

    #[test]
    fn zones_fit_without_overlap_across_window_sizes() {
        let mut table = table();
        table.toggle(table.view().hand[1]);
        table.toggle(table.view().hand[2]);
        for (width, height) in [
            (560.0, 400.0),
            (920.0, 600.0),
            (560.0, 900.0),
            (1440.0, 400.0),
            (1920.0, 1080.0),
            (200.0, 130.0),
        ] {
            let cards = arrange(&table, width, height);
            let zones = [0..5, 5..15, 15..20];
            for card in &cards {
                let rect = card.rect;
                assert!((rect.height() / rect.width - ASPECT).abs() < 1e-9);
                assert!(rect.x >= 0.0 && rect.y >= 0.0);
                assert!(rect.x + rect.width <= width && rect.y + rect.height() <= height);
            }
            for (index, zone) in zones.iter().enumerate() {
                for other in &zones[index + 1..] {
                    for a in &cards[zone.clone()] {
                        for b in &cards[other.clone()] {
                            let a = a.rect;
                            let b = b.rect;
                            assert!(
                                a.x + a.width <= b.x
                                    || b.x + b.width <= a.x
                                    || a.y + a.height() <= b.y
                                    || b.y + b.height() <= a.y,
                                "zones {zone:?} and {other:?} at {width}×{height}",
                            );
                        }
                    }
                }
            }
        }
        assert!(arrange(&table, 0.0, 400.0).is_empty());
    }

    #[test]
    fn hit_testing_uses_the_top_visible_card_and_tracks_selection_on_resize() {
        let mut table = table();
        let first = table.view().hand[0];
        let second = table.view().hand[1];
        let placements = arrange(&table, 560.0, 400.0);
        let left = placements[15].rect;
        let right = placements[16].rect;
        let x = (left.x + left.width + right.x) / 2.0;
        let y = right.y + right.height() / 2.0;
        assert_eq!(hand_at(&placements, x, y).unwrap().0, second);
        assert!(hand_at(&placements, left.x, left.y).is_none());
        table.toggle(first);
        table.toggle(second);
        for (width, height) in [(560.0, 400.0), (920.0, 600.0)] {
            let placements = arrange(&table, width, height);
            for card in [first, second] {
                let item = placements
                    .iter()
                    .find(|item| matches!(item.face, Face::Front(placed) if placed == card))
                    .unwrap();
                let hit = hand_at(
                    &placements,
                    item.rect.x + item.rect.width * 0.25,
                    item.rect.y + item.rect.height() * 0.06,
                )
                .unwrap();
                assert_eq!(hit.0, card);
                assert!(item.selected);
            }
        }
    }
}
