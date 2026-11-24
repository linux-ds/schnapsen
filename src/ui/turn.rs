use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use adw::{glib, gtk, prelude::*};
use schnapsen::core::{
    cards::Suit,
    game::{
        Announcement, CloseError, Exchange, ExchangeError, FinishReason, HandResult, PlayError,
        Player, TrickReviewError,
    },
};

use super::{review, table::Table};

pub(super) struct TableControls {
    pub play: glib::WeakRef<gtk::Button>,
    pub king: glib::WeakRef<gtk::Button>,
    pub ober: glib::WeakRef<gtk::Button>,
    pub announcement: glib::WeakRef<gtk::Label>,
    pub score: glib::WeakRef<gtk::Label>,
    pub advance: glib::WeakRef<gtk::Button>,
}

impl TableControls {
    fn update(&self, table: &Table) {
        let session = table.session();
        if let Some(score) = self.score.upgrade() {
            score.set_label(&format!(
                "Restpunkte Du {} · Computer {}    Hände {}    Bummerl Du {} · Computer {}",
                session.remaining(Player::Local),
                session.remaining(Player::Opponent),
                session.completed_hands(),
                session.bummerls(Player::Local),
                session.bummerls(Player::Opponent),
            ));
        }
        if let Some(advance) = self.advance.upgrade() {
            advance.set_label(if session.result().is_some() {
                "Neue Partie"
            } else {
                "Nächste Hand"
            });
            advance.set_visible(table.result().is_some());
        }
        if let Some(play) = self.play.upgrade() {
            play.set_visible(table.can_play_selected());
        }
        let marriage = table.selected_marriage();
        if let Some(king) = self.king.upgrade() {
            if let Some(marriage) = marriage {
                let label = if marriage.may_play_ober {
                    format!("{} ansagen · König", marriage.eyes)
                } else {
                    format!("{} ansagen", marriage.eyes)
                };
                king.set_label(&label);
            }
            king.set_visible(marriage.is_some());
        }
        if let Some(ober) = self.ober.upgrade() {
            let option = marriage.filter(|marriage| marriage.may_play_ober);
            if let Some(marriage) = option {
                ober.set_label(&format!("{} ansagen · Ober", marriage.eyes));
            }
            ober.set_visible(option.is_some());
        }
        if let Some(label) = self.announcement.upgrade() {
            let event = table.announcement().filter(|_| table.result().is_none());
            if let Some(event) = event {
                let player = if event.player == Player::Local {
                    "Du"
                } else {
                    "Computer"
                };
                label.set_label(&format!("{player} · {}", event.marriage.eyes));
            }
            label.set_visible(event.is_some());
        }
    }
}

pub(super) struct TurnTimers {
    state: Rc<RefCell<Table>>,
    area: glib::WeakRef<gtk::DrawingArea>,
    notice: glib::WeakRef<gtk::Label>,
    overlay: glib::WeakRef<adw::ToastOverlay>,
    feedback: RefCell<Option<adw::Toast>>,
    pending: RefCell<Option<glib::SourceId>>,
    controls: TableControls,
    shown_effects: Cell<(Option<Announcement>, Option<Exchange>)>,
    effects_timer: RefCell<Option<glib::SourceId>>,
    modal_open: Cell<bool>,
}

impl TurnTimers {
    pub fn new(
        state: &Rc<RefCell<Table>>,
        area: &gtk::DrawingArea,
        notice: &gtk::Label,
        overlay: &adw::ToastOverlay,
        controls: TableControls,
    ) -> Rc<Self> {
        Rc::new(Self {
            state: state.clone(),
            area: area.downgrade(),
            notice: notice.downgrade(),
            overlay: overlay.downgrade(),
            feedback: RefCell::new(None),
            pending: RefCell::new(None),
            controls,
            shown_effects: Cell::new((None, None)),
            effects_timer: RefCell::new(None),
            modal_open: Cell::new(false),
        })
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
            self.show_feedback(close_message(error));
            return;
        }
        self.confirm(
            "Talon zudrehen?",
            "Ihr zieht keine Karten mehr nach. Ab jetzt gilt Farb-, Stich- und Trumpfzwang. Du musst mindestens 66 wirksame Augen erreichen, sonst gewinnt der Computer.",
            "Zudrehen",
            |timers| {
                let result = timers.state.borrow_mut().close_talon();
                let message = result.err().map_or("Talon zugedreht", close_message);
                timers.show_feedback(message);
            },
        );
    }

    pub fn request_reset_bummerls(self: &Rc<Self>) {
        self.confirm(
            "Bummerl zurücksetzen?",
            "Die Bummerl-Zähler beider Seiten werden auf null gesetzt. Der aktuelle Partiestand bleibt erhalten.",
            "Zurücksetzen",
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
        dialog.add_responses(&[("cancel", "Abbrechen"), ("confirm", action)]);
        dialog.set_close_response("cancel");
        dialog.set_default_response(Some("cancel"));
        dialog.set_response_appearance("confirm", adw::ResponseAppearance::Destructive);
        let timers = Rc::downgrade(self);
        dialog.connect_response(Some("confirm"), move |_, _| {
            if let Some(timers) = timers.upgrade() {
                apply(&timers);
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
            self.show_feedback("Warte, bis der laufende Zug abgeschlossen ist.");
            return;
        }
        let allowed = self.state.borrow().session().hand().review_tricks(owner);
        let tricks = match allowed {
            Ok(tricks) => tricks,
            Err(error) => {
                self.show_feedback(match error {
                    TrickReviewError::Forbidden => {
                        "Bei scharfem Schnapsen bleiben die Stiche verdeckt."
                    }
                    TrickReviewError::Empty => "Dieser Stapel enthält noch keine Stiche.",
                });
                return;
            }
        };
        self.present_modal(&review::dialog(owner, tricks));
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
                timers.modal_open.set(false);
                timers.refresh();
                let restored = previous_focus
                    .as_ref()
                    .and_then(glib::WeakRef::upgrade)
                    .is_some_and(|widget| widget.grab_focus());
                if !restored {
                    if let Some(area) = timers.area.upgrade() {
                        area.grab_focus();
                    }
                }
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

    pub fn refresh(self: &Rc<Self>) {
        if self.input_blocked() {
            return;
        }
        let Some(area) = self.area.upgrade() else {
            return;
        };
        area.queue_draw();
        let message = {
            let mut table = self.state.borrow_mut();
            let play_error = table.take_play_error();
            table
                .take_exchange_error()
                .map(exchange_message)
                .or_else(|| play_error.map(play_message))
        };
        if let Some(message) = message {
            self.show_feedback(&message);
        }
        let table = self.state.borrow();
        let tooltip = if table.exchange_available() {
            Some("Trumpf tauschen · T oder offene Karte antippen")
        } else if table.check_close_talon().is_ok() {
            Some("Talon zudrehen · Z oder verdeckten Stapel antippen")
        } else {
            None
        };
        area.set_tooltip_text(tooltip);
        self.controls.update(&table);
        self.update_effects((table.announcement(), table.exchange()));
        let result = table.result();
        if let Some(notice) = self.notice.upgrade() {
            if let Some(result) = result {
                let mut message = result_message(result);
                if let Some(partie) = table.session().result() {
                    let (winner, loser) = match partie.winner {
                        Player::Local => ("Du gewinnst die Partie", "Der Computer erhält"),
                        Player::Opponent => ("Der Computer gewinnt die Partie", "Du erhältst"),
                    };
                    let award = if partie.awarded_bummerls == 2 {
                        "ein Schneiderbummerl (2 Bummerl)"
                    } else {
                        "ein Bummerl"
                    };
                    message.push_str(&format!("\n{winner}. {loser} {award}."));
                }
                notice.set_label(&message);
            }
            notice.set_visible(result.is_some());
        }
        if result.is_some() || self.pending.borrow().is_some() {
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

fn exchange_message(error: ExchangeError) -> String {
    match error {
        ExchangeError::TalonNotOpen | ExchangeError::NoOpenTrump => {
            "Zum Tauschen brauchst du eine offene Trumpfkarte."
        }
        ExchangeError::TrickStarted => "Du kannst nur vor dem Ausspiel tauschen.",
        ExchangeError::CannotLead => "Du kannst nur mit Ausspielrecht tauschen.",
        ExchangeError::MissingTrumpUnder => "Wähle den Trumpf-Unter aus.",
        ExchangeError::FirstTrickNotCompleted => "Erst nach dem ersten Stich möglich.",
        ExchangeError::NotEnoughTalonCards => "Zu wenige Karten im Talon.",
    }
    .into()
}

fn result_message(result: HandResult) -> String {
    let winner = match result.outcome.winner {
        Player::Local => "Du hast gewonnen",
        Player::Opponent => "Der Computer hat gewonnen",
    };
    let reason = match result.outcome.reason {
        FinishReason::Reached66 => "66 Augen erreicht",
        FinishReason::LastTrick => "letzter Stich",
        FinishReason::SuccessfulClosure => "Zudrehen gelungen",
        FinishReason::FailedClosure => "Zudrehen fehlgeschlagen",
    };
    let points = match result.outcome.game_points {
        1 => "1 Spielpunkt".into(),
        total => format!("{total} Spielpunkte"),
    };
    format!(
        "{winner} ({reason}, {points}).\nAugen: Du {} · Computer {}",
        result.score(Player::Local).eyes,
        result.score(Player::Opponent).eyes
    )
}

fn close_message(error: CloseError) -> &'static str {
    match error {
        CloseError::TalonNotOpen => "Der Talon muss offen sein.",
        CloseError::TrickStarted => "Du kannst nur vor dem Ausspiel zudrehen.",
        CloseError::FirstTrickNotCompleted => "Erst nach dem ersten Stich möglich.",
        CloseError::CannotLead => "Du kannst nur mit Ausspielrecht zudrehen.",
        CloseError::NotEnoughTalonCards => "Zu wenige Karten im Talon.",
    }
}

fn play_message(error: PlayError) -> String {
    match error {
        PlayError::MustBeat(suit) => format!(
            "Du musst mit einer höheren {}-Karte stechen.",
            suit_name(suit)
        ),
        PlayError::MustFollow(suit) => format!("Du musst {} bedienen.", suit_name(suit)),
        PlayError::MustTrump => "Du musst Trumpf spielen.".into(),
        PlayError::OutOfTurn { .. } => "Du bist noch nicht am Zug.".into(),
        PlayError::TrickPending => "Der Stich wird noch aufgelöst.".into(),
        PlayError::CardNotInHand => "Diese Karte ist nicht mehr auf deiner Hand.".into(),
        PlayError::UnsupportedPhase(_) => "Die Hand ist bereits beendet.".into(),
    }
}

fn suit_name(suit: Suit) -> &'static str {
    match suit {
        Suit::Hearts => "Herz",
        Suit::Bells => "Schellen",
        Suit::Leaves => "Grün",
        Suit::Acorns => "Eichel",
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
            notice: glib::WeakRef::new(),
            overlay: glib::WeakRef::new(),
            feedback: RefCell::new(None),
            pending: RefCell::new(None),
            controls: TableControls {
                play: glib::WeakRef::new(),
                king: glib::WeakRef::new(),
                ober: glib::WeakRef::new(),
                announcement: glib::WeakRef::new(),
                score: glib::WeakRef::new(),
                advance: glib::WeakRef::new(),
            },
            shown_effects: Cell::new((None, None)),
            effects_timer: RefCell::new(None),
            modal_open: Cell::new(false),
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
