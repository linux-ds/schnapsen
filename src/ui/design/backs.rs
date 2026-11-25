use super::{rounded_rect, CardBack, Color};
use crate::ui::layout::ASPECT;
use adw::gtk::cairo::{Context, Error};
use std::f64::consts::TAU;

pub(super) fn legacy(cr: &Context) -> Result<(), Error> {
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

fn panel(cr: &Context, base: Color, ink: Color) -> Result<(), Error> {
    rounded_rect(cr, 0.055, 0.055, 0.89, ASPECT - 0.11, 0.054);
    cr.set_source_rgb(base.0, base.1, base.2);
    cr.fill()?;
    rounded_rect(cr, 0.10, 0.10, 0.80, ASPECT - 0.20, 0.038);
    cr.set_source_rgb(ink.0, ink.1, ink.2);
    cr.set_line_width(0.012);
    cr.stroke_preserve()?;
    cr.clip();
    Ok(())
}

fn diamond(cr: &Context, x: f64, y: f64, wide: f64, tall: f64) {
    cr.move_to(x, y - tall);
    cr.line_to(x + wide, y);
    cr.line_to(x, y + tall);
    cr.line_to(x - wide, y);
    cr.close_path();
}

fn rosette(cr: &Context, x: f64, y: f64, radius: f64, petals: usize) {
    for petal in 0..petals {
        let angle = TAU * petal as f64 / petals as f64;
        let a = x + radius * angle.cos();
        let b = y + radius * angle.sin();
        cr.new_sub_path();
        cr.arc(a, b, radius * 0.73, 0.0, TAU);
    }
}

fn classic(cr: &Context) -> Result<(), Error> {
    panel(cr, (0.08, 0.17, 0.33), (0.56, 0.72, 0.86))?;
    cr.set_line_width(0.011);
    for row in -3..=3 {
        for col in -2..=2 {
            diamond(
                cr,
                0.5 + f64::from(col) * 0.171,
                0.70 + f64::from(row) * 0.175,
                0.085,
                0.086,
            );
        }
    }
    cr.stroke()
}

fn viennese(cr: &Context) -> Result<(), Error> {
    panel(cr, (0.39, 0.075, 0.13), (0.97, 0.84, 0.66))?;
    cr.set_line_width(0.015);
    for y in [0.29, 1.11] {
        rosette(cr, 0.5, y, 0.072, 6);
        for x in [0.23, 0.77] {
            diamond(cr, x, y, 0.035, 0.055);
        }
    }
    rosette(cr, 0.5, 0.7, 0.149, 8);
    diamond(cr, 0.5, 0.7, 0.31, 0.31);
    cr.stroke()
}

fn alpine(cr: &Context) -> Result<(), Error> {
    panel(cr, (0.09, 0.25, 0.18), (0.68, 0.84, 0.69))?;
    cr.set_line_width(0.022);
    for direction in [-1.0, 1.0] {
        for offset in [0.0, 0.15] {
            for (index, (x, y)) in [
                (0.17, 0.08),
                (0.33, -0.09),
                (0.42, -0.01),
                (0.52, -0.16),
                (0.83, 0.08),
            ]
            .into_iter()
            .enumerate()
            {
                let x = 0.5 + (x - 0.5) * direction;
                let y = 0.7 + direction * (0.22 + offset + y);
                if index == 0 {
                    cr.move_to(x, y);
                } else {
                    cr.line_to(x, y);
                }
            }
        }
    }
    diamond(cr, 0.5, 0.7, 0.085, 0.06);
    cr.stroke()
}

fn geometric(cr: &Context) -> Result<(), Error> {
    panel(cr, (0.15, 0.16, 0.18), (0.90, 0.73, 0.42))?;
    cr.set_line_width(0.014);
    for extent in [0.12, 0.22, 0.32] {
        diamond(cr, 0.5, 0.7, extent, extent * 1.55);
    }
    for x in [0.18, 0.82] {
        cr.move_to(x, 0.2);
        cr.line_to(x, 1.2);
    }
    for y in [0.22, 1.18] {
        diamond(cr, 0.5, y, 0.044, 0.063);
    }
    cr.stroke()
}

fn ornament(cr: &Context) -> Result<(), Error> {
    panel(cr, (0.31, 0.15, 0.44), (0.88, 0.75, 0.92))?;
    cr.set_line_width(0.009);
    for y in [0.26, 1.14] {
        for x in [0.28, 0.72] {
            rosette(cr, x, y, 0.052, 8);
        }
    }
    rosette(cr, 0.5, 0.7, 0.16, 12);
    cr.new_sub_path();
    cr.arc(0.5, 0.7, 0.305, 0.0, TAU);
    cr.stroke()
}

fn minimal(cr: &Context) -> Result<(), Error> {
    panel(cr, (0.92, 0.87, 0.76), (0.34, 0.30, 0.25))?;
    cr.set_line_width(0.017);
    diamond(cr, 0.5, 0.7, 0.12, 0.17);
    cr.stroke()
}

pub(super) const LEGACY: CardBack = CardBack {
    id: "legacy",
    label: msgid!("Original"),
    render: legacy,
};
pub(super) const CLASSIC: CardBack = CardBack {
    id: "classic",
    label: msgid!("Classic"),
    render: classic,
};
pub(super) const VIENNESE: CardBack = CardBack {
    id: "viennese",
    label: msgid!("Viennese"),
    render: viennese,
};
pub(super) const ALPINE: CardBack = CardBack {
    id: "alpine",
    label: msgid!("Alpine"),
    render: alpine,
};
pub(super) const GEOMETRIC: CardBack = CardBack {
    id: "geometric",
    label: msgid!("Geometric"),
    render: geometric,
};
pub(super) const ORNAMENT: CardBack = CardBack {
    id: "ornament",
    label: msgid!("Ornamental"),
    render: ornament,
};
pub(super) const MINIMAL: CardBack = CardBack {
    id: "minimal",
    label: msgid!("Minimal"),
    render: minimal,
};
