use adw::prelude::*;

use crate::ui;

const APP_ID: &str = "io.github.schnapsen.Schnapsen";

pub fn run() -> adw::glib::ExitCode {
    let app = adw::Application::builder().application_id(APP_ID).build();
    app.connect_activate(|app| {
        if let Some(window) = app.active_window() {
            window.present();
        } else {
            ui::build_window(app).present();
        }
    });
    app.run()
}
