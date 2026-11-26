use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use adw::{gio, glib, gtk, prelude::*};
use schnapsen::core::{
    cards::Rank,
    game::{
        Announcement, CloseError, Exchange, ExchangeError, PlayError, Player, TrickReviewError,
    },
    match_play::DealError,
};

use crate::app::{
    save::{SaveError, SaveStore},
    settings::Preferences,
};

use super::{design::FrontTheme, result, review, table::Table, window_state};

type AdvanceHand = Box<dyn Fn(&mut Table) -> Result<(), DealError>>;

pub(super) struct TableControls {
    pub play: glib::WeakRef<gtk::Button>,
    pub king: glib::WeakRef<gtk::Button>,
    pub ober: glib::WeakRef<gtk::Button>,
    pub announcement: glib::WeakRef<gtk::Label>,
}

impl TableControls {
    fn update(&self, table: &Table) {
        if let Some(play) = self.play.upgrade() {
            play.set_visible(table.can_play_selected());
        }
        let marriage = table.selected_marriage();
        let theme = table.design().front;
        if let Some(king) = self.king.upgrade() {
            if let Some(marriage) = marriage {
                let label = if marriage.may_play_ober {
                    tr!(
                        "Declare {points} · {rank}",
                        points = marriage.eyes,
                        rank = theme.rank_name(Rank::King)
                    )
                } else {
                    tr!("Declare {points}", points = marriage.eyes)
                };
                king.set_label(&label);
            }
            king.set_visible(marriage.is_some());
        }
        if let Some(ober) = self.ober.upgrade() {
            let option = marriage.filter(|marriage| marriage.may_play_ober);
            if let Some(marriage) = option {
                ober.set_label(&tr!(
                    "Declare {points} · {rank}",
                    points = marriage.eyes,
                    rank = theme.rank_name(Rank::Ober)
                ));
            }
            ober.set_visible(option.is_some());
        }
        if let Some(label) = self.announcement.upgrade() {
            let event = table.announcement().filter(|_| table.result().is_none());
            if let Some(event) = event {
                let player = if event.player == Player::Local {
                    tr!("You")
                } else {
                    tr!("Computer")
                };
                label.set_label(&tr!(
                    "{player} · {points}",
                    player = player,
                    points = event.marriage.eyes
                ));
            }
            label.set_visible(event.is_some());
        }
    }
}

pub(super) struct TurnTimers {
    state: Rc<RefCell<Table>>,
    area: glib::WeakRef<gtk::DrawingArea>,
    advance: RefCell<Option<AdvanceHand>>,
    overlay: glib::WeakRef<adw::ToastOverlay>,
    review_host: glib::WeakRef<gtk::Overlay>,
    feedback: RefCell<Option<adw::Toast>>,
    pending: RefCell<Option<glib::SourceId>>,
    controls: TableControls,
    shown_effects: Cell<(Option<Announcement>, Option<Exchange>)>,
    effects_timer: RefCell<Option<glib::SourceId>>,
    modal_open: Cell<bool>,
    store: Option<RefCell<SaveStore>>,
    save_failed: Cell<bool>,
}

impl TurnTimers {
    pub fn new(
        state: &Rc<RefCell<Table>>,
        area: &gtk::DrawingArea,
        overlay: &adw::ToastOverlay,
        review_host: &gtk::Overlay,
        controls: TableControls,
        store: SaveStore,
    ) -> Rc<Self> {
        Rc::new(Self {
            state: state.clone(),
            area: area.downgrade(),
            advance: RefCell::new(None),
            overlay: overlay.downgrade(),
            review_host: review_host.downgrade(),
            feedback: RefCell::new(None),
            pending: RefCell::new(None),
            controls,
            shown_effects: Cell::new((None, None)),
            effects_timer: RefCell::new(None),
            modal_open: Cell::new(false),
            store: Some(RefCell::new(store)),
            save_failed: Cell::new(false),
        })
    }

    pub fn bind_save(
        self: &Rc<Self>,
        window: &adw::ApplicationWindow,
        load_error: Option<SaveError>,
        preferences: &Preferences,
    ) {
        self.modal_open.set(load_error.is_some());
        let timers = self.clone();
        let active_alert = glib::WeakRef::<adw::AlertDialog>::new();
        let preferences = preferences.clone();
        let try_close = Rc::new(move |window: &adw::ApplicationWindow| {
            window_state::save(window, &preferences);
            if active_alert.upgrade().is_some() {
                return glib::Propagation::Stop;
            }
            match timers.persist() {
                Ok(()) | Err(SaveError::Blocked) => glib::Propagation::Proceed,
                Err(error) => {
                    eprintln!("Could not save the match on close: {error}");
                    let alert = adw::AlertDialog::new(
                        Some(&tr!("Could not save the match")),
                        Some(&tr!("Any unsaved progress will be lost. Close anyway?")),
                    );
                    alert.add_responses(&[
                        ("cancel", &tr!("Keep playing")),
                        ("quit", &tr!("Close without saving")),
                    ]);
                    alert.set_close_response("cancel");
                    alert.set_default_response(Some("cancel"));
                    alert.set_response_appearance("quit", adw::ResponseAppearance::Destructive);
                    let window_ref = window.downgrade();
                    alert.connect_response(Some("quit"), move |_, _| {
                        if let Some(window) = window_ref.upgrade() {
                            window.destroy();
                        }
                    });
                    active_alert.set(Some(&alert));
                    if timers.input_blocked() {
                        alert.present(Some(window));
                    } else {
                        timers.present_modal(&alert);
                    }
                    glib::Propagation::Stop
                }
            }
        });
        let close_request = try_close.clone();
        window.connect_close_request(move |window| close_request(window));

        // A non-dismissible result dialog intercepts native window closing.
        // The application quit action still saves and exits without advancing.
        let quit = gio::SimpleAction::new("quit", None);
        let window_ref = window.downgrade();
        quit.connect_activate(move |_, _| {
            if let Some(window) = window_ref.upgrade() {
                if try_close(&window) == glib::Propagation::Proceed {
                    window.destroy();
                }
            }
        });
        window.add_action(&quit);

        let weak = Rc::downgrade(self);
        let window = window.downgrade();
        glib::idle_add_local_once(move || {
            if let (Some(timers), Some(window)) = (weak.upgrade(), window.upgrade()) {
                if let Some(error) = load_error {
                    eprintln!("Could not load the match: {error}");
                    let message = match error {
                        SaveError::UnsupportedVersion(_) => {
                            tr!("This saved match uses an unsupported format.")
                        }
                        SaveError::Corrupt => tr!("This saved match is damaged or inconsistent."),
                        _ => tr!("The saved match could not be read."),
                    };
                    timers.request_save_recovery(&window, &message);
                } else {
                    timers.refresh();
                }
            }
        });
    }

    fn request_save_recovery(self: &Rc<Self>, window: &adw::ApplicationWindow, message: &str) {
        let alert = adw::AlertDialog::new(Some(&tr!("Could not restore the match")), Some(message));
        alert.add_responses(&[
            ("quit", &tr!("Keep the file and quit")),
            ("restart", &tr!("Archive the file and start over")),
        ]);
        alert.set_close_response("quit");
        alert.set_default_response(Some("quit"));
        let weak = Rc::downgrade(self);
        let window_ref = window.downgrade();
        alert.connect_response(None, move |_, response| {
            let (Some(timers), Some(window)) = (weak.upgrade(), window_ref.upgrade()) else {
                return;
            };
            if response != "restart" {
                let _ = gtk::prelude::WidgetExt::activate_action(&window, "win.quit", None);
                return;
            }
            let archived = timers
                .store
                .as_ref()
                .unwrap()
                .borrow_mut()
                .archive_invalid();
            glib::idle_add_local_once(move || match archived {
                Ok(()) => {
                    timers.modal_open.set(false);
                    timers.refresh();
                }
                Err(error) => {
                    eprintln!("Could not archive the saved match: {error}");
                    timers.request_save_recovery(
                        &window,
                        &tr!("The saved file could not be archived. You can retry or quit."),
                    );
                }
            });
        });
        alert.present(Some(window));
    }

    fn persist(&self) -> Result<(), SaveError> {
        if let Some(store) = &self.store {
            let state = self.state.borrow();
            store
                .borrow_mut()
                .save(state.session(), state.selection())?;
        }
        Ok(())
    }

    fn autosave(&self) {
        match self.persist() {
            Ok(()) => self.save_failed.set(false),
            Err(SaveError::Blocked) => {}
            Err(error) if !self.save_failed.replace(true) => {
                eprintln!("Could not autosave the match: {error}");
                if let Some(overlay) = self.overlay.upgrade() {
                    overlay.add_toast(adw::Toast::new(&tr!("Could not save the match")));
                }
            }
            Err(_) => {}
        }
    }

    pub fn set_advance(&self, advance: impl Fn(&mut Table) -> Result<(), DealError> + 'static) {
        self.advance.replace(Some(Box::new(advance)));
    }

    pub fn cancel(&self) {
        if let Some(source) = self.pending.take() {
            source.remove();
        }
        self.dismiss_feedback();
        self.cancel_effects_timer();
        self.shown_effects.set((None, None));
    }

    fn cancel_effects_timer(&self) {
        if let Some(source) = self.effects_timer.take() {
            source.remove();
        }
    }

    fn update_effects(self: &Rc<Self>, effects: (Option<Announcement>, Option<Exchange>)) {
        if self.shown_effects.replace(effects) == effects {
            return;
        }
        self.cancel_effects_timer();
        if effects != (None, None) {
            let timers = Rc::downgrade(self);
            let source = glib::timeout_add_local_once(Duration::from_millis(1400), move || {
                if let Some(timers) = timers.upgrade() {
                    timers.effects_timer.take();
                    timers.state.borrow_mut().hide_card_effects();
                    timers.refresh();
                }
            });
            self.effects_timer.replace(Some(source));
        }
    }

    fn dismiss_feedback(&self) {
        if let Some(toast) = self.feedback.take() {
            toast.dismiss();
        }
    }

    fn show_feedback(&self, message: &str) {
        self.dismiss_feedback();
        if let Some(overlay) = self.overlay.upgrade() {
            let toast = adw::Toast::new(message);
            toast.set_timeout(2);
            overlay.add_toast(toast.clone());
            self.feedback.replace(Some(toast));
        }
    }

    pub fn request_close(self: &Rc<Self>) {
        if self.input_blocked() {
            return;
        }
        if let Err(error) = self.state.borrow().check_close_talon() {
            self.show_feedback(&close_message(error));
            return;
        }
        self.confirm(
            &tr!("Close the stock?"),
            &tr!("Neither player draws any more cards. From now on, you must follow suit and win the trick if possible, or play trump if you cannot follow suit. You must reach at least 66 card points, or the computer wins."),
            &tr!("Close Stock"),
            |timers| {
                let result = timers.state.borrow_mut().close_talon();
                let message = result.err().map_or_else(|| tr!("Stock closed"), close_message);
                timers.show_feedback(&message);
            },
        );
    }

    pub fn request_reset_bummerls(self: &Rc<Self>) {
        self.confirm(
            &tr!("Reset the Bummerl counters?"),
            &tr!("Both Bummerl counters will be set to zero. The current match score stays the same."),
            &tr!("Reset"),
            |timers| timers.state.borrow_mut().reset_bummerls(),
        );
    }

    fn confirm(
        self: &Rc<Self>,
        title: &str,
        body: &str,
        action: &str,
        apply: impl Fn(&Self) + 'static,
    ) {
        let dialog = adw::AlertDialog::new(Some(title), Some(body));
        dialog.set_presentation_mode(adw::DialogPresentationMode::Floating);
        dialog.add_responses(&[("cancel", &tr!("Cancel")), ("confirm", action)]);
        dialog.set_close_response("cancel");
        dialog.set_default_response(Some("cancel"));
        dialog.set_response_appearance("confirm", adw::ResponseAppearance::Destructive);
        let timers = Rc::downgrade(self);
        dialog.connect_response(Some("confirm"), move |_, _| {
            if let Some(timers) = timers.upgrade() {
                apply(&timers);
                timers.autosave();
            }
        });
        self.present_modal(&dialog);
    }

    pub fn input_blocked(&self) -> bool {
        self.modal_open.get()
    }

    pub fn request_review(self: &Rc<Self>, owner: Player) {
        if self.input_blocked() {
            return;
        }
        if self.pending.borrow().is_some() {
            self.show_feedback(&tr!("Wait for the current turn to finish."));
            return;
        }
        let allowed = self.state.borrow().session().hand().review_tricks(owner);
        let tricks = match allowed {
            Ok(tricks) => tricks,
            Err(error) => {
                self.show_feedback(&match error {
                    TrickReviewError::Forbidden => {
                        tr!("Tricks stay face down in Sharp Schnapsen.")
                    }
                    TrickReviewError::Empty => tr!("This pile has no tricks yet."),
                });
                return;
            }
        };
        let Some(host) = self.review_host.upgrade() else {
            return;
        };
        let Some(window) = host.root().and_downcast::<gtk::Window>() else {
            return;
        };
        let Some(background) = host.child() else {
            return;
        };
        let previous_focus = GtkWindowExt::focus(&window).map(|widget| widget.downgrade());
        self.modal_open.set(true);
        self.state.borrow_mut().cancel_mouse_click();
        self.dismiss_feedback();

        let weak = Rc::downgrade(self);
        let layer_ref = Rc::new(glib::WeakRef::<gtk::Overlay>::new());
        let close_layer = layer_ref.clone();
        let theme = self.state.borrow().design().front;
        let (layer, close) = review::layer(owner, tricks, theme, move || {
            if let (Some(timers), Some(layer)) = (weak.upgrade(), close_layer.upgrade()) {
                let Some(host) = timers.review_host.upgrade() else {
                    return;
                };
                host.remove_overlay(&layer);
                if let Some(background) = host.child() {
                    background.set_sensitive(true);
                }
                timers.restore_focus(previous_focus.as_ref());
            }
        });
        layer_ref.set(Some(&layer));
        background.set_sensitive(false);
        host.add_overlay(&layer);
        host.set_measure_overlay(&layer, false);
        close.grab_focus();
    }

    fn restore_focus(self: &Rc<Self>, previous_focus: Option<&glib::WeakRef<gtk::Widget>>) {
        self.modal_open.set(false);
        self.refresh();
        let restored = previous_focus
            .and_then(glib::WeakRef::upgrade)
            .is_some_and(|widget| widget.grab_focus());
        if !restored {
            if let Some(area) = self.area.upgrade() {
                area.grab_focus();
            }
        }
    }

    fn present_modal(self: &Rc<Self>, dialog: &impl IsA<adw::Dialog>) {
        if self.input_blocked() {
            return;
        }
        let Some(area) = self.area.upgrade() else {
            return;
        };
        let Some(window) = area.root().and_downcast::<gtk::Window>() else {
            return;
        };
        let previous_focus = GtkWindowExt::focus(&window).map(|widget| widget.downgrade());
        let weak = Rc::downgrade(self);
        dialog.connect_closed(move |_| {
            if let Some(timers) = weak.upgrade() {
                timers.restore_focus(previous_focus.as_ref());
            }
        });
        self.modal_open.set(true);
        self.state.borrow_mut().cancel_mouse_click();
        if let Some(source) = self.pending.take() {
            source.remove();
        }
        self.dismiss_feedback();
        dialog.present(Some(&window));
    }

    pub fn refresh_appearance(self: &Rc<Self>) {
        self.dismiss_feedback();
        if let Some(area) = self.area.upgrade() {
            area.queue_draw();
        }
        self.refresh();
    }

    pub fn refresh(self: &Rc<Self>) {
        if self.input_blocked() {
            return;
        }
        self.autosave();
        let Some(area) = self.area.upgrade() else {
            return;
        };
        area.queue_draw();
        let message = {
            let mut table = self.state.borrow_mut();
            let play_error = table.take_play_error();
            let theme = table.design().front;
            table
                .take_exchange_error()
                .map(|error| exchange_message(error, theme))
                .or_else(|| play_error.map(|error| play_message(error, theme)))
        };
        if let Some(message) = message {
            self.show_feedback(&message);
        }
        let table = self.state.borrow();
        let session = table.session();
        let status = tr!(
            "Top: {opponent_points}, {opponent_bummerls}. {hands}. Bottom: {local_points}, {local_bummerls}. Trump: {suit}.",
            opponent_points = remaining_points(session.remaining(Player::Opponent)),
            opponent_bummerls = bummerls(session.bummerls(Player::Opponent)),
            hands = trn!("{count} hand played", "{count} hands played", session.completed_hands()),
            local_points = remaining_points(session.remaining(Player::Local)),
            local_bummerls = bummerls(session.bummerls(Player::Local)),
            suit = table.design().front.suit_name(session.hand().trump_suit()),
        );
        area.update_property(&[gtk::accessible::Property::Description(&status)]);
        let tooltip = if table.exchange_available() {
            Some(tr!("Exchange trump · Press T or tap the face-up card"))
        } else if table.check_close_talon().is_ok() {
            Some(tr!("Close the stock · Press Z or tap the face-down pile"))
        } else {
            None
        };
        area.set_tooltip_text(tooltip.as_deref());
        self.controls.update(&table);
        self.update_effects((table.announcement(), table.exchange()));
        if let Some(hand) = table.result() {
            let (dialog, advance) = result::dialog(hand, session);
            drop(table);
            let weak = Rc::downgrade(self);
            let dialog_ref = dialog.downgrade();
            advance.connect_clicked(move |button| {
                let (Some(timers), Some(dialog)) = (weak.upgrade(), dialog_ref.upgrade()) else {
                    return;
                };
                button.set_sensitive(false);
                let advanced = timers
                    .advance
                    .borrow()
                    .as_ref()
                    .is_some_and(|advance| advance(&mut timers.state.borrow_mut()).is_ok());
                if advanced {
                    timers.autosave();
                    timers.cancel();
                    dialog.force_close();
                } else {
                    button.set_sensitive(true);
                }
            });
            self.present_modal(&dialog);
            return;
        }
        if self.pending.borrow().is_some() {
            return;
        }

        let (delay, step): (u64, fn(&mut Table)) = if table.view().played.len() == 2 {
            (700, Table::resolve_trick)
        } else if table.opponent_to_play() {
            (350, Table::play_opponent)
        } else {
            return;
        };
        let timers = Rc::downgrade(self);
        let source = glib::timeout_add_local_once(Duration::from_millis(delay), move || {
            if let Some(timers) = timers.upgrade() {
                timers.pending.take();
                step(&mut timers.state.borrow_mut());
                timers.refresh();
            }
        });
        self.pending.replace(Some(source));
    }
}

fn remaining_points(count: u8) -> String {
    trn!("{count} point remaining", "{count} points remaining", count)
}

fn bummerls(count: u32) -> String {
    trn!("{count} Bummerl", "{count} Bummerls", count)
}

fn exchange_message(error: ExchangeError, theme: &FrontTheme) -> String {
    match error {
        ExchangeError::TalonNotOpen | ExchangeError::NoOpenTrump => {
            tr!("You need a face-up trump card to exchange.")
        }
        ExchangeError::TrickStarted => tr!("Exchange the trump card before leading to the trick."),
        ExchangeError::CannotLead => {
            tr!("You can exchange the trump card only when it is your lead.")
        }
        ExchangeError::MissingTrumpUnder => {
            tr!(
                "Select the trump {rank}.",
                rank = theme.rank_name(Rank::Unter)
            )
        }
        ExchangeError::FirstTrickNotCompleted => tr!("Available after the first trick."),
        ExchangeError::NotEnoughTalonCards => tr!("Too few cards in the stock."),
    }
}

fn close_message(error: CloseError) -> String {
    match error {
        CloseError::TalonNotOpen => tr!("The stock must be open."),
        CloseError::TrickStarted => tr!("Close the stock before leading to the trick."),
        CloseError::FirstTrickNotCompleted => tr!("Available after the first trick."),
        CloseError::CannotLead => tr!("You can close the stock only when it is your lead."),
        CloseError::NotEnoughTalonCards => tr!("Too few cards in the stock."),
    }
}

fn play_message(error: PlayError, theme: &FrontTheme) -> String {
    match error {
        PlayError::MustBeat(suit) => tr!(
            "You must play a higher card of the same suit ({suit}).",
            suit = theme.suit_name(suit)
        ),
        PlayError::MustFollow(suit) => tr!(
            "You must follow suit: {suit}.",
            suit = theme.suit_name(suit)
        ),
        PlayError::MustTrump => tr!("You must play a trump card."),
        PlayError::OutOfTurn { .. } => tr!("It is not your turn yet."),
        PlayError::TrickPending => tr!("The current trick is still being resolved."),
        PlayError::CardNotInHand => tr!("This card is no longer in your hand."),
        PlayError::UnsupportedPhase(_) => tr!("The hand is already over."),
    }
}

impl Drop for TurnTimers {
    fn drop(&mut self) {
        self.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use schnapsen::core::{
        game::Variant,
        match_play::{Match, SchneiderRule},
    };

    #[test]
    fn play_feedback_uses_the_suit_and_rank_of_the_active_theme() {
        use crate::ui::design::CardDesign;
        use schnapsen::core::cards::Suit;

        let german = CardDesign::from_ids("double-german", "legacy").front;
        let french = CardDesign::from_ids("french", "legacy").front;
        let must_follow = PlayError::MustFollow(Suit::Leaves);

        assert_eq!(
            play_message(must_follow, german),
            tr!("You must follow suit: {suit}.", suit = tr!("Leaves")),
        );
        assert_eq!(
            play_message(must_follow, french),
            tr!("You must follow suit: {suit}.", suit = tr!("Spades")),
        );
        assert_eq!(
            exchange_message(ExchangeError::MissingTrumpUnder, french),
            tr!("Select the trump {rank}.", rank = tr!("Jack")),
        );
    }

    #[test]
    fn cancelling_turn_timers_discards_queued_steps_and_card_effects() {
        let state = Rc::new(RefCell::new(Table::new(Match::new(
            Player::Opponent,
            Variant::Soft,
            SchneiderRule::Double,
            |upper| upper - 1,
        ))));
        let timers = TurnTimers {
            state: state.clone(),
            area: glib::WeakRef::new(),
            advance: RefCell::new(None),
            overlay: glib::WeakRef::new(),
            review_host: glib::WeakRef::new(),
            feedback: RefCell::new(None),
            pending: RefCell::new(None),
            controls: TableControls {
                play: glib::WeakRef::new(),
                king: glib::WeakRef::new(),
                ober: glib::WeakRef::new(),
                announcement: glib::WeakRef::new(),
            },
            shown_effects: Cell::new((None, None)),
            effects_timer: RefCell::new(None),
            modal_open: Cell::new(false),
            store: None,
            save_failed: Cell::new(false),
        };
        let context = glib::MainContext::default();
        let _owner = context.acquire().unwrap();
        let old_state = state.clone();
        let step = glib::idle_add_local_once(move || old_state.borrow_mut().play_opponent());
        let effect_expired = Rc::new(Cell::new(false));
        let expired = effect_expired.clone();
        let effect = glib::idle_add_local_once(move || expired.set(true));
        assert!(context.find_source_by_id(&step).is_some());
        assert!(context.find_source_by_id(&effect).is_some());
        timers.pending.replace(Some(step));
        timers.effects_timer.replace(Some(effect));

        timers.cancel();
        timers.cancel();
        let barrier_ran = Rc::new(Cell::new(false));
        let barrier = barrier_ran.clone();
        glib::idle_add_local_once(move || barrier.set(true));
        while !barrier_ran.get() {
            context.iteration(false);
        }
        assert!(state.borrow().opponent_to_play());
        assert!(state.borrow().view().played.is_empty());
        assert!(!effect_expired.get());
    }
}
