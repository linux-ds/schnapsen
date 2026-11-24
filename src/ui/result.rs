use adw::{gtk, prelude::*};
use schnapsen::core::{
    game::{FinishReason, HandResult, Outcome, Player},
    match_play::Match,
};

pub(super) fn dialog(result: HandResult, session: &Match) -> (adw::Dialog, gtk::Button) {
    let partie = session.result();
    let title = match (result.outcome.winner, partie.is_some()) {
        (Player::Local, false) => "Du hast gewonnen!",
        (Player::Opponent, false) => "Du hast verloren.",
        (Player::Local, true) => "Du hast die Partie gewonnen!",
        (Player::Opponent, true) => "Du hast die Partie verloren.",
    };
    let dialog = adw::Dialog::builder()
        .title(title)
        .can_close(false)
        .presentation_mode(adw::DialogPresentationMode::Floating)
        .content_width(410)
        .build();
    let frame = gtk::Box::new(gtk::Orientation::Vertical, 16);
    frame.set_margin_top(24);
    frame.set_margin_bottom(20);
    frame.set_margin_start(24);
    frame.set_margin_end(24);
    let body = gtk::Box::new(gtk::Orientation::Vertical, 12);
    body.append(&label(title, "title-1"));
    body.append(&label(reason(result.outcome.reason), "dim-label"));

    let eyes = gtk::Box::new(gtk::Orientation::Horizontal, 30);
    eyes.set_halign(gtk::Align::Center);
    eyes.append(&statistic("Deine Augen", result.score(Player::Local).eyes));
    eyes.append(&statistic(
        "Gegnerische Augen",
        result.score(Player::Opponent).eyes,
    ));
    body.append(&eyes);

    let points = match result.outcome.game_points {
        1 => "1 Spielpunkt".into(),
        count => format!("{count} Spielpunkte"),
    };
    body.append(&label(&points, "title-2"));
    if let Some(grade) = grade(result.outcome) {
        body.append(&label(grade, "dim-label"));
    }
    if let Some(partie) = partie {
        let award = if partie.awarded_bummerls == 2 {
            "Schneiderbummerl · 2 Bummerl"
        } else {
            "1 Bummerl"
        };
        body.append(&label(award, "title-3"));
        body.append(&label(
            &format!(
                "Restpunkte · Du {} : {} Computer",
                session.remaining(Player::Local),
                session.remaining(Player::Opponent),
            ),
            "dim-label",
        ));
    }

    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .propagate_natural_height(true)
        .child(&body)
        .build();
    frame.append(&scroll);
    let advance = gtk::Button::with_label(if partie.is_some() {
        "Neue Partie"
    } else {
        "Nächste Hand"
    });
    advance.add_css_class("suggested-action");
    advance.add_css_class("pill");
    advance.set_halign(gtk::Align::Center);
    advance.set_height_request(44);
    frame.append(&advance);
    dialog.set_child(Some(&frame));
    dialog.set_focus(Some(&advance));
    (dialog, advance)
}

fn label(text: &str, class: &str) -> gtk::Label {
    gtk::Label::builder()
        .label(text)
        .wrap(true)
        .justify(gtk::Justification::Center)
        .css_classes([class])
        .build()
}

fn statistic(caption: &str, value: u8) -> gtk::Box {
    let column = gtk::Box::new(gtk::Orientation::Vertical, 2);
    column.append(&label(&value.to_string(), "title-1"));
    column.append(&label(caption, "dim-label"));
    column
}

fn reason(reason: FinishReason) -> &'static str {
    match reason {
        FinishReason::Reached66 => "66 Augen erreicht",
        FinishReason::LastTrick => "Der letzte Stich hat entschieden",
        FinishReason::SuccessfulClosure => "Zudrehen gelungen",
        FinishReason::FailedClosure => "Zudrehen fehlgeschlagen",
    }
}

fn grade(outcome: Outcome) -> Option<&'static str> {
    match (outcome.reason, outcome.game_points) {
        (FinishReason::Reached66, 2) => Some("Schneider"),
        (FinishReason::Reached66, 3) => Some("Schwarz"),
        (FinishReason::SuccessfulClosure, 2) => Some("Schneider beim Zudrehen"),
        (FinishReason::SuccessfulClosure, 3) => Some("Schwarz beim Zudrehen"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn result_grade_distinguishes_wienerisch_rewards_from_penalties() {
        let outcome = |reason, game_points| Outcome {
            winner: Player::Local,
            reason,
            game_points,
            closure: None,
        };
        assert_eq!(grade(outcome(FinishReason::Reached66, 1)), None);
        assert_eq!(
            grade(outcome(FinishReason::Reached66, 2)),
            Some("Schneider")
        );
        assert_eq!(grade(outcome(FinishReason::Reached66, 3)), Some("Schwarz"));
        assert_eq!(grade(outcome(FinishReason::LastTrick, 1)), None);
        assert_eq!(
            grade(outcome(FinishReason::SuccessfulClosure, 3)),
            Some("Schwarz beim Zudrehen"),
        );
        for points in [2, 3] {
            assert_eq!(grade(outcome(FinishReason::FailedClosure, points)), None);
        }
    }
}
