use adw::{gtk, prelude::*};
use schnapsen::core::{
    game::{Player, Variant},
    match_play::{Match, SchneiderRule},
};
use std::{cell::RefCell, rc::Rc};

mod input;
mod layout;
mod paint;
mod result;
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
    let opponent_eyes = gtk::CheckButton::with_label("Gegnerische Augen anzeigen");
    options.append(&opponent_eyes);
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
    let overlay = adw::ToastOverlay::new();
    let modal_layer = gtk::Overlay::new();
    modal_layer.set_child(Some(&overlay));
    let table_overlay = gtk::Overlay::new();
    table_overlay.set_vexpand(true);
    table_overlay.set_child(Some(&table));
    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    actions.set_halign(gtk::Align::Center);
    actions.set_valign(gtk::Align::Center);
    let play = gtk::Button::with_label("Ausspielen");
    let king = gtk::Button::new();
    let ober = gtk::Button::new();
    let announcement = gtk::Label::new(None);
    announcement.add_css_class("title-3");
    for button in [&play, &king, &ober] {
        button.set_visible(false);
        button.set_height_request(44);
        button.set_valign(gtk::Align::Center);
        actions.append(button);
    }
    play.add_css_class("suggested-action");
    announcement.set_visible(false);
    actions.append(&announcement);
    table_overlay.add_overlay(&actions);
    table_overlay.set_measure_overlay(&actions, false);
    let controls = turn::TableControls {
        play: play.downgrade(),
        king: king.downgrade(),
        ober: ober.downgrade(),
        announcement: announcement.downgrade(),
    };
    let timers = turn::TurnTimers::new(&state, &table, &overlay, &modal_layer, controls);
    let touch = input::attach(&table, &state, &timers);
    for (button, play_ober) in [(&king, false), (&ober, true)] {
        let model = state.clone();
        let turns = timers.clone();
        let area = table.downgrade();
        button.connect_clicked(move |_| {
            if !turns.input_blocked() && model.borrow_mut().announce_selected(play_ober) {
                if let Some(area) = area.upgrade() {
                    area.grab_focus();
                }
                turns.refresh();
            }
        });
    }
    let play_model = state.clone();
    let play_timers = timers.clone();
    let play_area = table.downgrade();
    play.connect_clicked(move |_| {
        if play_timers.input_blocked() {
            return;
        }
        play_model.borrow_mut().play_selected();
        if let Some(area) = play_area.upgrade() {
            area.grab_focus();
        }
        play_timers.refresh();
    });
    let reset_timers = timers.clone();
    let reset_popover = popover.downgrade();
    reset_bummerls.connect_clicked(move |_| {
        if let Some(popover) = reset_popover.upgrade() {
            popover.popdown();
        }
        reset_timers.request_reset_bummerls();
    });
    let learning_model = state.clone();
    let learning_timers = timers.clone();
    opponent_eyes.connect_toggled(move |help| {
        learning_model
            .borrow_mut()
            .set_show_opponent_eyes(help.is_active());
        learning_timers.refresh();
    });
    let help_area = table.downgrade();
    let help_model = state.clone();
    help.connect_toggled(move |help| {
        help_model.borrow_mut().set_show_legal(help.is_active());
        if let Some(area) = help_area.upgrade() {
            area.queue_draw();
        }
    });
    let touch = touch.downgrade();
    timers.set_advance(move |model| {
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
        let result = if model.session().result().is_some() {
            model.new_match(rules, rule, choose_index)
        } else {
            model.next_hand(choose_index)
        };
        result?;
        title.set_subtitle(match model.session().variant() {
            Variant::Soft => "Weiches Schnapsen",
            Variant::Sharp => "Scharfes Schnapsen",
        });
        if let Some(touch) = touch.upgrade() {
            touch.reset();
        }
        Ok(())
    });
    table.set_draw_func(move |area, cr, width, height| {
        let state = state.borrow();
        let placements = layout::arrange(&state, width.into(), height.into());
        let focus = state.focus().filter(|_| area.has_visible_focus());
        if let Err(error) = paint::table(cr, &placements, focus).and_then(|()| {
            paint::eyes(
                cr,
                width.into(),
                height.into(),
                state.local_eyes(),
                state.opponent_eyes(),
            )?;
            paint::match_status(cr, width.into(), height.into(), state.session())?;
            paint::trump(
                cr,
                width.into(),
                height.into(),
                state.session().hand().trump_suit(),
            )
        }) {
            eprintln!("Could not draw the card table: {error}");
        }
    });
    content.append(&table_overlay);
    overlay.set_child(Some(&content));

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Schnapsen")
        .default_width(680)
        .default_height(560)
        .content(&modal_layer)
        .build();
    GtkWindowExt::set_focus(&window, Some(&table));
    timers.refresh();
    window
}
