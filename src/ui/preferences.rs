use adw::{gtk, prelude::*};

use crate::app::settings::Preferences;

use super::{
    design::{CardDesign, BACKS, FRONTS},
    paint,
};

pub(super) fn present(
    settings: &Preferences,
    parent: &impl IsA<gtk::Widget>,
) -> adw::PreferencesDialog {
    let dialog = adw::PreferencesDialog::builder()
        .title(tr!("Preferences"))
        .search_enabled(false)
        .build();
    let page = adw::PreferencesPage::new();
    let appearance = adw::PreferencesGroup::builder()
        .title(tr!("Appearance"))
        .build();
    let design = CardDesign::from_ids(&settings.front_id(), &settings.back_id());

    let themes = gtk::StringList::new(&[]);
    for front in FRONTS {
        themes.append(&front.label());
    }
    let selected_front = FRONTS
        .iter()
        .position(|front| front.id == design.front.id)
        .expect("the selected card theme is registered");
    let front = adw::ComboRow::builder()
        .title(tr!("Card theme"))
        .model(&themes)
        .selected(selected_front as u32)
        .build();
    let preference = settings.clone();
    front.connect_selected_notify(move |row| {
        if let Some(front) = FRONTS.get(row.selected() as usize) {
            preference.set_front_id(front.id);
        }
    });
    appearance.add(&front);

    let backs = adw::ExpanderRow::builder()
        .title(tr!("Card backs"))
        .expanded(true)
        .build();
    let mut group = None;
    for back in BACKS {
        let label = back.label();
        let row = adw::ActionRow::builder().title(label.as_str()).build();
        let preview = gtk::DrawingArea::builder()
            .content_width(44)
            .content_height(62)
            .can_target(false)
            .build();
        preview.set_draw_func(move |_, cairo, width, height| {
            if let Err(error) = paint::back_preview(cairo, width.into(), height.into(), back) {
                eprintln!("Could not draw a card back preview: {error}");
            }
        });
        row.add_prefix(&preview);
        let choice = gtk::CheckButton::new();
        choice.set_valign(gtk::Align::Center);
        choice.update_property(&[gtk::accessible::Property::Label(&label)]);
        if let Some(first) = &group {
            choice.set_group(Some(first));
        } else {
            group = Some(choice.clone());
        }
        choice.set_active(back.id == design.back.id);
        let preference = settings.clone();
        choice.connect_toggled(move |button| {
            if button.is_active() {
                preference.set_back_id(back.id);
            }
        });
        row.add_suffix(&choice);
        row.set_activatable_widget(Some(&choice));
        backs.add_row(&row);
    }
    appearance.add(&backs);
    page.add(&appearance);

    let aids = adw::PreferencesGroup::builder()
        .title(tr!("Playing aids"))
        .build();
    let legal = adw::SwitchRow::builder()
        .title(tr!("Highlight playable cards"))
        .active(settings.show_legal())
        .build();
    let preference = settings.clone();
    legal.connect_active_notify(move |row| preference.set_show_legal(row.is_active()));
    aids.add(&legal);
    let eyes = adw::SwitchRow::builder()
        .title(tr!("Show opponent’s card points"))
        .active(settings.show_opponent_eyes())
        .build();
    let preference = settings.clone();
    eyes.connect_active_notify(move |row| preference.set_show_opponent_eyes(row.is_active()));
    aids.add(&eyes);
    page.add(&aids);
    dialog.add(&page);
    dialog.present(Some(parent));
    dialog
}
