use super::{rank_german, suit_german, Color, FrontTheme};
use crate::ui::layout::ASPECT;
use adw::gtk::cairo::{Context, Error, FontSlant, FontWeight, LineJoin};
use schnapsen::core::cards::{Card, Rank, Suit};
use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI, TAU};

pub(in crate::ui::design) const THEME: FrontTheme = FrontTheme {
    id: "schnapsen",
    label: msgid!("Double German (Test)"),
    rank,
    suit,
    symbol,
    render: draw,
};

fn rank(rank: Rank) -> (&'static str, &'static str) {
    match rank {
        Rank::Ten => ("X", msgid!("Ten")),
        _ => rank_german(rank),
    }
}

fn suit(suit: Suit) -> (&'static str, Color) {
    let color = match suit {
        Suit::Hearts => (0.73, 0.08, 0.14),
        Suit::Bells => (0.56, 0.32, 0.03),
        Suit::Leaves => (0.12, 0.38, 0.18),
        Suit::Acorns => (0.36, 0.20, 0.11),
    };
    (suit_german(suit).0, color)
}

fn symbol(cr: &Context, suit: Suit) -> Result<(), Error> {
    cr.set_line_width(0.022);
    cr.set_line_join(LineJoin::Round);
    match suit {
        Suit::Hearts => heart(cr),
        Suit::Bells => bell(cr),
        Suit::Leaves => leaf(cr),
        Suit::Acorns => acorn(cr),
    }
}

fn outline(cr: &Context) -> Result<(), Error> {
    cr.set_source_rgb(0.07, 0.06, 0.05);
    cr.stroke()
}

fn field(
    cr: &Context,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    (r, g, b): Color,
) -> Result<(), Error> {
    cr.rectangle(x, y, width, height);
    cr.set_source_rgb(r, g, b);
    cr.fill()
}

// Paint flat ink fields inside the current silhouette, then print its contour.
fn print_shape(
    cr: &Context,
    fields: impl FnOnce(&Context) -> Result<(), Error>,
) -> Result<(), Error> {
    let silhouette = cr.copy_path()?;
    cr.save()?;
    cr.clip();
    let result = fields(cr);
    cr.restore()?;
    result?;
    cr.append_path(&silhouette);
    outline(cr)
}

fn heart(cr: &Context) -> Result<(), Error> {
    cr.move_to(0.50, 0.95);
    cr.curve_to(0.33, 0.75, 0.06, 0.58, 0.05, 0.31);
    cr.curve_to(0.03, 0.06, 0.31, 0.00, 0.50, 0.17);
    cr.curve_to(0.69, 0.00, 0.97, 0.06, 0.95, 0.31);
    cr.curve_to(0.94, 0.58, 0.67, 0.75, 0.50, 0.95);
    cr.close_path();
    print_shape(cr, |cr| {
        field(cr, 0.0, 0.0, 1.0, 1.0, (0.87, 0.09, 0.08))?;
        field(cr, 0.50, 0.0, 0.50, 1.0, (0.62, 0.04, 0.07))?;
        cr.move_to(0.50, 0.17);
        cr.line_to(0.50, 0.95);
        outline(cr)
    })
}

fn bell(cr: &Context) -> Result<(), Error> {
    cr.arc(0.50, 0.90, 0.065, 0.0, TAU);
    cr.set_source_rgb(0.22, 0.42, 0.15);
    cr.fill_preserve()?;
    outline(cr)?;

    cr.arc(0.50, 0.46, 0.435, 0.0, TAU);
    print_shape(cr, |cr| {
        field(cr, 0.0, 0.0, 1.0, 1.0, (0.96, 0.80, 0.15))?;
        field(cr, 0.0, 0.33, 1.0, 0.27, (0.83, 0.11, 0.08))?;
        field(cr, 0.0, 0.60, 1.0, 0.14, (0.76, 0.57, 0.11))?;
        field(cr, 0.0, 0.74, 1.0, 0.26, (0.22, 0.47, 0.17))?;
        field(cr, 0.50, 0.33, 0.50, 0.27, (0.62, 0.07, 0.07))?;
        field(cr, 0.50, 0.60, 0.50, 0.14, (0.59, 0.40, 0.09))?;
        field(cr, 0.50, 0.74, 0.50, 0.26, (0.13, 0.34, 0.13))?;

        for y in [0.33, 0.60, 0.74] {
            cr.move_to(0.04, y);
            cr.line_to(0.96, y);
        }
        outline(cr)?;
        for x in [0.23, 0.50, 0.77] {
            cr.new_sub_path();
            cr.arc(x, 0.465, 0.065, 0.0, TAU);
        }
        outline(cr)
    })
}

fn leaf(cr: &Context) -> Result<(), Error> {
    cr.move_to(0.42, 0.78);
    cr.line_to(0.53, 0.96);
    cr.line_to(0.60, 0.93);
    cr.line_to(0.49, 0.75);
    cr.close_path();
    cr.set_source_rgb(0.54, 0.32, 0.16);
    cr.fill_preserve()?;
    outline(cr)?;

    cr.move_to(0.50, 0.03);
    cr.line_to(0.64, 0.13);
    cr.line_to(0.66, 0.10);
    cr.line_to(0.78, 0.23);
    cr.line_to(0.76, 0.26);
    cr.line_to(0.84, 0.37);
    cr.line_to(0.81, 0.39);
    cr.curve_to(0.98, 0.64, 0.94, 0.86, 0.70, 0.83);
    cr.curve_to(0.61, 0.82, 0.55, 0.79, 0.50, 0.76);
    cr.curve_to(0.38, 0.82, 0.31, 0.86, 0.20, 0.84);
    cr.curve_to(0.00, 0.80, 0.04, 0.64, 0.15, 0.43);
    cr.line_to(0.12, 0.41);
    cr.line_to(0.24, 0.23);
    cr.line_to(0.23, 0.19);
    cr.line_to(0.36, 0.10);
    cr.line_to(0.39, 0.13);
    cr.close_path();
    print_shape(cr, |cr| {
        field(cr, 0.0, 0.0, 1.0, 1.0, (0.92, 0.83, 0.16))?;
        field(cr, 0.50, 0.0, 0.50, 1.0, (0.20, 0.46, 0.16))?;
        cr.move_to(0.50, 0.13);
        cr.line_to(0.50, 0.76);
        for (y, span) in [(0.33, 0.19), (0.49, 0.24), (0.65, 0.31)] {
            cr.move_to(0.50 - span, y - 0.06);
            cr.line_to(0.50, y);
            cr.line_to(0.50 + span, y - 0.06);
        }
        outline(cr)
    })
}

fn acorn(cr: &Context) -> Result<(), Error> {
    cr.move_to(0.54, 0.74);
    cr.curve_to(0.55, 0.87, 0.50, 0.94, 0.36, 0.97);
    cr.line_to(0.42, 0.90);
    cr.curve_to(0.46, 0.87, 0.48, 0.81, 0.47, 0.74);
    cr.close_path();
    cr.set_source_rgb(0.20, 0.41, 0.15);
    cr.fill_preserve()?;
    outline(cr)?;

    cr.move_to(0.34, 0.53);
    cr.line_to(0.34, 0.21);
    cr.curve_to(0.34, 0.10, 0.40, 0.045, 0.50, 0.045);
    cr.curve_to(0.60, 0.045, 0.66, 0.10, 0.66, 0.21);
    cr.line_to(0.66, 0.53);
    cr.close_path();
    print_shape(cr, |cr| {
        field(cr, 0.30, 0.0, 0.40, 0.54, (0.96, 0.80, 0.14))?;
        field(cr, 0.50, 0.0, 0.20, 0.54, (0.72, 0.08, 0.07))?;
        field(cr, 0.30, 0.43, 0.40, 0.12, (0.15, 0.21, 0.47))?;
        field(cr, 0.50, 0.43, 0.20, 0.12, (0.09, 0.10, 0.17))
    })?;

    cr.move_to(0.34, 0.53);
    cr.line_to(0.66, 0.53);
    cr.curve_to(0.84, 0.53, 0.80, 0.77, 0.50, 0.79);
    cr.curve_to(0.20, 0.77, 0.16, 0.53, 0.34, 0.53);
    cr.close_path();
    print_shape(cr, |cr| {
        field(cr, 0.0, 0.53, 1.0, 0.29, (0.24, 0.53, 0.17))?;
        field(cr, 0.50, 0.53, 0.50, 0.29, (0.12, 0.35, 0.13))?;
        for (x, tip) in [(0.33, 0.39), (0.50, 0.50), (0.67, 0.61)] {
            cr.move_to(x - 0.026, 0.60);
            cr.curve_to(x - 0.043, 0.64, tip - 0.025, 0.71, tip, 0.73);
            cr.curve_to(tip - 0.006, 0.70, x + 0.040, 0.63, x + 0.026, 0.60);
            cr.close_path();
        }
        cr.set_source_rgb(0.07, 0.09, 0.05);
        cr.fill()
    })
}

fn draw(cr: &Context, card: Card, theme: &FrontTheme) -> Result<(), Error> {
    let value = theme.rank(card.rank).0;
    let (r, g, b) = theme.suit(card.suit).1;
    let center = ASPECT / 2.0;

    cr.select_font_face("Serif", FontSlant::Normal, FontWeight::Bold);
    for (x, y, angle) in [(0.0, 0.0, 0.0), (1.0, ASPECT, PI)] {
        cr.save()?;
        cr.translate(x, y);
        cr.rotate(angle);
        match card.rank {
            Rank::King => theme.draw_symbol(cr, card.suit, 0.24, 0.26, 0.30)?,
            Rank::Ober => {
                let size = 0.30;
                let top = ASPECT * 0.10;
                theme.draw_symbol(cr, card.suit, 0.24, top + size / 2.0, size)?;
            }
            Rank::Unter => theme.draw_symbol(cr, card.suit, 0.19, ASPECT / 3.0, 0.25)?,
            Rank::Ace => {
                for (x, tilt) in [(0.21, -FRAC_PI_4), (0.79, FRAC_PI_4)] {
                    cr.save()?;
                    cr.translate(x, 0.21);
                    cr.rotate(tilt);
                    theme.draw_symbol(cr, card.suit, 0.0, 0.0, 0.28)?;
                    cr.restore()?;
                }
            }
            Rank::Ten => {
                cr.set_source_rgb(r, g, b);
                draw_index(cr, value, 0.50, 0.12, 0.21)?;
                match card.suit {
                    Suit::Hearts | Suit::Bells => {
                        for x in [0.24, 0.76] {
                            for y in [0.28, 0.36, 0.44, 0.52] {
                                theme.draw_symbol(cr, card.suit, x, y, 0.19)?;
                            }
                        }
                        for y in [0.36, 0.47] {
                            theme.draw_symbol(cr, card.suit, 0.50, y, 0.19)?;
                        }
                    }
                    Suit::Leaves | Suit::Acorns => {
                        for (x, angle) in [(0.24, -FRAC_PI_2), (0.76, FRAC_PI_2)] {
                            for y in [0.26, 0.33, 0.40, 0.47, 0.54] {
                                cr.save()?;
                                cr.translate(x, y);
                                cr.rotate(angle);
                                theme.draw_symbol(cr, card.suit, 0.0, 0.0, 0.19)?;
                                cr.restore()?;
                            }
                        }
                    }
                }
            }
        }
        cr.restore()?;
    }

    if card.rank != Rank::Ten {
        cr.set_source_rgb(r, g, b);
        draw_index(cr, value, 0.50, center, 0.56)?;
    }
    Ok(())
}

fn draw_index(cr: &Context, value: &str, x: f64, y: f64, size: f64) -> Result<(), Error> {
    cr.set_font_size(size);
    let extents = cr.text_extents(value)?;
    cr.move_to(
        x - extents.x_bearing() - extents.width() / 2.0,
        y - extents.y_bearing() - extents.height() / 2.0,
    );
    cr.show_text(value)
}
