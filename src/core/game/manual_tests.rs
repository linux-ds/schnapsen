use super::*;
use crate::core::cards::full_deck;

#[test]
fn manual_leads_distinguish_talon_actions_at_start_and_at_two_cards() {
    for (profile, soft) in [
        (Variant::ClassicSoft, true),
        (Variant::ClassicSharp, false),
        (Variant::TournamentSoft, true),
        (Variant::TournamentSharp, false),
    ] {
        let deal = || {
            Game::with_variant(Player::Local, profile, |upper| {
                if upper == 10 {
                    7
                } else {
                    upper - 1
                }
            })
        };
        let mut game = deal();
        assert!(game.check_close_talon(Player::Local).is_ok());
        assert_eq!(game.trump_exchange(Player::Local).is_ok(), soft);
        let cards = full_deck();
        game.play_card(Player::Local, cards[0]).unwrap();
        game.play_card(Player::Opponent, cards[3]).unwrap();
        game.resolve_trick().unwrap();
        assert_eq!(
            game.claim_window(),
            Some(ClaimWindow::AfterTrick(Player::Local))
        );
        assert_eq!(
            game.trump_exchange(Player::Local),
            Err(ExchangeError::TrickStarted)
        );
        game.continue_claim(Player::Local).unwrap();
        assert!(game.trump_exchange(Player::Local).is_ok());
        game.talon.drain(..game.talon.len() - 2);
        assert_eq!(
            game.check_close_talon(Player::Local),
            Err(CloseError::NotEnoughTalonCards)
        );
        assert_eq!(game.trump_exchange(Player::Local).is_ok(), soft);
    }
}

#[test]
fn manual_claim_window_precedes_draw_and_survives_serialization() {
    let cards = full_deck();
    let mut game = Game::with_variant(Player::Local, Variant::ClassicSoft, |upper| upper - 1);
    assert_eq!(game.claim(Player::Local), Err(ClaimError::NoWindow));
    game.play_card(Player::Local, cards[0]).unwrap();
    game.play_card(Player::Opponent, cards[3]).unwrap();
    game.resolve_trick().unwrap();
    assert!(game.validate_snapshot(Player::Local));
    let saved = serde_json::to_vec(&game).unwrap();
    let mut game: Game = serde_json::from_slice(&saved).unwrap();
    assert_eq!(
        game.claim_window(),
        Some(ClaimWindow::AfterTrick(Player::Local))
    );
    assert_eq!(game.talon.len(), 10);
    assert_eq!(game.hands.each_ref().map(Vec::len), [4, 4]);
    assert!(game.legal_cards(Player::Local).is_empty());
    assert_eq!(game.claim(Player::Opponent), Err(ClaimError::WrongPlayer));
    assert_eq!(
        game.continue_claim(Player::Opponent),
        Err(ClaimError::WrongPlayer)
    );
    let mut invalid = game.clone();
    invalid.claim_window = Some(ClaimWindow::AfterTrick(Player::Opponent));
    assert!(!invalid.validate_snapshot(Player::Local));
    let mut false_claim = game.clone();
    false_claim.claim(Player::Local).unwrap();
    assert_eq!(
        false_claim.result().unwrap().outcome,
        Outcome {
            reason: FinishReason::FalseClaim,
            winner: Player::Opponent,
            game_points: 3,
            closure: None,
        }
    );
    assert!(false_claim.validate_snapshot(Player::Local));
    assert_eq!(false_claim.claim(Player::Local), Err(ClaimError::NoWindow));
    game.continue_claim(Player::Local).unwrap();
    assert_eq!(game.talon.len(), 8);
    assert_eq!(game.hands.each_ref().map(Vec::len), [5, 5]);
    assert_eq!(game.next_player(), Some(Player::Local));
    assert!(game.validate_snapshot(Player::Local));
}

#[test]
fn marriage_window_protects_pending_eyes_and_waits_for_claim_before_reply() {
    let cards = full_deck();
    let mut game = Game::with_variant(Player::Local, Variant::ClassicSoft, |upper| upper - 1);
    let announced = game
        .announce_and_play(Player::Local, Suit::Bells, cards[8])
        .unwrap();
    assert_eq!(announced.played.rank, Rank::Ober);
    assert_eq!(
        game.claim_window(),
        Some(ClaimWindow::AfterMarriage(Player::Local))
    );
    assert_eq!(game.eye_breakdown(Player::Local).pending_announcements, 40);
    assert_eq!(game.eye_breakdown(Player::Local).effective_announcements, 0);
    assert!(game.validate_snapshot(Player::Local));
    let mut false_claim = game.clone();
    false_claim.claim(Player::Local).unwrap();
    assert!(false_claim.validate_snapshot(Player::Local));
    assert_eq!(
        false_claim.result().unwrap().outcome.reason,
        FinishReason::FalseClaim
    );
    game.continue_claim(Player::Local).unwrap();
    assert_eq!(game.next_player(), Some(Player::Opponent));
    assert!(game.validate_snapshot(Player::Local));
    game.play_card(Player::Opponent, cards[3]).unwrap();
    assert_eq!(game.resolve_trick().unwrap(), Player::Local);
    assert_eq!(game.eye_breakdown(Player::Local).pending_announcements, 0);
    assert_eq!(
        game.eye_breakdown(Player::Local).effective_announcements,
        40
    );
    assert_eq!(game.eyes(Player::Local), 46);
    assert!(game.validate_snapshot(Player::Local));

    let mut sharp = Game::with_variant(Player::Local, Variant::ClassicSharp, |upper| upper - 1);
    assert_eq!(
        sharp.announce_and_play(Player::Local, Suit::Bells, cards[7]),
        Err(MarriageError::NoTrickWon)
    );
    sharp.play_card(Player::Local, cards[0]).unwrap();
    sharp.play_card(Player::Opponent, cards[3]).unwrap();
    sharp.resolve_trick().unwrap();
    sharp.continue_claim(Player::Local).unwrap();
    assert_eq!(
        sharp.announce_and_play(Player::Local, Suit::Bells, cards[8]),
        Err(MarriageError::InvalidLeadCard)
    );
    assert!(sharp
        .announce_and_play(Player::Local, Suit::Bells, cards[7])
        .is_ok());
}

#[test]
fn exact_and_above_target_success_waits_for_the_manual_action() {
    let cards = full_deck();
    for (reply, expected) in [(12, 66), (16, 72)] {
        let mut game = Game::with_variant(Player::Local, Variant::ClassicSoft, |upper| upper - 1);
        game.tricks[0] = [[0, 5], [10, 15], [1, 2]]
            .map(|[a, b]| [cards[a], cards[b]])
            .to_vec();
        game.hands[0] = vec![cards[7]];
        game.hands[1] = vec![cards[reply]];
        game.play_card(Player::Local, cards[7]).unwrap();
        game.play_card(Player::Opponent, cards[reply]).unwrap();
        game.resolve_trick().unwrap();
        assert_eq!(game.eyes(Player::Local), expected);
        assert!(game.result().is_none());
        let mut declined = game.clone();
        declined.continue_claim(Player::Local).unwrap();
        assert!(declined.result().is_none());
        game.claim(Player::Local).unwrap();
        assert_eq!(
            game.result().unwrap().outcome.reason,
            FinishReason::Claimed66
        );
        assert_eq!(game.result().unwrap().outcome.game_points, 3);
    }
}

#[test]
fn effective_announcement_can_be_claimed_before_the_reply() {
    let cards = full_deck();
    let mut game = Game::with_variant(Player::Local, Variant::ClassicSoft, |upper| upper - 1);
    game.phase = Phase::EmptyTalon;
    game.talon.clear();
    game.tricks[0] = [[0, 5], [10, 15], [1, 4]]
        .map(|[a, b]| [cards[a], cards[b]])
        .to_vec();
    game.tricks[1].push([cards[2], cards[3]]);
    game.hands[0] = vec![cards[7], cards[8]];
    let event = game
        .announce_and_play(Player::Local, Suit::Bells, cards[8])
        .unwrap();
    assert_eq!(event.marriage.eyes, 40);
    assert_eq!(game.eyes(Player::Local), 96);
    assert_eq!(game.next_player(), None);
    assert_eq!(game.played, [(Player::Local, cards[8])]);
    assert!(game.result().is_none());
    game.claim(Player::Local).unwrap();
    assert_eq!(game.result().unwrap().outcome.game_points, 2);
    assert_eq!(
        game.result().unwrap().outcome.reason,
        FinishReason::Claimed66
    );
}

#[test]
fn successful_closure_uses_the_selected_score_moment_at_exactly_33() {
    let cards = full_deck();
    for (profile, expected) in [(Variant::ClassicSoft, 2), (Variant::TournamentSoft, 1)] {
        let mut game = Game::with_variant(Player::Local, profile, |upper| upper - 1);
        game.tricks[1].push([cards[6], cards[11]]);
        game.close_talon(Player::Local).unwrap();
        game.tricks[1].push([cards[16], cards[8]]);
        assert_eq!(game.eyes(Player::Opponent), 33);
        game.tricks[0] = [[0, 5], [10, 15], [1, 2], [7, 12]]
            .map(|[a, b]| [cards[a], cards[b]])
            .to_vec();
        game.claim_window = Some(ClaimWindow::AfterTrick(Player::Local));
        game.claim(Player::Local).unwrap();
        assert_eq!(
            game.result().unwrap().outcome.reason,
            FinishReason::SuccessfulClosure
        );
        assert_eq!(game.result().unwrap().outcome.game_points, expected);
    }
}

#[test]
fn false_claim_never_reduces_closure_penalty_and_opponent_can_claim_first() {
    let cards = full_deck();
    for profile in [Variant::ClassicSoft, Variant::TournamentSoft] {
        let mut game = Game::with_variant(Player::Local, profile, |upper| upper - 1);
        game.close_talon(Player::Local).unwrap();
        game.tricks[1].push([cards[6], cards[11]]);
        game.claim_window = Some(ClaimWindow::AfterTrick(Player::Local));
        let mut false_claim = game.clone();
        false_claim.claim(Player::Local).unwrap();
        assert_eq!(
            false_claim.result().unwrap().outcome.reason,
            FinishReason::FalseClaim
        );
        assert_eq!(false_claim.result().unwrap().outcome.game_points, 3);
        game.tricks[1] = [[0, 5], [10, 15], [1, 2], [7, 12]]
            .map(|[a, b]| [cards[a], cards[b]])
            .to_vec();
        game.claim_window = Some(ClaimWindow::AfterTrick(Player::Opponent));
        game.claim(Player::Opponent).unwrap();
        assert_eq!(
            game.result().unwrap().outcome.reason,
            FinishReason::FailedClosure
        );
        assert_eq!(game.result().unwrap().outcome.winner, Player::Opponent);
        assert_eq!(game.result().unwrap().outcome.game_points, 3);
    }
}

#[test]
fn manual_last_trick_still_ends_on_decline_and_respects_closure() {
    let cards = full_deck();
    for closed in [false, true] {
        let mut game = Game::with_variant(Player::Local, Variant::ClassicSharp, |upper| upper - 1);
        if closed {
            game.close_talon(Player::Local).unwrap();
        } else {
            game.phase = Phase::EmptyTalon;
        }
        game.hands = [vec![cards[0]], vec![cards[3]]];
        game.play_card(Player::Local, cards[0]).unwrap();
        game.play_card(Player::Opponent, cards[3]).unwrap();
        game.resolve_trick().unwrap();
        assert!(game.result().is_none());
        assert_eq!(
            game.review_tricks(Player::Local),
            Err(TrickReviewError::Forbidden)
        );
        game.continue_claim(Player::Local).unwrap();
        let outcome = game.result().unwrap().outcome;
        assert_eq!(
            (outcome.winner, outcome.reason, outcome.game_points),
            if closed {
                (Player::Opponent, FinishReason::FailedClosure, 3)
            } else {
                (Player::Local, FinishReason::LastTrick, 1)
            }
        );
        assert_eq!(game.review_tricks(Player::Local).unwrap().len(), 1);
        assert_eq!(
            game.review_tricks(Player::Opponent),
            Err(TrickReviewError::Empty)
        );
    }
}
