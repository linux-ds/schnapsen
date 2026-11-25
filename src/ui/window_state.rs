use adw::{glib, gtk, prelude::*};

use crate::app::settings::Preferences;

use super::layout;

pub(super) fn bind(window: &adw::ApplicationWindow, preferences: &Preferences) {
    let (width, height, maximized) = preferences.window_state();
    let display = gtk::prelude::WidgetExt::display(window);
    // Wayland assigns the monitor when mapping the window. Use the largest
    // available logical geometry as a bound before that assignment.
    let bounds = display
        .monitors()
        .iter::<gtk::gdk::Monitor>()
        .filter_map(Result::ok)
        .map(|monitor| monitor.geometry())
        .max_by_key(|geometry| i64::from(geometry.width()) * i64::from(geometry.height()))
        .map(|geometry| (geometry.width(), geometry.height()));
    let (width, height) = normal_size((width, height), bounds);
    window.set_default_size(width, height);
    if maximized {
        window.maximize();
    }

    let preferences = preferences.clone();
    window.connect_close_request(move |window| {
        // GTK keeps the last normal size here even while maximized.
        let (width, height) = window.default_size();
        preferences.save_window_state((width, height, window.is_maximized()));
        glib::Propagation::Proceed
    });
}

fn normal_size(size: (i32, i32), bounds: Option<(i32, i32)>) -> (i32, i32) {
    let (width, height) = size;
    let (max_width, max_height) = bounds.unwrap_or((i32::MAX, i32::MAX));
    let width = if width < layout::MIN_WIDTH {
        680
    } else {
        width
    };
    let height = if height < layout::MIN_HEIGHT {
        560
    } else {
        height
    };
    // The existing GTK size requests still apply on unusually small displays.
    (
        width.min(max_width.max(layout::MIN_WIDTH)),
        height.min(max_height.max(layout::MIN_HEIGHT)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_size_keeps_valid_logical_dimensions_and_replaces_invalid_values() {
        assert_eq!(normal_size((1080, 760), Some((1920, 1080))), (1080, 760));
        assert_eq!(normal_size((-1, 0), None), (680, 560));
        assert_eq!(normal_size((900, 1), None), (900, 560));
    }

    #[test]
    fn window_size_fits_changed_monitor_geometry() {
        assert_eq!(
            normal_size((i32::MAX, 1440), Some((1280, 720))),
            (1280, 720)
        );
        assert_eq!(
            normal_size((1920, 1080), Some((480, 360))),
            (layout::MIN_WIDTH, layout::MIN_HEIGHT)
        );
    }
}
