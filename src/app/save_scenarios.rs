use std::{
    env, fs,
    path::PathBuf,
    process::Command,
    sync::atomic::{AtomicU32, Ordering},
};

use schnapsen::core::{
    game::{Phase, Player},
    match_play::Match,
    rules::{RuleProfile, SchneiderRule},
};

use super::save::SaveStore;

struct TestSlot(PathBuf);

impl TestSlot {
    fn new() -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let directory = env::temp_dir().join(format!(
            "schnapsen-scenarios-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed),
        ));
        fs::create_dir(&directory).unwrap();
        Self(directory)
    }

    fn path(&self) -> PathBuf {
        self.0.join("save.json")
    }
}

impl Drop for TestSlot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn shuffle(mut seed: u32) -> impl FnMut(usize) -> usize {
    move |upper| {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        seed as usize % upper
    }
}

fn step(session: &mut Match, seed: u32, close: bool) -> &'static str {
    if session.hand().result().is_some() {
        session.next_hand(shuffle(seed)).unwrap();
        return "deal";
    }
    if session.hand().view(Player::Local).played.len() == 2 {
        session.resolve_trick().unwrap();
        return "resolve";
    }
    let hand = session.hand();
    let player = hand.next_player().unwrap();
    if hand.trump_exchange(player).is_ok() {
        session.exchange_trump(player).unwrap();
        return "exchange";
    }
    if close && hand.check_close_talon(player).is_ok() {
        session.close_talon(player).unwrap();
        return "close";
    }
    if let Some(marriage) = hand.legal_marriages(player).first() {
        session
            .announce_and_play(player, marriage.suit, marriage.king())
            .unwrap();
        return "announce";
    }
    let cards = hand.legal_cards(player);
    session
        .play_card(player, cards[seed as usize % cards.len()])
        .unwrap();
    "play"
}

#[test]
fn resuming_a_snapshot_after_each_transition_matches_uninterrupted_play() {
    use std::collections::BTreeSet;

    let required = BTreeSet::from([
        "deal",
        "exchange",
        "close",
        "announce",
        "empty",
        "one-card",
        "computer",
        "two-cards",
        "hand-end",
        "match-end",
    ]);
    for (profile, schneider) in [
        (RuleProfile::Soft, SchneiderRule::Double),
        (RuleProfile::Sharp, SchneiderRule::Single),
    ] {
        let mut covered = BTreeSet::new();
        for seed in 1..=16 {
            let slot = TestSlot::new();
            let close = seed % 2 == 0;
            let mut direct = Match::new(Player::Local, profile, schneider, shuffle(seed));
            let mut resumed = Match::new(Player::Local, profile, schneider, shuffle(seed));
            for turn in 0..200 {
                let hand = direct.hand();
                match hand.view(Player::Local).played.len() {
                    1 => {
                        covered.insert("one-card");
                    }
                    2 => {
                        covered.insert("two-cards");
                    }
                    _ => {}
                }
                if hand.next_player() == Some(Player::Opponent) {
                    covered.insert("computer");
                }
                if hand.phase() == Phase::EmptyTalon {
                    covered.insert("empty");
                }
                if hand.result().is_some() {
                    covered.insert("hand-end");
                }
                let mut writer = SaveStore::new(slot.path());
                writer.load().unwrap();
                writer.save(&resumed, &[]).unwrap_or_else(|error| {
                    panic!("{profile:?} seed {seed} turn {turn}: {error:?}\n{resumed:?}");
                });
                resumed = SaveStore::new(slot.path()).load().unwrap().unwrap().session;
                assert_eq!(direct, resumed);
                if direct.result().is_some() {
                    covered.insert("match-end");
                    break;
                }
                let action_seed = seed * 1024 + turn + 1;
                let action = step(&mut direct, action_seed, close);
                assert_eq!(action, step(&mut resumed, action_seed, close));
                covered.insert(action);
                assert_eq!(direct, resumed);
            }
            assert!(resumed.result().is_some(), "{profile:?}, seed {seed}");
            if required.is_subset(&covered) {
                break;
            }
        }
        assert!(required.is_subset(&covered), "{profile:?}: {covered:?}");
    }
}

#[test]
fn saves_a_selected_card_across_de_and_en_processes() {
    const PATH: &str = "SCHNAPSEN_SAVE_TEST_PATH";
    const MODE: &str = "SCHNAPSEN_SAVE_TEST_MODE";
    let fixture = || {
        Match::new(
            Player::Local,
            RuleProfile::Soft,
            SchneiderRule::Double,
            shuffle(99),
        )
    };
    match env::var(MODE).as_deref() {
        Ok(mode) => {
            crate::i18n::init();
            let expected = fixture();
            let selection = &expected.hand().view(Player::Local).hand[..1];
            let mut store = SaveStore::new(PathBuf::from(env::var_os(PATH).unwrap()));
            if mode == "write" {
                assert_eq!(tr!("Preferences"), "Einstellungen");
                assert!(store.load().unwrap().is_none());
                store.save(&expected, selection).unwrap();
            } else {
                assert_eq!(tr!("Preferences"), "Preferences");
                let snapshot = store.load().unwrap().unwrap();
                assert_eq!(snapshot.session, expected);
                assert_eq!(snapshot.selection, selection);
            }
        }
        Err(_) => {
            let slot = TestSlot::new();
            for (mode, locale) in [("write", "de_AT.UTF-8"), ("read", "en_GB.UTF-8")] {
                let output = Command::new(env::current_exe().unwrap())
                    .args([
                        "--exact",
                        "app::save_scenarios::saves_a_selected_card_across_de_and_en_processes",
                    ])
                    .env(PATH, slot.path())
                    .env(MODE, mode)
                    .env("LC_ALL", locale)
                    .env("LANGUAGE", "")
                    .output()
                    .unwrap();
                let stdout = String::from_utf8_lossy(&output.stdout);
                assert!(
                    output.status.success() && stdout.contains("1 passed"),
                    "{stdout}\n{}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }
    }
}
