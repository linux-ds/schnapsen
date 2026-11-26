use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU32, Ordering},
};

use schnapsen::{
    core::{game::Player, match_play::Match, rules::MatchConfig},
    save::SaveStore,
};

struct Directory(PathBuf);

impl Directory {
    fn new() -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let root = std::env::temp_dir().join(format!(
            "schnapsen-lab-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed),
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }

    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_schnapsen_lab"))
            .current_dir(&self.0)
            .args(args)
            .output()
            .unwrap()
    }

    fn check(&self, args: &[&str]) {
        let result = self.run(args);
        assert!(
            result.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&result.stderr),
        );
    }

    fn bytes(&self, path: impl AsRef<Path>) -> Vec<u8> {
        fs::read(self.0.join(path)).unwrap()
    }
}

impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn records_a_seed_and_replays_the_export() {
    let directory = Directory::new();
    directory.check(&[
        "record",
        "--seed",
        "19",
        "--profile",
        "sharp",
        "--schneider",
        "single",
        "--local",
        "medium",
        "--opponent",
        "easy",
        "--forehand",
        "opponent",
        "--mirror",
        "--validate",
        "--output",
        "case.json",
    ]);
    directory.check(&["replay", "case.json"]);
    directory.check(&["validate-case", "case.json"]);
    assert!(!directory
        .run(&["record", "--output", "case.json"])
        .status
        .success());
    directory.check(&["replay", "case.json"]);
}

#[test]
fn importing_a_save_is_read_only() {
    let directory = Directory::new();
    let mut state = Match::with_config(Player::Local, MatchConfig::default(), |upper| upper - 1);
    state
        .play_card(Player::Local, state.hand().legal_cards(Player::Local)[0])
        .unwrap();
    let mut save = SaveStore::new(directory.0.join("save.json"));
    assert!(save.load().unwrap().is_none());
    save.save(&state, &[]).unwrap();
    let before = directory.bytes("save.json");
    directory.check(&["validate-save", "save.json"]);
    directory.check(&["from-save", "save.json", "--output", "regression.json"]);
    directory.check(&["replay", "regression.json"]);
    assert_eq!(directory.bytes("save.json"), before);
}
