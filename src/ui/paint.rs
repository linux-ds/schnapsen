use adw::gtk::cairo::{Context, Error, FontSlant, FontWeight};
use schnapsen::core::{
    cards::{Card, Suit},
    game::Player,
    match_play::Match,
};
use std::f64::consts::TAU;

use super::design::{rounded_rect, CardBack, CardDesign, FrontTheme};
use super::layout::{
    eye_layout, status_layout, trump_indicator, Face, Placement, ASPECT, CORNER_RADIUS,
};

const TABLE_TEXT: (f64, f64, f64) = (0.93, 0.95, 0.87);

pub(super) fn table(
    cr: &Context,
    placements: &[Placement],
    focus: Option<Card>,
    design: CardDesign,
) -> Result<(), Error> {
    cr.set_source_rgb(0.10, 0.25, 0.21);
    cr.paint()?;

    for placement in placements {
        let rect = placement.rect;
        cr.save()?;
        if placement.sideways {
            cr.translate(rect.x + rect.height(), rect.y);
            cr.rotate(TAU / 4.0);
        } else {
            cr.translate(rect.x, rect.y);
        }
        cr.scale(rect.width, rect.width);
        let focused =
            placement.in_hand && matches!(placement.face, Face::Front(card) if focus == Some(card));
        card(
            cr,
            placement.face,
            placement.selected,
            placement.highlighted,
            focused,
            design,
        )?;
        cr.restore()?;
    }
    Ok(())
}

pub(super) fn eyes(
    cr: &Context,
    width: f64,
    height: f64,
    local: u8,
    opponent: Option<u8>,
) -> Result<(), Error> {
    cr.save()?;
    cr.select_font_face("Sans", FontSlant::Normal, FontWeight::Bold);
    let (positions, font_size) = eye_layout(width, height);
    cr.set_font_size(font_size);
    let (r, g, b) = TABLE_TEXT;
    cr.set_source_rgb(r, g, b);
    for ((x, y), value) in positions.into_iter().zip([Some(local), opponent]) {
        if let Some(value) = value {
            let label = eye_label(value);
            let text_width = cr.text_extents(&label)?.x_advance();
            cr.move_to(x - text_width / 2.0, y);
            cr.show_text(&label)?;
        }
    }
    cr.restore()
}

pub(super) fn match_status(
    cr: &Context,
    width: f64,
    height: f64,
    session: &Match,
) -> Result<(), Error> {
    let layout = status_layout(width, height);
    let [top, middle, bottom] = layout.rows;
    let unit = layout.unit;
    let (r, g, b) = TABLE_TEXT;
    cr.save()?;
    cr.select_font_face("Sans", FontSlant::Normal, FontWeight::Bold);
    let label = |text: &str, y, size, opacity| -> Result<(), Error> {
        cr.set_source_rgba(r, g, b, opacity);
        cr.set_font_size(size * unit);
        let metrics = cr.text_extents(text)?;
        if metrics.width() > layout.width {
            cr.set_font_size(size * unit * layout.width / metrics.width());
        }
        let metrics = cr.text_extents(text)?;
        cr.move_to(layout.x - metrics.x_bearing() - metrics.width() / 2.0, y);
        cr.show_text(text)
    };
    for (player, y) in [(Player::Opponent, top), (Player::Local, bottom)] {
        label(&session.remaining(player).to_string(), y, 0.28, 1.0)?;
        label(
            &tr!("B {count}", count = session.bummerls(player)),
            y + unit * 0.22,
            0.14,
            0.80,
        )?;
    }
    label(&tr!("Hands"), middle - unit * 0.08, 0.12, 0.80)?;
    label(
        &session.completed_hands().to_string(),
        middle + unit * 0.14,
        0.19,
        1.0,
    )?;
    cr.restore()
}

pub(super) fn trump(
    cr: &Context,
    width: f64,
    height: f64,
    suit: Suit,
    theme: &FrontTheme,
) -> Result<(), Error> {
    let (x, y, radius) = trump_indicator(width, height);
    cr.save()?;
    cr.new_sub_path();
    cr.arc(x, y, radius, 0.0, TAU);
    cr.set_source_rgb(0.98, 0.96, 0.90);
    cr.fill()?;
    theme.draw_symbol(cr, suit, x, y, radius * 1.35)?;
    cr.restore()
}

fn eye_label(value: u8) -> String {
    value.to_string()
}

pub(super) fn review_pair(
    cr: &Context,
    cards: [Card; 2],
    theme: &'static FrontTheme,
) -> Result<(), Error> {
    let design = CardDesign {
        front: theme,
        ..CardDesign::default()
    };
    const WIDTH: f64 = 96.0;
    for (index, face) in cards.into_iter().enumerate() {
        cr.save()?;
        cr.translate(3.0 + index as f64 * (WIDTH + 10.0), 3.0);
        cr.scale(WIDTH, WIDTH);
        card(cr, Face::Front(face), false, false, false, design)?;
        cr.restore()?;
    }
    Ok(())
}

pub(super) fn back_preview(
    cr: &Context,
    width: f64,
    height: f64,
    back: &'static CardBack,
) -> Result<(), Error> {
    let size = width.min(height / ASPECT) * 0.94;
    cr.save()?;
    cr.translate((width - size) / 2.0, (height - size * ASPECT) / 2.0);
    cr.scale(size, size);
    card(
        cr,
        Face::Back,
        false,
        false,
        false,
        CardDesign {
            back,
            ..CardDesign::default()
        },
    )?;
    cr.restore()
}

fn card(
    cr: &Context,
    face: Face,
    selected: bool,
    highlighted: bool,
    focused: bool,
    design: CardDesign,
) -> Result<(), Error> {
    rounded_rect(cr, 0.014, 0.032, 1.0, ASPECT, CORNER_RADIUS);
    cr.set_source_rgba(0.02, 0.08, 0.06, 0.38);
    cr.fill()?;

    rounded_rect(cr, 0.0, 0.0, 1.0, ASPECT, CORNER_RADIUS);
    cr.set_source_rgb(0.98, 0.96, 0.90);
    cr.fill_preserve()?;
    if selected {
        cr.set_source_rgb(0.79, 0.53, 0.14);
        cr.set_line_width(0.027);
    } else {
        cr.set_source_rgb(0.68, 0.65, 0.55);
        cr.set_line_width(0.012);
    }
    cr.stroke()?;

    match face {
        Face::Front(card) => design.front.draw(cr, card)?,
        Face::Back => design.back.draw(cr)?,
    }
    if highlighted {
        cr.new_sub_path();
        cr.arc(0.16, ASPECT - 0.14, 0.043, 0.0, TAU);
        cr.set_source_rgb(0.20, 0.43, 0.29);
        cr.fill()?;
    }
    if focused {
        rounded_rect(cr, 0.046, 0.046, 0.908, ASPECT - 0.092, 0.047);
        cr.set_source_rgb(0.19, 0.34, 0.48);
        cr.set_line_width(0.017);
        cr.set_dash(&[0.042, 0.032], 0.0);
        cr.stroke()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::{
        design::{BACKS, FRONTS},
        layout::Rect,
    };
    use adw::gtk::cairo::{Format, ImageSurface};

    #[test]
    fn cairo_device_scale_keeps_card_geometry_in_logical_pixels() {
        let card = Placement {
            face: Face::Back,
            rect: Rect::new(40.0, 30.0, 80.0),
            in_hand: false,
            open_trump: false,
            talon_stack: false,
            trick_owner: None,
            sideways: false,
            selected: false,
            highlighted: false,
        };
        for scale in [1.0, 1.5, 2.0] {
            let mut surface = ImageSurface::create(
                Format::ARgb32,
                (160.0 * scale) as i32,
                (160.0 * scale) as i32,
            )
            .unwrap();
            surface.set_device_scale(scale, scale);
            let cr = Context::new(&surface).unwrap();
            table(
                &cr,
                std::slice::from_ref(&card),
                None,
                CardDesign::default(),
            )
            .unwrap();
            drop(cr);
            let stride = surface.stride() as usize;
            let data = surface.data().unwrap();
            let pixel = |x: f64, y: f64| {
                let start = (y * scale) as usize * stride + (x * scale) as usize * 4;
                u32::from_ne_bytes(data[start..start + 4].try_into().unwrap())
            };
            let background = pixel(20.0, 80.0);
            assert_eq!(pixel(34.0, 80.0), background);
            assert_ne!(pixel(46.0, 80.0), background);
            assert_ne!(pixel(114.0, 80.0), background);
            assert_eq!(pixel(126.0, 80.0), background);
        }
    }

    #[test]
    fn trump_indicator_renders_all_four_suits_at_small_and_large_sizes() {
        for (width, height) in [(560, 400), (3840, 2160)] {
            for theme in FRONTS {
                for suit in [Suit::Hearts, Suit::Bells, Suit::Leaves, Suit::Acorns] {
                    let mut surface = ImageSurface::create(Format::ARgb32, width, height).unwrap();
                    let cr = Context::new(&surface).unwrap();
                    trump(&cr, width.into(), height.into(), suit, theme).unwrap();
                    drop(cr);
                    let (x, y, radius) = trump_indicator(width.into(), height.into());
                    if theme.id == "schnapsen" {
                        assert_test_symbol_colors(&mut surface, suit, x, y, radius);
                        continue;
                    }
                    let stride = surface.stride() as usize;
                    let data = surface.data().unwrap();
                    let (r, g, b) = theme.suit(suit).1;
                    let channels = [r, g, b].map(|channel| (channel * 255.0).round() as u32);
                    assert!(((y - radius) as usize..(y + radius) as usize).any(|row| {
                        ((x - radius) as usize..(x + radius) as usize).any(|col| {
                            let start = row * stride + col * 4;
                            let pixel =
                                u32::from_ne_bytes(data[start..start + 4].try_into().unwrap());
                            pixel >> 24 == 255
                                && [16, 8, 0].into_iter().zip(channels).all(|(shift, target)| {
                                    ((pixel >> shift) & 255).abs_diff(target) <= 1
                                })
                        })
                    }));
                }
            }
        }
    }

    fn assert_test_symbol_colors(surface: &mut ImageSurface, suit: Suit, x: f64, y: f64, r: f64) {
        let stride = surface.stride() as usize;
        let data = surface.data().unwrap();
        let mut first = false;
        let mut second = false;
        let mut bell_band = false;
        for row in (y - r) as usize..(y + r).ceil() as usize {
            for col in (x - r) as usize..(x + r).ceil() as usize {
                let start = row * stride + col * 4;
                let [a, red, green, blue] =
                    u32::from_ne_bytes(data[start..start + 4].try_into().unwrap()).to_be_bytes();
                if a != 255 {
                    continue;
                }
                let is_green = i16::from(green) > i16::from(red) + 20 && green > blue;
                match suit {
                    Suit::Hearts => {
                        first |= red > 200 && green < 95 && blue < 90;
                        second |= (140..200).contains(&red) && green < 55 && blue < 80;
                    }
                    Suit::Bells => {
                        first |= red > 165 && green > 115 && blue < 100;
                        second |= is_green;
                        bell_band |= red > 140 && green < 100 && blue < 80;
                    }
                    Suit::Acorns => {
                        first |= (row as f64) < y && red > 165 && green > 115 && blue < 100;
                        second |= (row as f64) >= y && is_green;
                    }
                    Suit::Leaves => {
                        first |= green > 142 && red > 115 && blue < 100;
                        second |= is_green;
                    }
                }
            }
        }
        assert!(
            first && second,
            "Missing one of the two main colors for {suit:?}"
        );
        assert!(suit != Suit::Bells || bell_band, "Missing red bell band");
    }

    #[test]
    fn schnapsen_test_ten_repeats_multicolored_symbols_on_small_cards() {
        use schnapsen::core::cards::Rank;

        let design = CardDesign::from_ids("schnapsen", "legacy");
        let width = 56;
        let scale = f64::from(width);
        let height = (scale * ASPECT).ceil() as i32;
        for suit in [Suit::Hearts, Suit::Bells, Suit::Leaves, Suit::Acorns] {
            let mut surface = ImageSurface::create(Format::ARgb32, width, height).unwrap();
            let cr = Context::new(&surface).unwrap();
            cr.scale(scale, scale);
            let face = Face::Front(Card {
                suit,
                rank: Rank::Ten,
            });
            card(&cr, face, false, false, false, design).unwrap();
            drop(cr);
            for (x, y) in [(0.33, 0.46), (0.50, ASPECT / 2.0), (0.67, 0.94)] {
                assert_test_symbol_colors(&mut surface, suit, x * scale, y * scale, 0.12 * scale);
            }
        }
    }

    #[test]
    fn schnapsen_test_front_renders_twenty_distinct_cards() {
        use schnapsen::core::cards::full_deck;
        use std::collections::HashSet;

        let design = CardDesign::from_ids("schnapsen", "legacy");
        assert_eq!(design.front.id, "schnapsen");
        for width in [32, 88] {
            let height = (f64::from(width) * ASPECT).ceil() as i32;
            let mut fronts = HashSet::new();
            for identity in full_deck() {
                let mut surface = ImageSurface::create(Format::ARgb32, width, height).unwrap();
                let cr = Context::new(&surface).unwrap();
                let scale = f64::from(width);
                cr.scale(scale, scale);
                card(&cr, Face::Front(identity), false, false, false, design).unwrap();
                drop(cr);
                assert!(
                    fronts.insert(surface.data().unwrap().to_vec()),
                    "{identity:?}"
                );
            }
        }
    }

    #[test]
    fn all_card_backs_render_distinct_patterns_at_both_scales() {
        use std::{
            collections::HashSet,
            hash::{DefaultHasher, Hash, Hasher},
        };

        for (width, height) in [(32, 45), (220, 308)] {
            let mut signatures = HashSet::new();
            for back in BACKS {
                let mut surface = ImageSurface::create(Format::ARgb32, width, height).unwrap();
                let cr = Context::new(&surface).unwrap();
                back_preview(&cr, width.into(), height.into(), back).unwrap();
                drop(cr);
                let data = surface.data().unwrap();
                assert!(data.iter().any(|&channel| channel != 0), "{}", back.id);
                let mut hasher = DefaultHasher::new();
                data.hash(&mut hasher);
                assert!(signatures.insert(hasher.finish()), "{}", back.id);
            }
        }
    }

    #[test]
    fn eye_labels_display_only_digits() {
        assert_eq!(eye_label(0), "0");
        assert_eq!(eye_label(46), "46");
        assert_eq!(eye_label(120), "120");
    }
}
