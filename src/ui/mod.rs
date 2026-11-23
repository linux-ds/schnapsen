use adw::{gtk, prelude::*};
use schnapsen::core::{
    game::{Player, Variant},
    match_play::{Match, SchneiderRule},
};
use std::{cell::RefCell, rc::Rc};

mod input;
mod layout;
mod paint;
mod review;
mod table;
mod turn;

fn choose_index(upper: usize) -> usize {
    adw::glib::random_int_range(0, upper as i32) as usize
}

pub fn build_window(app: &adw::Application) -> adw::ApplicationWindow {
    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let header = adw::HeaderBar::new();
    let title = adw::WindowTitle::new("Schnapsen", "Weiches Schnapsen");
    header.set_title_widget(Some(&title));
    let options = gtk::Box::new(gtk::Orientation::Vertical, 8);
    options.set_margin_start(12);
    options.set_margin_end(12);
    options.set_margin_top(12);
    options.set_margin_bottom(12);
    options.append(&gtk::Label::new(Some("Für die nächste Partie")));
    let variant = gtk::DropDown::from_strings(&["Weich", "Scharf"]);
    options.append(&variant);
    let schneider = gtk::CheckButton::with_label("Schneider zählt doppelt");
    schneider.set_active(true);
    options.append(&schneider);
    let reset_bummerls = gtk::Button::with_label("Bummerl zurücksetzen …");
    options.append(&reset_bummerls);
    let popover = gtk::Popover::new();
    popover.set_child(Some(&options));
    let match_menu = gtk::MenuButton::builder()
        .label("Partie")
        .popover(&popover)
        .build();
    header.pack_start(&match_menu);
    let help = gtk::CheckButton::with_label("Spielhilfe");
    help.set_tooltip_text(Some("Zulässige Karten markieren"));
    header.pack_end(&help);
    let declare = gtk::Button::with_label("Ausmelden");
    declare.set_sensitive(false);
    header.pack_end(&declare);
    let eyes = gtk::Label::new(Some("Deine Augen: 0"));
    header.pack_end(&eyes);
    content.append(&header);

    let table = gtk::DrawingArea::builder()
        .width_request(layout::MIN_WIDTH)
        .height_request(layout::MIN_HEIGHT)
        .vexpand(true)
        .focusable(true)
        .build();
    let state = Rc::new(RefCell::new(table::Table::new(Match::new(
        Player::Local,
        Variant::Soft,
        SchneiderRule::default(),
        choose_index,
    ))));
    let score = gtk::Label::builder()
        .wrap(true)
        .margin_top(6)
        .margin_bottom(6)
        .build();
    content.append(&score);
    let notice = gtk::Label::builder()
        .wrap(true)
        .css_classes(["dim-label"])
        .margin_top(8)
        .margin_bottom(8)
        .visible(false)
        .build();
    let overlay = adw::ToastOverlay::new();
    let marriage_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    marriage_row.set_halign(gtk::Align::Center);
    let king = gtk::Button::new();
    let ober = gtk::Button::new();
    let proceed = gtk::Button::with_label("Weiterspielen");
    let announcement = gtk::Label::new(None);
    let advance = gtk::Button::builder()
        .visible(false)
        .halign(gtk::Align::Center)
        .margin_bottom(8)
        .build();
    announcement.add_css_class("title-3");
    for button in [&king, &ober, &proceed] {
        button.set_visible(false);
        marriage_row.append(button);
    }
    announcement.set_visible(false);
    marriage_row.append(&announcement);
    let controls = turn::TableControls {
        king: king.downgrade(),
        ober: ober.downgrade(),
        proceed: proceed.downgrade(),
        announcement: announcement.downgrade(),
        score: score.downgrade(),
        advance: advance.downgrade(),
    };
    let timers =
        turn::TurnTimers::new(&state, &table, &notice, &overlay, &eyes, &declare, controls);
    timers.refresh();
    let touch = input::attach(&table, &state, &timers);
    for (button, play_ober) in [(&king, false), (&ober, true)] {
        let model = state.clone();
        let turns = timers.clone();
        let area = table.downgrade();
        button.connect_clicked(move |_| {
            if model.borrow_mut().announce_selected(play_ober) {
                if let Some(area) = area.upgrade() {
                    area.grab_focus();
                }
                turns.refresh();
            }
        });
    }
    let proceed_model = state.clone();
    let proceed_turns = timers.clone();
    let proceed_area = table.downgrade();
    proceed.connect_clicked(move |_| {
        proceed_model.borrow_mut().continue_play();
        if let Some(area) = proceed_area.upgrade() {
            area.grab_focus();
        }
        proceed_turns.refresh();
    });
    let declaration_timers = timers.clone();
    let declaration_model = state.clone();
    declare.connect_clicked(move |button| {
        if !declaration_model.borrow().may_declare() {
            return;
        }
        let Some(window) = button.root().and_downcast::<gtk::Window>() else {
            return;
        };
        let dialog = adw::AlertDialog::new(
            Some("Ausmelden?"),
            Some("Damit endet die Hand. Bei weniger als 66 Augen gewinnt der Gegner."),
        );
        dialog.add_responses(&[("cancel", "Abbrechen"), ("declare", "Ausmelden")]);
        dialog.set_default_response(Some("cancel"));
        dialog.set_close_response("cancel");
        let model = declaration_model.clone();
        let timers = declaration_timers.clone();
        dialog.connect_response(Some("declare"), move |_, _| {
            if model.borrow_mut().declare().is_ok() {
                timers.cancel();
                timers.refresh();
            }
        });
        dialog.present(Some(&window));
    });
    let model = state.clone();
    let reset_model = state.clone();
    let reset_timers = timers.clone();
    let reset_popover = popover.downgrade();
    reset_bummerls.connect_clicked(move |button| {
        let Some(window) = button.root().and_downcast::<gtk::Window>() else { return };
        if let Some(popover) = reset_popover.upgrade() { popover.popdown(); }
        let dialog = adw::AlertDialog::new(
            Some("Bummerl zurücksetzen?"),
            Some("Die Bummerl-Zähler beider Seiten werden auf null gesetzt. Der aktuelle Partiestand bleibt erhalten."),
        );
        dialog.add_responses(&[("cancel", "Abbrechen"), ("reset", "Zurücksetzen")]);
        dialog.set_default_response(Some("cancel"));
        dialog.set_close_response("cancel");
        dialog.set_response_appearance("reset", adw::ResponseAppearance::Destructive);
        let model = reset_model.clone();
        let timers = reset_timers.clone();
        dialog.connect_response(Some("reset"), move |_, _| {
            model.borrow_mut().reset_bummerls();
            timers.refresh();
        });
        dialog.present(Some(&window));
    });
    let help_area = table.downgrade();
    let help_model = state.clone();
    help.connect_toggled(move |help| {
        help_model.borrow_mut().set_show_legal(help.is_active());
        if let Some(area) = help_area.upgrade() {
            area.queue_draw();
        }
    });
    let area = table.downgrade();
    advance.connect_clicked(move |_| {
        let rules = if variant.selected() == 1 {
            Variant::Sharp
        } else {
            Variant::Soft
        };
        let rule = if schneider.is_active() {
            SchneiderRule::Double
        } else {
            SchneiderRule::Single
        };
        let mut model = model.borrow_mut();
        let result = if model.session().result().is_some() {
            model.new_match(rules, rule, choose_index)
        } else {
            model.next_hand(choose_index)
        };
        if result.is_err() {
            return;
        }
        title.set_subtitle(match model.session().variant() {
            Variant::Soft => "Weiches Schnapsen",
            Variant::Sharp => "Scharfes Schnapsen",
        });
        drop(model);
        timers.cancel();
        touch.reset();
        if let Some(area) = area.upgrade() {
            area.grab_focus();
        }
        timers.refresh();
    });
    table.set_draw_func(move |area, cr, width, height| {
        let state = state.borrow();
        let placements = layout::arrange(&state, width.into(), height.into());
        let focus = state.focus().filter(|_| area.has_visible_focus());
        if let Err(error) = paint::table(cr, &placements, focus) {
            eprintln!("Could not draw the card table: {error}");
        }
    });
    content.append(&table);
    content.append(&marriage_row);
    content.append(&notice);
    content.append(&advance);
    overlay.set_child(Some(&content));

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Schnapsen")
        .default_width(920)
        .default_height(650)
        .content(&overlay)
        .build();
    GtkWindowExt::set_focus(&window, Some(&table));
    window
}
