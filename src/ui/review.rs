use adw::{gdk, glib, gtk, prelude::*};
use schnapsen::core::game::{Player, ReviewedTrick};

use super::{design::FrontTheme, paint};

pub(super) fn layer(
    owner: Player,
    tricks: Vec<ReviewedTrick>,
    theme: &'static FrontTheme,
    dismiss: impl Fn() + Clone + 'static,
) -> (gtk::Overlay, gtk::Button) {
    let title = match owner {
        Player::Local => tr!("Your tricks"),
        Player::Opponent => tr!("Computer’s first trick"),
    };
    let layer = gtk::Overlay::new();
    layer.set_halign(gtk::Align::Fill);
    layer.set_valign(gtk::Align::Fill);
    let scrim = gtk::DrawingArea::new();
    scrim.set_can_target(true);
    scrim.set_draw_func(|_, cr, _, _| {
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.43);
        let _ = cr.paint();
    });
    layer.set_child(Some(&scrim));
    let click = gtk::GestureClick::new();
    click.set_button(gdk::BUTTON_PRIMARY);
    let close_outside = dismiss.clone();
    click.connect_released(move |click, count, _, _| {
        if count == 1 {
            click.set_state(gtk::EventSequenceState::Claimed);
            close_outside();
        }
    });
    scrim.add_controller(click);
    let flow = gtk::FlowBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .min_children_per_line(1)
        .max_children_per_line(3)
        .column_spacing(10)
        .row_spacing(12)
        .margin_top(12)
        .margin_bottom(12)
        .margin_start(12)
        .margin_end(12)
        .valign(gtk::Align::Start)
        .build();
    for trick in tricks {
        let pair = gtk::Box::new(gtk::Orientation::Vertical, 6);
        let number = gtk::Label::new(Some(&tr!("Trick {number}", number = trick.number)));
        number.add_css_class("dim-label");
        pair.append(&number);
        let cards = gtk::DrawingArea::builder()
            .content_width(208)
            .content_height(144)
            .halign(gtk::Align::Center)
            .accessible_role(gtk::AccessibleRole::Img)
            .build();
        let [first, second] = trick.cards.map(|card| theme.card_name(card));
        cards.update_property(&[gtk::accessible::Property::Label(&tr!(
            "{first}, {second}",
            first = first,
            second = second
        ))]);
        cards.set_draw_func(move |_, cr, _, _| {
            if let Err(error) = paint::review_pair(cr, trick.cards, theme) {
                eprintln!("Could not draw reviewed trick: {error}");
            }
        });
        pair.append(&cards);
        let child = gtk::FlowBoxChild::builder()
            .focusable(false)
            .child(&pair)
            .build();
        flow.insert(&child, -1);
    }
    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .min_content_width(460)
        .max_content_width(690)
        .propagate_natural_width(true)
        .min_content_height(275)
        .max_content_height(360)
        .propagate_natural_height(true)
        .vexpand(true)
        .child(&flow)
        .build();
    let close = gtk::Button::with_label(&tr!("Close"));
    close.add_css_class("suggested-action");
    let header = adw::HeaderBar::builder()
        .show_start_title_buttons(false)
        .show_end_title_buttons(false)
        .title_widget(&adw::WindowTitle::new(&title, ""))
        .build();
    header.pack_end(&close);
    let content = adw::ToolbarView::new();
    content.add_top_bar(&header);
    content.set_content(Some(&scroll));
    content.add_css_class("card");
    content.set_halign(gtk::Align::Center);
    content.set_valign(gtk::Align::Center);
    content.set_overflow(gtk::Overflow::Hidden);
    content.set_margin_start(12);
    content.set_margin_end(12);
    content.set_margin_top(12);
    content.set_margin_bottom(12);
    layer.add_overlay(&content);
    let close_button = dismiss.clone();
    close.connect_clicked(move |_| close_button());
    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    keys.connect_key_pressed(move |_, key, _, _| {
        if key == gdk::Key::Escape {
            dismiss();
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });
    layer.add_controller(keys);
    (layer, close)
}
