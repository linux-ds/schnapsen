use adw::gtk::cairo::{Context, Error, FontSlant, FontWeight};
use schnapsen::core::{
    cards::{Card, Rank, Suit},
    game::Player,
    match_play::Match,
};
use std::f64::consts::TAU;

use super::layout::{eye_layout, status_layout, Face, Placement, ASPECT, CORNER_RADIUS};

const TABLE_TEXT: (f64, f64, f64) = (0.93, 0.95, 0.87);

pub(super) fn table(
    cr: &Context,
    placements: &[Placement],
    focus: Option<Card>,
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
            &format!("B {}", session.bummerls(player)),
            y + unit * 0.22,
            0.14,
            0.80,
        )?;
    }
    label("Hände", middle - unit * 0.08, 0.12, 0.80)?;
    label(
        &session.completed_hands().to_string(),
        middle + unit * 0.14,
        0.19,
        1.0,
    )?;
    cr.restore()
}

fn eye_label(value: u8) -> String {
    value.to_string()
}

fn rounded_rect(cr: &Context, x: f64, y: f64, width: f64, height: f64, radius: f64) {
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

pub(super) fn review_pair(cr: &Context, cards: [Card; 2]) -> Result<(), Error> {
    const WIDTH: f64 = 96.0;
    for (index, face) in cards.into_iter().enumerate() {
        cr.save()?;
        cr.translate(3.0 + index as f64 * (WIDTH + 10.0), 3.0);
        cr.scale(WIDTH, WIDTH);
        card(cr, Face::Front(face), false, false, false)?;
        cr.restore()?;
    }
    Ok(())
}

fn card(
    cr: &Context,
    face: Face,
    selected: bool,
    highlighted: bool,
    focused: bool,
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
        Face::Front(card) => front(cr, card)?,
        Face::Back => back(cr)?,
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

fn front(cr: &Context, card: Card) -> Result<(), Error> {
    let value = match card.rank {
        Rank::Ace => "A",
        Rank::Ten => "10",
        Rank::King => "K",
        Rank::Ober => "O",
        Rank::Unter => "U",
    };
    let color = match card.suit {
        Suit::Hearts => (0.67, 0.15, 0.21),
        Suit::Bells => (0.56, 0.32, 0.04),
        Suit::Leaves => (0.12, 0.37, 0.24),
        Suit::Acorns => (0.29, 0.19, 0.14),
    };
    cr.set_source_rgb(color.0, color.1, color.2);
    cr.select_font_face("Sans", FontSlant::Normal, FontWeight::Bold);
    cr.set_font_size(0.255);
    cr.move_to(0.105, 0.315);
    cr.show_text(value)?;
    symbol(cr, card.suit, 0.19, 0.47, 0.19)?;
    symbol(cr, card.suit, 0.50, 0.81, 0.53)
}

fn symbol(cr: &Context, suit: Suit, x: f64, y: f64, size: f64) -> Result<(), Error> {
    cr.save()?;
    cr.translate(x - size / 2.0, y - size / 2.0);
    cr.scale(size, size);
    cr.new_path();
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
    cr.fill()?;
    cr.restore()
}

fn back(cr: &Context) -> Result<(), Error> {
    rounded_rect(cr, 0.055, 0.055, 0.89, ASPECT - 0.11, 0.054);
    cr.set_source_rgb(0.17, 0.34, 0.42);
    cr.fill()?;

    rounded_rect(cr, 0.13, 0.13, 0.74, ASPECT - 0.26, 0.043);
    cr.set_source_rgb(0.52, 0.70, 0.72);
    cr.set_line_width(0.016);
    cr.stroke()?;

    cr.move_to(0.50, 0.44);
    cr.line_to(0.68, 0.70);
    cr.line_to(0.50, 0.96);
    cr.line_to(0.32, 0.70);
    cr.close_path();
    cr.stroke()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::layout::Rect;
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
            table(&cr, std::slice::from_ref(&card), None).unwrap();
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
    fn eye_labels_display_only_digits() {
        assert_eq!(eye_label(0), "0");
        assert_eq!(eye_label(46), "46");
        assert_eq!(eye_label(120), "120");
    }
}
