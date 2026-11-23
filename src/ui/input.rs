use adw::{gdk, glib, gtk, prelude::*};
use schnapsen::core::cards::Card;
use schnapsen::core::game::Player;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use super::{layout, table::Table, turn::TurnTimers};

#[derive(Clone, Copy)]
enum Target {
    Hand(Card),
    OpenTrump,
    Talon,
    Tricks(Player),
}

pub(super) fn attach(
    area: &gtk::DrawingArea,
    state: &Rc<RefCell<Table>>,
    timers: &Rc<TurnTimers>,
) -> gtk::GestureDrag {
    let keys = gtk::EventControllerKey::new();
    let model = state.clone();
    let turns = timers.clone();
    keys.connect_key_pressed(move |_, key, _, modifiers| {
        if turns.review_open() {
            return glib::Propagation::Stop;
        }
        if modifiers.intersects(
            gdk::ModifierType::CONTROL_MASK
                | gdk::ModifierType::ALT_MASK
                | gdk::ModifierType::SUPER_MASK,
        ) {
            return glib::Propagation::Proceed;
        }
        let mut table = model.borrow_mut();
        match key {
            gdk::Key::Left => table.move_focus(-1),
            gdk::Key::Right => table.move_focus(1),
            gdk::Key::Return | gdk::Key::KP_Enter => {
                if let Some(card) = table.focus() {
                    table.toggle(card);
                }
            }
            gdk::Key::Escape => table.clear_selection(),
            gdk::Key::t | gdk::Key::T => table.exchange_selected(),
            gdk::Key::s | gdk::Key::S => {
                drop(table);
                let owner = if modifiers.contains(gdk::ModifierType::SHIFT_MASK) {
                    Player::Opponent
                } else {
                    Player::Local
                };
                turns.request_review(owner);
                return glib::Propagation::Stop;
            }
            gdk::Key::z | gdk::Key::Z => {
                drop(table);
                turns.request_close();
                return glib::Propagation::Stop;
            }
            gdk::Key::space => {
                table.play_selected();
            }
            _ => return glib::Propagation::Proceed,
        }
        drop(table);
        turns.refresh();
        glib::Propagation::Stop
    });
    area.add_controller(keys);
    area.connect_has_focus_notify(|widget| widget.queue_draw());

    let mouse = gtk::GestureClick::new();
    mouse.set_button(0);
    let model = state.clone();
    let turns = timers.clone();
    mouse.connect_released(move |gesture, _, x, y| {
        if turns.review_open() {
            return;
        }
        let Some(event) = gesture.current_event() else {
            return;
        };
        if event
            .device()
            .is_some_and(|device| device.source() == gdk::InputSource::Touchscreen)
        {
            return;
        }
        let Ok(event) = event.downcast::<gdk::ButtonEvent>() else {
            return;
        };
        let Some(widget) = gesture.widget() else {
            return;
        };
        let mut table = model.borrow_mut();
        let Some((target, _)) = target_at(&widget, &table, x, y) else {
            return;
        };
        match (event.button(), target) {
            (gdk::BUTTON_PRIMARY, Target::Hand(card)) => table.toggle(card),
            (gdk::BUTTON_PRIMARY, Target::OpenTrump) => table.exchange_selected(),
            (gdk::BUTTON_PRIMARY, Target::Talon) => {
                drop(table);
                turns.request_close();
                return;
            }
            (gdk::BUTTON_PRIMARY, Target::Tricks(owner)) => {
                drop(table);
                turns.request_review(owner);
                return;
            }
            (gdk::BUTTON_SECONDARY, Target::Hand(card)) if table.selected_card() == Some(card) => {
                table.play_selected();
            }
            _ => return,
        }
        drop(table);
        widget.grab_focus();
        turns.refresh();
    });
    area.add_controller(mouse);

    let touch = gtk::GestureDrag::new();
    touch.set_touch_only(true);
    let start = Rc::new(Cell::new(None));
    let pending = start.clone();
    let model = state.clone();
    touch.connect_drag_begin(move |gesture, x, y| {
        pending.set(
            gesture
                .widget()
                .and_then(|widget| target_at(&widget, &model.borrow(), x, y)),
        );
    });
    let pending = start.clone();
    touch.connect_cancel(move |_, _| pending.set(None));
    let model = state.clone();
    let turns = timers.clone();
    touch.connect_drag_end(move |gesture, dx, dy| {
        let Some((target, height)) = start.take() else {
            return;
        };
        if turns.review_open() {
            return;
        }
        if !gesture
            .current_event()
            .is_some_and(|event| event.event_type() == gdk::EventType::TouchEnd)
        {
            return;
        }
        let Some(widget) = gesture.widget() else {
            return;
        };
        {
            let mut table = model.borrow_mut();
            match target {
                Target::Hand(card) if is_tap(dx, dy, height) => table.toggle(card),
                Target::Hand(card) if is_upward_swipe(dx, dy, height) => {
                    table.swipe(card);
                }
                Target::OpenTrump if is_tap(dx, dy, height) => table.exchange_selected(),
                Target::Talon if is_tap(dx, dy, height) => {
                    drop(table);
                    turns.request_close();
                    return;
                }
                Target::Tricks(owner) if is_tap(dx, dy, height) => {
                    drop(table);
                    turns.request_review(owner);
                    return;
                }
                _ => return,
            }
        }
        widget.grab_focus();
        turns.refresh();
    });
    area.add_controller(touch.clone());
    touch
}

fn target_at(widget: &gtk::Widget, state: &Table, x: f64, y: f64) -> Option<(Target, f64)> {
    let placements = layout::arrange(state, widget.width().into(), widget.height().into());
    layout::hand_at(&placements, x, y)
        .map(|(card, height)| (Target::Hand(card), height))
        .or_else(|| {
            layout::open_trump_at(&placements, x, y).map(|height| (Target::OpenTrump, height))
        })
        .or_else(|| layout::talon_at(&placements, x, y).map(|height| (Target::Talon, height)))
        .or_else(|| {
            layout::trick_at(&placements, x, y)
                .map(|(owner, height)| (Target::Tricks(owner), height))
        })
}

fn is_tap(dx: f64, dy: f64, height: f64) -> bool {
    dx.hypot(dy) <= 0.08 * height
}

fn is_upward_swipe(dx: f64, dy: f64, height: f64) -> bool {
    dy <= -0.32 * height && dx.abs() <= -dy
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn touch_distinguishes_taps_and_upward_swipes() {
        assert!(is_tap(3.0, -4.0, 100.0));
        assert!(!is_tap(10.0, -40.0, 100.0));
        assert!(!is_tap(0.0, -24.0, 100.0));
        assert!(!is_tap(70.0, -40.0, 100.0));
        assert!(!is_tap(0.0, 40.0, 100.0));
        assert!(is_upward_swipe(10.0, -40.0, 100.0));
        assert!(!is_upward_swipe(0.0, -24.0, 100.0));
        assert!(!is_upward_swipe(70.0, -40.0, 100.0));
    }
}
