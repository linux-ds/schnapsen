use adw::{gio, glib, gtk, prelude::*};
use schnapsen::core::{
    game::Player,
    match_play::Match,
    rules::{GameVariant, MatchConfig, RuleProfile, SchneiderRule},
};
use std::{cell::RefCell, rc::Rc};

use crate::app::settings::Preferences;

mod design;
mod input;
mod layout;
mod paint;
mod preferences;
mod result;
mod review;
mod table;
mod turn;
mod window_state;

fn variant_label(variant: GameVariant) -> String {
    match variant {
        GameVariant::AustrianSchnapsen => tr!("Austrian two-player Schnapsen"),
    }
}

fn profile_label(profile: RuleProfile) -> String {
    match profile {
        RuleProfile::Soft => tr!("Soft Schnapsen"),
        RuleProfile::Sharp => tr!("Sharp Schnapsen"),
    }
}

fn choose_index(upper: usize) -> usize {
    adw::glib::random_int_range(0, upper as i32) as usize
}

fn apply_preferences(table: &mut table::Table, preferences: &Preferences) {
    table.set_design(design::CardDesign::from_ids(
        &preferences.front_id(),
        &preferences.back_id(),
    ));
    table.set_show_legal(preferences.show_legal());
    table.set_show_opponent_eyes(preferences.show_opponent_eyes());
}

pub fn build_window(app: &adw::Application, preferences: &Preferences) -> adw::ApplicationWindow {
    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let header = adw::HeaderBar::new();
    let initial_config = MatchConfig::default();
    let title = adw::WindowTitle::new("Schnapsen", &profile_label(initial_config.profile));
    header.set_title_widget(Some(&title));
    let options = gtk::Box::new(gtk::Orientation::Vertical, 8);
    options.set_margin_start(12);
    options.set_margin_end(12);
    options.set_margin_top(12);
    options.set_margin_bottom(12);
    options.append(&gtk::Label::new(Some(&tr!("For the next match"))));
    let game_variant = initial_config.variant;
    options.append(&gtk::Label::new(Some(&variant_label(game_variant))));
    let profiles = game_variant.profiles();
    let labels: Vec<_> = profiles.iter().copied().map(profile_label).collect();
    let labels: Vec<_> = labels.iter().map(String::as_str).collect();
    let profile = gtk::DropDown::from_strings(&labels);
    profile.set_selected(
        profiles
            .iter()
            .position(|&profile| profile == initial_config.profile)
            .expect("the default profile belongs to the game variant") as u32,
    );
    profile.update_property(&[gtk::accessible::Property::Label(&tr!("Rules profile"))]);
    options.append(&profile);
    let schneider = gtk::CheckButton::with_label(&tr!("Count Schneider double"));
    schneider.set_active(initial_config.schneider == SchneiderRule::Double);
    options.append(&schneider);
    options.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    let reset_bummerls = gtk::Button::with_label(&tr!("Reset Bummerl…"));
    options.append(&reset_bummerls);
    let popover = gtk::Popover::new();
    popover.set_child(Some(&options));
    let match_menu = gtk::MenuButton::builder()
        .label(tr!("Match"))
        .popover(&popover)
        .build();
    header.pack_start(&match_menu);
    let app_menu = gio::Menu::new();
    app_menu.append(Some(&tr!("Preferences")), Some("win.preferences"));
    let app_menu = gtk::MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .tooltip_text(tr!("Open menu"))
        .menu_model(&app_menu)
        .build();
    header.pack_end(&app_menu);
    content.append(&header);

    let table = gtk::DrawingArea::builder()
        .width_request(layout::MIN_WIDTH)
        .height_request(layout::MIN_HEIGHT)
        .vexpand(true)
        .focusable(true)
        .build();
    let state = Rc::new(RefCell::new(table::Table::new(Match::with_config(
        Player::Local,
        initial_config,
        choose_index,
    ))));
    apply_preferences(&mut state.borrow_mut(), preferences);
    let overlay = adw::ToastOverlay::new();
    let modal_layer = gtk::Overlay::new();
    modal_layer.set_child(Some(&overlay));
    let table_overlay = gtk::Overlay::new();
    table_overlay.set_vexpand(true);
    table_overlay.set_child(Some(&table));
    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    actions.set_halign(gtk::Align::Center);
    actions.set_valign(gtk::Align::Center);
    let play = gtk::Button::with_label(&tr!("Play card"));
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
    for button in [&play, &king, &ober] {
        button.add_css_class("suggested-action");
    }
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
    let preference_model = Rc::downgrade(&state);
    let preference_timers = Rc::downgrade(&timers);
    preferences.connect_changed(move |preferences| {
        if let (Some(model), Some(timers)) =
            (preference_model.upgrade(), preference_timers.upgrade())
        {
            apply_preferences(&mut model.borrow_mut(), preferences);
            timers.refresh_appearance();
        }
    });
    let touch = touch.downgrade();
    timers.set_advance(move |model| {
        if model.session().result().is_some() {
            let config = MatchConfig {
                variant: game_variant,
                profile: profiles[profile.selected() as usize],
                schneider: if schneider.is_active() {
                    SchneiderRule::Double
                } else {
                    SchneiderRule::Single
                },
            };
            model.new_match(config, choose_index)?;
        } else {
            model.next_hand(choose_index)?;
        }
        title.set_subtitle(&profile_label(model.session().config().profile));
        if let Some(touch) = touch.upgrade() {
            touch.reset();
        }
        Ok(())
    });
    table.set_draw_func(move |area, cr, width, height| {
        let state = state.borrow();
        let placements = layout::arrange(&state, width.into(), height.into());
        let focus = state.focus().filter(|_| area.has_visible_focus());
        if let Err(error) = paint::table(cr, &placements, focus, state.design()).and_then(|()| {
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
                state.design().front,
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
        .content(&modal_layer)
        .build();
    window_state::bind(&window, preferences);
    let preferences_action = gio::SimpleAction::new("preferences", None);
    let parent = window.downgrade();
    let preferences = preferences.clone();
    let presented = glib::WeakRef::<adw::PreferencesDialog>::new();
    let settings_timers = timers.clone();
    preferences_action.connect_activate(move |_, _| {
        if let Some(parent) = parent.upgrade() {
            if !settings_timers.input_blocked() && presented.upgrade().is_none() {
                let dialog = preferences::present(&preferences, &parent);
                presented.set(Some(&dialog));
            }
        }
    });
    window.add_action(&preferences_action);
    GtkWindowExt::set_focus(&window, Some(&table));
    timers.refresh();
    window
}
