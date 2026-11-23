use adw::{gdk, glib, gtk, prelude::*};
use schnapsen::core::game::{Player, ReviewedTrick};

use super::paint;

pub(super) fn dialog(owner: Player, tricks: Vec<ReviewedTrick>) -> adw::Dialog {
    let title = match owner {
        Player::Local => "Deine Stiche",
        Player::Opponent => "Erster Stich des Computers",
    };
    let dialog = adw::Dialog::builder()
        .title(title)
        .presentation_mode(adw::DialogPresentationMode::Floating)
        .content_width(720)
        .content_height(420)
        .build();
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
        let number = gtk::Label::new(Some(&format!("Stich {}", trick.number)));
        number.add_css_class("dim-label");
        pair.append(&number);
        let cards = gtk::DrawingArea::builder()
            .content_width(208)
            .content_height(144)
            .halign(gtk::Align::Center)
            .accessible_role(gtk::AccessibleRole::Img)
            .build();
        let [first, second] = trick.cards.map(card_name);
        cards.update_property(&[gtk::accessible::Property::Label(&format!(
            "{first}, {second}"
        ))]);
        cards.set_draw_func(move |_, cr, _, _| {
            if let Err(error) = paint::review_pair(cr, trick.cards) {
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
        .vexpand(true)
        .child(&flow)
        .build();
    let close = gtk::Button::with_label("Schließen");
    close.add_css_class("suggested-action");
    let header = adw::HeaderBar::builder()
        .show_end_title_buttons(false)
        .build();
    header.pack_end(&close);
    let content = adw::ToolbarView::new();
    content.add_top_bar(&header);
    content.set_content(Some(&scroll));
    dialog.set_child(Some(&content));
    dialog.set_focus(Some(&close));
    let weak = dialog.downgrade();
    close.connect_clicked(move |_| {
        if let Some(dialog) = weak.upgrade() {
            dialog.close();
        }
    });
    let keys = gtk::EventControllerKey::new();
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    let weak = dialog.downgrade();
    keys.connect_key_pressed(move |_, key, _, _| {
        if key == gdk::Key::Escape {
            if let Some(dialog) = weak.upgrade() {
                dialog.close();
            }
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });
    dialog.add_controller(keys);
    dialog
}

fn card_name(card: schnapsen::core::cards::Card) -> String {
    use schnapsen::core::cards::{Rank, Suit};
    let suit = match card.suit {
        Suit::Hearts => "Herz",
        Suit::Bells => "Schellen",
        Suit::Leaves => "Grün",
        Suit::Acorns => "Eichel",
    };
    let rank = match card.rank {
        Rank::Ace => "Ass",
        Rank::Ten => "Zehn",
        Rank::King => "König",
        Rank::Ober => "Ober",
        Rank::Unter => "Unter",
    };
    format!("{suit} {rank}")
}
