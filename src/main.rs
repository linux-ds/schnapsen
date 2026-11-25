#[macro_use]
mod i18n;

mod app;
mod ui;

fn main() -> adw::glib::ExitCode {
    i18n::init();
    app::run()
}
