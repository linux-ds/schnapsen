use schnapsen::core::cards::Card;
use schnapsen::core::game::Player;

use super::table::Table;

pub(super) const MIN_WIDTH: i32 = 560;
pub(super) const MIN_HEIGHT: i32 = 400;
pub(super) const ASPECT: f64 = 1.4;
pub(super) const CORNER_RADIUS: f64 = 0.085;
const STACK_SPREAD: f64 = 0.19;
const EYE_BASELINE_GAP: f64 = 0.058;
const EYE_FONT_SIZE: f64 = 0.162;

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
    let step = 0.048_f64.min(STACK_SPREAD / steps) * base.width;
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

struct TableGeometry {
    size: f64,
    center: f64,
    top: f64,
    own_y: f64,
    hand_step: f64,
    opponent_stack: Rect,
    own_stack: Rect,
}

impl TableGeometry {
    fn new(width: f64, height: f64) -> Self {
        let size = (width / 6.45).min(height / 4.55);
        let center = width / 2.0;
        let top = size * 0.18;
        let own_y = height - size * (ASPECT + 0.25);
        let hand_step = (width / size / 4.0 - 0.78).clamp(0.82, 1.06) * size;
        let stack_size = size * 0.60;
        let stack_x = center + size * 0.70 + 2.0 * hand_step;
        Self {
            size,
            center,
            top,
            own_y,
            hand_step,
            opponent_stack: Rect::new(stack_x, size * 0.23 + stack_size * STACK_SPREAD, stack_size),
            own_stack: Rect::new(
                stack_x,
                height - size * 0.25 - stack_size * ASPECT,
                stack_size,
            ),
        }
    }

    fn action_top(self, height: f64) -> f64 {
        // Leave space for the native 44 px action row, including on a
        // narrow drawing area whose cards are limited by its width.
        height / 2.0 - (self.size * 0.32).max(28.0_f64.min(height * 0.07))
    }
}

pub(super) fn eye_layout(width: f64, height: f64) -> ([(f64, f64); 2], f64) {
    let geometry = TableGeometry::new(width, height);
    let positions = [geometry.own_stack, geometry.opponent_stack].map(|stack| {
        (
            stack.x + stack.width / 2.0,
            stack.y - STACK_SPREAD * stack.width - EYE_BASELINE_GAP * geometry.size,
        )
    });
    (positions, EYE_FONT_SIZE * geometry.size)
}

pub(super) struct StatusLayout {
    pub x: f64,
    pub width: f64,
    pub rows: [f64; 3],
    pub unit: f64,
}

pub(super) fn trump_indicator(width: f64, height: f64) -> (f64, f64, f64) {
    let unit = TableGeometry::new(width, height).size;
    let radius = unit * 0.18;
    (width - unit * 0.54, height * 0.5, radius)
}

pub(super) fn status_layout(width: f64, height: f64) -> StatusLayout {
    let unit = TableGeometry::new(width, height).size;
    StatusLayout {
        x: unit * 0.25,
        width: unit * 0.40,
        rows: [unit * 0.65, height * 0.5, height - unit * 0.65],
        unit,
    }
}

pub(super) fn arrange(table: &Table, width: f64, height: f64) -> Vec<Placement> {
    if width <= 0.0 || height <= 0.0 {
        return Vec::new();
    }

    let geometry = TableGeometry::new(width, height);
    let TableGeometry {
        size,
        center,
        top,
        own_y,
        hand_step,
        opponent_stack,
        own_stack,
    } = geometry;
    let opponent_size = size * 0.48;
    let played_size = size * 0.71;
    let row_bottom = geometry.action_top(height);
    let played_y = row_bottom - played_size * ASPECT;
    let talon_size = size * 0.70;
    let talon_y = row_bottom - talon_size * ASPECT;
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
    let mut covered_opponent_cards = view.opponent_trick_card_count;
    if let Some(pair) = view.opponent_open_trick {
        let pair_size = size * 0.44;
        let gap = size * 0.09;
        let start = opponent_stack.x - 2.0 * pair_size - 2.0 * gap;
        for (index, card) in pair.into_iter().enumerate() {
            add(
                Face::Front(card),
                Rect::new(start + index as f64 * (pair_size + gap), top, pair_size),
                false,
                false,
                Some(Player::Opponent),
            );
        }
        covered_opponent_cards -= pair.len();
    }
    for rect in stack_positions(opponent_stack, covered_opponent_cards) {
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
        played_y,
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
    for rect in stack_positions(own_stack, view.own_trick_card_count) {
        add(Face::Back, rect, false, false, Some(Player::Local));
    }
    if view.talon_closed {
        placements.push(Placement {
            face: Face::Back,
            rect: Rect::new(
                stock.x - talon_size * 0.15,
                talon_y + talon_size * 0.12,
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

    const VIEWPORTS: [(f64, f64); 8] = [
        (560.0, 400.0),
        (680.0, 560.0),
        (1280.0, 720.0),
        (1920.0, 1080.0),
        (2560.0, 1440.0),
        (3840.0, 2160.0),
        (2560.0, 400.0),
        (560.0, 900.0),
    ];

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
        let mut session = Match::new(
            Player::Local,
            Variant::Soft,
            SchneiderRule::Double,
            |upper| upper - 1,
        );
        for player in [Player::Local, Player::Opponent] {
            let card = session.hand().legal_cards(player)[0];
            session.play_card(player, card).unwrap();
        }
        assert_eq!(session.resolve_trick(), Ok(Player::Local));
        let mut table = Table::new(session);
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
    fn first_computer_trick_is_open_only_in_soft_and_counts_once() {
        for variant in [Variant::Soft, Variant::Sharp] {
            let mut session =
                Match::new(Player::Opponent, variant, SchneiderRule::Double, |upper| {
                    upper - 1
                });
            let mut first = None;
            for _ in 0..2 {
                let pair = [Player::Opponent, Player::Local].map(|player| {
                    let card = session.hand().legal_cards(player)[0];
                    session.play_card(player, card).unwrap();
                    card
                });
                assert_eq!(session.resolve_trick(), Ok(Player::Opponent));
                first.get_or_insert(pair);
            }
            let table = Table::new(session);
            let expected = if variant == Variant::Soft {
                first
            } else {
                None
            };
            assert_eq!(table.view().opponent_open_trick, expected);

            for (width, height) in [(560.0, 400.0), (680.0, 560.0), (920.0, 650.0)] {
                let cards = arrange(&table, width, height);
                assert_eq!(cards.len(), 20);
                let computer_tricks: Vec<_> = cards
                    .iter()
                    .filter(|entry| entry.trick_owner == Some(Player::Opponent))
                    .collect();
                assert_eq!(computer_tricks.len(), 4);
                let open: Vec<_> = computer_tricks
                    .iter()
                    .filter_map(|entry| match entry.face {
                        Face::Front(card) => Some(card),
                        Face::Back => None,
                    })
                    .collect();
                assert_eq!(open, expected.into_iter().flatten().collect::<Vec<_>>());
                for card in open {
                    assert_eq!(
                        cards
                            .iter()
                            .filter(
                                |entry| matches!(entry.face, Face::Front(shown) if shown == card)
                            )
                            .count(),
                        1
                    );
                }
                let opponent_hand_right = cards[..5]
                    .iter()
                    .map(|entry| entry.rect.x + entry.rect.width)
                    .fold(0.0, f64::max);
                assert!(computer_tricks
                    .iter()
                    .all(|entry| entry.rect.x > opponent_hand_right));
            }
        }
    }

    #[test]
    fn board_reserves_a_clear_center_action_row_with_two_cards_in_play() {
        let mut session = Match::new(
            Player::Local,
            Variant::Soft,
            SchneiderRule::Double,
            |upper| upper - 1,
        );
        for player in [Player::Local, Player::Opponent] {
            let card = session.hand().legal_cards(player)[0];
            session.play_card(player, card).unwrap();
        }
        let mut table = Table::new(session);
        table.toggle(table.view().hand[0]);
        for (width, height) in VIEWPORTS {
            let cards = arrange(&table, width, height);
            let center = height / 2.0;
            for entry in cards {
                let rect = entry.rect;
                let bottom = rect.y
                    + if entry.sideways {
                        rect.width
                    } else {
                        rect.height()
                    };
                assert!(bottom < center - 22.0 || rect.y > center + 22.0);
            }
        }
    }

    #[test]
    fn eye_labels_stay_above_the_full_stack_at_all_window_sizes() {
        for (width, height) in VIEWPORTS {
            let geometry = TableGeometry::new(width, height);
            let stacks = [geometry.own_stack, geometry.opponent_stack];
            let (positions, font_size) = eye_layout(width, height);
            for ((center, baseline), stack) in positions.into_iter().zip(stacks) {
                assert!(center > font_size && center < width - font_size);
                assert!(baseline >= font_size);
                for card in stack_positions(stack, 20) {
                    assert!(baseline + 0.28 * font_size < card.y);
                }
            }
        }
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
        for (width, height) in VIEWPORTS.into_iter().chain([(200.0, 130.0)]) {
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
    fn cards_keep_growing_and_scale_proportionally_to_the_logical_viewport() {
        let table = table();
        let mut previous = 0.0;
        for (width, height) in VIEWPORTS[..6].iter().copied() {
            let cards = arrange(&table, width, height);
            let size = cards.iter().find(|card| card.in_hand).unwrap().rect.width;
            assert!(size > previous, "hand did not grow at {width}×{height}");
            assert!((cards[0].rect.width / size - 0.48).abs() < 1e-9);
            let trump = cards.iter().find(|card| card.open_trump).unwrap();
            assert!((trump.rect.width / size - 0.70).abs() < 1e-9);
            previous = size;
        }
        let reference = arrange(&table, 1280.0, 720.0);
        let large = arrange(&table, 3840.0, 2160.0);
        for (small, large) in reference.iter().zip(&large) {
            assert!((3.0 * small.rect.width - large.rect.width).abs() < 1e-9);
            assert!((3.0 * small.rect.x - large.rect.x).abs() < 1e-9);
            assert!((3.0 * small.rect.y - large.rect.y).abs() < 1e-9);
        }
        let (small_positions, small_font) = eye_layout(1280.0, 720.0);
        let (large_positions, large_font) = eye_layout(3840.0, 2160.0);
        assert!((3.0 * small_font - large_font).abs() < 1e-9);
        for ((x, y), (large_x, large_y)) in small_positions.into_iter().zip(large_positions) {
            assert!((3.0 * x - large_x).abs() < 1e-9);
            assert!((3.0 * y - large_y).abs() < 1e-9);
        }
    }

    #[test]
    fn full_hands_stay_inside_the_viewport_in_open_and_closed_states() {
        for closed in [false, true] {
            let mut session = Match::new(
                Player::Opponent,
                Variant::Soft,
                SchneiderRule::Double,
                |upper| upper - 1,
            );
            while let Some(player) = session.hand().next_player() {
                if closed && session.hand().check_close_talon(player).is_ok() {
                    session.close_talon(player).unwrap();
                }
                let card = session.hand().legal_cards(player)[0];
                session.play_card(player, card).unwrap();
                let mut table = Table::new(session);
                for card in table
                    .view()
                    .hand
                    .iter()
                    .take(2)
                    .copied()
                    .collect::<Vec<_>>()
                {
                    table.toggle(card);
                }
                for (width, height) in VIEWPORTS {
                    let status = status_layout(width, height);
                    let [top, middle, bottom] = status.rows;
                    assert!(status.x >= status.width / 2.0);
                    assert!(top > status.unit * 0.28);
                    assert!(top + status.unit * 0.22 < middle - status.unit * 0.24);
                    assert!(middle < bottom - status.unit * 0.28);
                    assert!(bottom + status.unit * 0.22 < height);
                    let (trump_x, trump_y, radius) = trump_indicator(width, height);
                    assert!(trump_x - radius > width / 2.0 + 180.0);
                    assert!(trump_x + radius < width);
                    assert!(trump_y - radius > 0.0 && trump_y + radius < height);
                    let placements = arrange(&table, width, height);
                    assert_eq!(placements.len(), 20);
                    for card in placements {
                        let rect = card.rect;
                        let (w, h) = if card.sideways {
                            (rect.height(), rect.width)
                        } else {
                            (rect.width, rect.height())
                        };
                        assert!(rect.x > status.x + status.width / 2.0);
                        assert!(rect.x + w <= width);
                        if rect.x + w >= trump_x - radius {
                            assert!(rect.y + h < trump_y - radius || rect.y > trump_y + radius);
                        }
                        assert!(rect.y >= 0.0 && rect.y + h <= height);
                    }
                }
                session = table.into_session();
                if session.hand().view(player).played.len() == 2 {
                    session.resolve_trick().unwrap();
                }
            }
            assert_eq!(session.hand().talon_closure().is_some(), closed);
        }
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
        for (width, height) in VIEWPORTS {
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
