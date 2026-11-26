use adw::prelude::*;

use crate::ui;

pub(crate) mod save;
#[cfg(test)]
mod save_scenarios;
pub(crate) mod settings;

const APP_ID: &str = "io.github.schnapsen.Schnapsen";

pub fn run() -> adw::glib::ExitCode {
    let app = adw::Application::builder().application_id(APP_ID).build();
    let preferences = settings::Preferences::new();
    app.connect_activate(move |app| {
        if let Some(window) = app.active_window() {
            window.present();
        } else {
            ui::build_window(app, &preferences).present();
        }
    });
    app.run()
}
