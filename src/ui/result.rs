use adw::{gtk, prelude::*};
use schnapsen::core::{
    game::{FinishReason, HandResult, Outcome, Player},
    match_play::Match,
};

use super::{design::FrontTheme, review};

pub(super) fn dialog(
    result: HandResult,
    session: &Match,
    theme: &'static FrontTheme,
) -> (adw::Dialog, gtk::Button) {
    let partie = session.result();
    let title = match (result.outcome.winner, partie.is_some()) {
        (Player::Local, false) => tr!("You won!"),
        (Player::Opponent, false) => tr!("You lost."),
        (Player::Local, true) => tr!("You won the match!"),
        (Player::Opponent, true) => tr!("You lost the match."),
    };
    let dialog = adw::Dialog::builder()
        .title(title.as_str())
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
    body.append(&label(&title, "title-1"));
    body.append(&label(&reason(result.outcome.reason), "dim-label"));

    let profile = session.config().profile;
    if profile.manual_claim() && result.outcome.reason == FinishReason::SuccessfulClosure {
        let basis = if profile.scores_closure_at_end() {
            tr!("Stock closing scored using the opponent’s points at the end of the hand.")
        } else {
            tr!("Stock closing scored using the opponent’s points at the time of closing.")
        };
        body.append(&label(&basis, "dim-label"));
    }

    let eyes = gtk::Box::new(gtk::Orientation::Horizontal, 30);
    eyes.set_halign(gtk::Align::Center);
    eyes.append(&statistic(
        &tr!("Your card points"),
        result.score(Player::Local).eyes,
    ));
    eyes.append(&statistic(
        &tr!("Opponent’s card points"),
        result.score(Player::Opponent).eyes,
    ));
    body.append(&eyes);

    let points = trn!(
        "{count} game point",
        "{count} game points",
        result.outcome.game_points
    );
    body.append(&label(&points, "title-2"));
    if let Some(grade) = grade(result.outcome) {
        body.append(&label(&grade, "dim-label"));
    }
    if let Some(partie) = partie {
        let award = if partie.awarded_bummerls == 2 {
            tr!(
                "Schneider · {count} Bummerls",
                count = partie.awarded_bummerls
            )
        } else {
            trn!(
                "{count} Bummerl",
                "{count} Bummerls",
                partie.awarded_bummerls
            )
        };
        body.append(&label(&award, "title-3"));
        body.append(&label(
            &tr!(
                "Points remaining · You {local} : {opponent} Computer",
                local = session.remaining(Player::Local),
                opponent = session.remaining(Player::Opponent)
            ),
            "dim-label",
        ));
    }

    if profile.allows_finished_trick_review() {
        let reviews = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        reviews.set_halign(gtk::Align::Center);
        for (owner, caption) in [
            (Player::Local, tr!("Your tricks")),
            (Player::Opponent, tr!("Opponent’s tricks")),
        ] {
            if let Ok(tricks) = session.hand().review_tricks(owner) {
                let button = gtk::Button::with_label(&caption);
                button.set_height_request(44);
                button.connect_clicked(move |button| {
                    review::dialog(&caption, tricks.clone(), theme, button)
                });
                reviews.append(&button);
            }
        }
        if reviews.first_child().is_some() {
            body.append(&reviews);
        }
    }

    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .propagate_natural_height(true)
        .child(&body)
        .build();
    frame.append(&scroll);
    let advance_label = if partie.is_some() {
        tr!("New Match")
    } else {
        tr!("Next Hand")
    };
    let advance = gtk::Button::with_label(&advance_label);
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

fn reason(reason: FinishReason) -> String {
    match reason {
        FinishReason::Claimed66 => tr!("Correctly claimed 66 card points"),
        FinishReason::FalseClaim => tr!("False claim of 66 card points"),
        FinishReason::Reached66 => tr!("Reached 66 card points"),
        FinishReason::LastTrick => tr!("Decided by the last trick"),
        FinishReason::SuccessfulClosure => tr!("Closing the stock succeeded"),
        FinishReason::FailedClosure => tr!("Closing the stock failed"),
    }
}

fn grade(outcome: Outcome) -> Option<String> {
    match (outcome.reason, outcome.game_points) {
        (FinishReason::Reached66 | FinishReason::Claimed66, 2) => Some(tr!("Schneider")),
        (FinishReason::Reached66 | FinishReason::Claimed66, 3) => Some(tr!("Schwarz")),
        (FinishReason::SuccessfulClosure, 2) => Some(tr!("Schneider after closing the stock")),
        (FinishReason::SuccessfulClosure, 3) => Some(tr!("Schwarz after closing the stock")),
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
            Some(tr!("Schneider"))
        );
        assert_eq!(
            grade(outcome(FinishReason::Reached66, 3)),
            Some(tr!("Schwarz"))
        );
        assert_eq!(grade(outcome(FinishReason::LastTrick, 1)), None);
        assert_eq!(
            grade(outcome(FinishReason::SuccessfulClosure, 3)),
            Some(tr!("Schwarz after closing the stock")),
        );
        for points in [2, 3] {
            assert_eq!(grade(outcome(FinishReason::FailedClosure, points)), None);
        }
    }
}
