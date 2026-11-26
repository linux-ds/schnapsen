use std::{
    fmt,
    fs::{self, DirBuilder, File, OpenOptions},
    io::{self, Read, Write},
    os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    ai,
    core::{cards::Card, game::Player, match_play::Match, rules::Difficulty},
};
use serde::{Deserialize, Serialize};

const VERSION: u64 = 3;
const MAX_BYTES: usize = 512 * 1024;
const FILE_NAME: &str = "save.json";

#[derive(Debug)]
pub enum SaveError {
    Io(io::Error),
    Corrupt,
    UnsupportedVersion(u64),
    Blocked,
}

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::Corrupt => f.write_str("invalid save file"),
            Self::UnsupportedVersion(version) => write!(f, "unsupported save version {version}"),
            Self::Blocked => f.write_str("save writes are blocked"),
        }
    }
}

impl From<io::Error> for SaveError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub session: Match,
    pub selection: Vec<Card>,
}

#[derive(Deserialize)]
struct Header {
    version: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SaveFile {
    version: u64,
    strategy_id: String,
    snapshot: Snapshot,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacySaveFile {
    version: u64,
    snapshot: Snapshot,
}

#[derive(Serialize)]
struct SaveFileRef<'a> {
    version: u64,
    strategy_id: &'static str,
    snapshot: SnapshotRef<'a>,
}

#[derive(Serialize)]
struct SnapshotRef<'a> {
    session: &'a Match,
    selection: &'a [Card],
}

enum State {
    Unread,
    Ready {
        last_bytes: Option<Vec<u8>>,
        legacy: Option<(u64, Vec<u8>)>,
    },
    Blocked,
}

pub struct SaveStore {
    path: PathBuf,
    state: State,
}

impl SaveStore {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            state: State::Unread,
        }
    }

    pub fn load(&mut self) -> Result<Option<Snapshot>, SaveError> {
        self.state = State::Blocked;
        let Some(bytes) = read_file(&self.path)? else {
            self.state = State::Ready {
                last_bytes: None,
                legacy: None,
            };
            return Ok(None);
        };
        let (snapshot, version) = decode(&bytes)?;
        // Migrate after validation, using the same atomic write as a normal save.
        self.state = if version == VERSION {
            State::Ready {
                last_bytes: Some(encode(&snapshot.session, &snapshot.selection)?),
                legacy: None,
            }
        } else {
            State::Ready {
                last_bytes: None,
                legacy: Some((version, bytes)),
            }
        };
        Ok(Some(snapshot))
    }

    pub fn save(&mut self, session: &Match, selection: &[Card]) -> Result<(), SaveError> {
        let State::Ready { last_bytes, legacy } = &mut self.state else {
            return Err(SaveError::Blocked);
        };
        let bytes = encode(session, selection)?;
        if last_bytes.as_ref() == Some(&bytes) {
            return Ok(());
        }
        if let Some((version, original)) = legacy {
            let parent = self
                .path
                .parent()
                .expect("save file has a parent directory");
            let backup = unique_dir(parent, &format!("legacy-v{version}"))?;
            replace(&backup.join(FILE_NAME), |file| file.write_all(original))?;
            File::open(parent)?.sync_all()?;
            *legacy = None;
        }
        replace(&self.path, |file| file.write_all(&bytes))?;
        *last_bytes = Some(bytes);
        Ok(())
    }

    /// Preserve the old file after an explicit choice to start playing again.
    pub fn archive_invalid(&mut self) -> Result<(), SaveError> {
        if !matches!(self.state, State::Blocked) {
            return Err(SaveError::Blocked);
        }
        let parent = self
            .path
            .parent()
            .expect("save file has a parent directory");
        match fs::symlink_metadata(&self.path) {
            Ok(_) => {
                let archive_dir = unique_dir(parent, "unreadable")?;
                if let Err(error) = fs::rename(&self.path, archive_dir.join(FILE_NAME)) {
                    let _ = fs::remove_dir(&archive_dir);
                    return Err(error.into());
                }
                File::open(&archive_dir)?.sync_all()?;
                File::open(parent)?.sync_all()?;
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        self.state = State::Ready {
            last_bytes: None,
            legacy: None,
        };
        Ok(())
    }
}

fn read_file(path: &Path) -> Result<Option<Vec<u8>>, SaveError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.is_file() || metadata.len() > MAX_BYTES as u64 => {
            return Err(SaveError::Corrupt);
        }
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    }
    let mut bytes = Vec::new();
    File::open(path)?
        .take(MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_BYTES {
        return Err(SaveError::Corrupt);
    }
    Ok(Some(bytes))
}

fn decode(bytes: &[u8]) -> Result<(Snapshot, u64), SaveError> {
    let header: Header = serde_json::from_slice(bytes).map_err(|_| SaveError::Corrupt)?;
    let snapshot = match header.version {
        1 => {
            let LegacySaveFile { version, snapshot } =
                serde_json::from_slice(bytes).map_err(|_| SaveError::Corrupt)?;
            debug_assert_eq!(version, 1);
            if snapshot.session.config().difficulty != Difficulty::Easy {
                return Err(SaveError::Corrupt);
            }
            snapshot
        }
        2 | VERSION => {
            let SaveFile {
                version,
                strategy_id,
                snapshot,
            } = serde_json::from_slice(bytes).map_err(|_| SaveError::Corrupt)?;
            debug_assert_eq!(version, header.version);
            if strategy_id != ai::strategy_id(snapshot.session.config().difficulty) {
                return Err(SaveError::Corrupt);
            }
            snapshot
        }
        version => return Err(SaveError::UnsupportedVersion(version)),
    };
    if !snapshot.session.validate_snapshot() {
        return Err(SaveError::Corrupt);
    }
    let hand = snapshot.session.hand().view(Player::Local).hand;
    if snapshot.selection.len() > 2
        || snapshot
            .selection
            .iter()
            .enumerate()
            .any(|(index, card)| !hand.contains(card) || snapshot.selection[..index].contains(card))
    {
        return Err(SaveError::Corrupt);
    }
    Ok((snapshot, header.version))
}

fn encode(session: &Match, selection: &[Card]) -> Result<Vec<u8>, SaveError> {
    let bytes = serde_json::to_vec(&SaveFileRef {
        version: VERSION,
        strategy_id: ai::strategy_id(session.config().difficulty),
        snapshot: SnapshotRef { session, selection },
    })
    .map_err(|_| SaveError::Corrupt)?;
    if bytes.len() > MAX_BYTES {
        return Err(SaveError::Corrupt);
    }
    Ok(bytes)
}

fn unique_name(prefix: &str) -> String {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let process = std::process::id();
    let counter = NEXT.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}-{time}-{process}-{counter}")
}

fn unique_dir(parent: &Path, prefix: &str) -> io::Result<PathBuf> {
    loop {
        let path = parent.join(unique_name(prefix));
        match DirBuilder::new().mode(0o700).create(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
}

fn replace(path: &Path, write: impl FnOnce(&mut File) -> io::Result<()>) -> io::Result<()> {
    let parent = path.parent().expect("save file has a parent directory");
    DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(parent)?;
    let (temporary, mut file) = loop {
        let temporary = parent.join(unique_name(".save-tmp"));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)
        {
            Ok(file) => break (temporary, file),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    };
    let result = (|| {
        write(&mut file)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, path)?;
        File::open(parent)?.sync_all()
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{game::Variant, match_play::SchneiderRule};

    struct Directory(PathBuf);

    impl Directory {
        fn new() -> Self {
            Self(unique_dir(&std::env::temp_dir(), "schnapsen-save-test").unwrap())
        }

        fn path(&self) -> PathBuf {
            self.0.join(FILE_NAME)
        }

        fn store(&self) -> SaveStore {
            SaveStore::new(self.path())
        }
    }

    impl Drop for Directory {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).ok();
        }
    }

    fn new_match(variant: Variant, schneider: SchneiderRule) -> Match {
        Match::new(Player::Local, variant, schneider, |upper| upper - 1)
    }

    #[test]
    fn restores_saved_cards_and_selection_in_a_separate_store() {
        let directory = Directory::new();
        let game = new_match(Variant::Sharp, SchneiderRule::Single);
        let card = game.hand().view(Player::Local).hand[0];
        let selection = [card];
        let mut store = directory.store();
        assert!(store.load().unwrap().is_none());
        store.save(&game, &selection).unwrap();
        let restored = directory.store().load().unwrap().unwrap();
        assert_eq!(restored.session, game);
        assert_eq!(restored.selection, selection);
    }

    #[test]
    fn restores_every_strategy_with_observed_exchange() {
        for difficulty in Difficulty::ALL {
            let directory = Directory::new();
            let mut session = Match::with_config(
                Player::Local,
                crate::core::rules::MatchConfig {
                    difficulty,
                    ..Default::default()
                },
                |upper| if upper == 10 { 7 } else { upper - 1 },
            );
            for player in [Player::Local, Player::Opponent] {
                session
                    .play_card(player, session.hand().legal_cards(player)[0])
                    .unwrap();
            }
            let leader = session.resolve_trick().unwrap();
            session.exchange_trump(leader).unwrap();
            let view = session.observation(leader);
            let mut store = directory.store();
            store.load().unwrap();
            store.save(&session, &[]).unwrap();
            let restored = directory.store().load().unwrap().unwrap().session;
            assert_eq!(restored, session);
            assert_eq!(restored.observation(leader), view);
            assert_eq!(restored.config().difficulty, difficulty);
            assert_eq!(
                ai::decide(difficulty, &restored.observation(leader)),
                ai::decide(difficulty, &view),
            );
        }
    }

    #[test]
    fn version_one_resumes_pending_computer_turn_and_migrates_atomically() {
        let directory = Directory::new();
        let mut session = new_match(Variant::Soft, SchneiderRule::Double);
        let lead = session.hand().legal_cards(Player::Local)[0];
        session.play_card(Player::Local, lead).unwrap();
        let mut value: serde_json::Value =
            serde_json::from_slice(&encode(&session, &[]).unwrap()).unwrap();
        let object = value.as_object_mut().unwrap();
        object.remove("strategy_id");
        object.insert("version".into(), 1.into());
        let old_session = value["snapshot"]["session"].as_object_mut().unwrap();
        old_session.remove("history");
        old_session
            .get_mut("config")
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove("difficulty");
        old_session
            .get_mut("hand")
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove("public_memory");
        let legacy = serde_json::to_vec(&value).unwrap();
        fs::write(directory.path(), &legacy).unwrap();

        let mut store = directory.store();
        let mut restored = store.load().unwrap().unwrap().session;
        assert_eq!(restored.config().difficulty, Difficulty::Easy);
        assert_eq!(restored.hand(), session.hand());
        assert_eq!(
            restored.history().coverage(),
            crate::core::history::HistoryCoverage::Partial
        );
        assert!(restored.history().entries().is_empty());
        assert_eq!(fs::read(directory.path()).unwrap(), legacy);
        let player = Player::Opponent;
        let action = ai::decide(Difficulty::Easy, &restored.observation(player)).unwrap();
        assert_eq!(
            ai::apply(&mut restored, player, action),
            Ok(ai::ActionEvent::Played)
        );
        assert!(ai::decide(Difficulty::Easy, &restored.observation(player)).is_none());
        store.save(&restored, &[]).unwrap();

        let value: serde_json::Value =
            serde_json::from_slice(&fs::read(directory.path()).unwrap()).unwrap();
        assert_eq!(value["version"], VERSION);
        assert_eq!(value["strategy_id"], "first_legal");
        assert_eq!(directory.store().load().unwrap().unwrap().session, restored);
    }

    #[test]
    fn version_two_migrates_without_inventing_previous_history() {
        use crate::core::{
            history::{HistoryCoverage, PublicEvent},
            rules::MatchConfig,
        };
        let directory = Directory::new();
        let mut session = Match::with_config(
            Player::Local,
            MatchConfig {
                difficulty: Difficulty::Hard,
                ..MatchConfig::default()
            },
            |upper| upper - 1,
        );
        let leader = Player::Local;
        session
            .play_card(leader, session.hand().legal_cards(leader)[0])
            .unwrap();
        let mut value: serde_json::Value =
            serde_json::from_slice(&encode(&session, &[]).unwrap()).unwrap();
        value["version"] = 2.into();
        value["snapshot"]["session"]
            .as_object_mut()
            .unwrap()
            .remove("history");
        let original = serde_json::to_vec(&value).unwrap();
        fs::write(directory.path(), &original).unwrap();
        let mut store = directory.store();
        let mut restored = store.load().unwrap().unwrap().session;
        assert_eq!(restored.config(), session.config());
        assert_eq!(
            restored.observation(Player::Opponent),
            session.observation(Player::Opponent)
        );
        assert_eq!(restored.history().coverage(), HistoryCoverage::Partial);
        assert_eq!(fs::read(directory.path()).unwrap(), original);

        let player = Player::Opponent;
        let card = restored.hand().legal_cards(player)[0];
        restored.play_card(player, card).unwrap();
        store.save(&restored, &[]).unwrap();
        let backup = fs::read_dir(&directory.0)
            .unwrap()
            .find_map(|entry| {
                let entry = entry.unwrap();
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("legacy-v2-")
                    .then(|| entry.path().join(FILE_NAME))
            })
            .unwrap();
        assert_eq!(fs::read(backup).unwrap(), original);
        let restored = directory.store().load().unwrap().unwrap().session;
        assert_eq!(restored.history().coverage(), HistoryCoverage::Partial);
        assert_eq!(
            restored
                .history()
                .public_events()
                .cloned()
                .collect::<Vec<_>>(),
            [PublicEvent::CardPlayed { player, card }],
        );
    }

    #[test]
    fn rejects_inconsistent_strategy_ids() {
        let directory = Directory::new();
        let game = new_match(Variant::Sharp, SchneiderRule::Single);
        let mut value: serde_json::Value =
            serde_json::from_slice(&encode(&game, &[]).unwrap()).unwrap();
        value["strategy_id"] = "foresight".into();
        fs::write(directory.path(), serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(matches!(directory.store().load(), Err(SaveError::Corrupt)));
    }

    #[test]
    fn rejects_invalid_files_and_preserves_them_until_archived() {
        for bytes in [b"{".as_slice(), br#"{"version":999}"#] {
            let directory = Directory::new();
            fs::write(directory.path(), bytes).unwrap();
            let mut store = directory.store();
            assert!(matches!(
                store.load(),
                Err(SaveError::Corrupt | SaveError::UnsupportedVersion(999))
            ));
            let game = new_match(Variant::Soft, SchneiderRule::Double);
            assert!(matches!(store.save(&game, &[]), Err(SaveError::Blocked)));
            assert_eq!(fs::read(directory.path()).unwrap(), bytes);
            store.archive_invalid().unwrap();
            store.save(&game, &[]).unwrap();
            let archived = fs::read_dir(&directory.0)
                .unwrap()
                .flatten()
                .find(|entry| {
                    entry
                        .file_name()
                        .to_string_lossy()
                        .starts_with("unreadable-")
                })
                .unwrap();
            assert_eq!(fs::read(archived.path().join(FILE_NAME)).unwrap(), bytes);
            assert_eq!(directory.store().load().unwrap().unwrap().session, game);
        }
    }

    #[test]
    fn read_errors_do_not_enable_writes_and_unchanged_state_is_not_replaced() {
        use std::os::unix::fs::MetadataExt;

        let directory = Directory::new();
        let unusable = directory.0.join("not-a-directory");
        fs::write(&unusable, b"keep").unwrap();
        let mut blocked = SaveStore::new(unusable.join(FILE_NAME));
        assert!(matches!(blocked.load(), Err(SaveError::Io(_))));
        assert!(matches!(blocked.archive_invalid(), Err(SaveError::Io(_))));
        let game = new_match(Variant::Soft, SchneiderRule::Double);
        assert!(matches!(blocked.save(&game, &[]), Err(SaveError::Blocked)));
        assert_eq!(fs::read(unusable).unwrap(), b"keep");

        let mut store = directory.store();
        store.load().unwrap();
        store.save(&game, &[]).unwrap();
        let inode = fs::metadata(directory.path()).unwrap().ino();
        store.save(&game, &[]).unwrap();
        assert_eq!(fs::metadata(directory.path()).unwrap().ino(), inode);
        let mut restored = directory.store();
        restored.load().unwrap();
        restored.save(&game, &[]).unwrap();
        assert_eq!(fs::metadata(directory.path()).unwrap().ino(), inode);
    }

    #[test]
    fn failed_or_interrupted_writes_leave_the_last_saved_snapshot_available() {
        let directory = Directory::new();
        let mut store = directory.store();
        store.load().unwrap();
        let game = new_match(Variant::Soft, SchneiderRule::Double);
        store.save(&game, &[]).unwrap();
        let saved_bytes = fs::read(directory.path()).unwrap();

        let orphan = directory.0.join(".save-tmp-interrupted");
        fs::write(&orphan, b"{").unwrap();
        let result = replace(&directory.path(), |file| {
            file.write_all(b"{")?;
            Err(io::Error::other("simulated write failure"))
        });
        assert!(result.is_err());
        assert_eq!(fs::read(directory.path()).unwrap(), saved_bytes);
        assert_eq!(fs::read(&orphan).unwrap(), b"{");
        assert_eq!(fs::read_dir(&directory.0).unwrap().count(), 2);
        assert_eq!(directory.store().load().unwrap().unwrap().session, game);
    }

    #[test]
    fn rejects_invalid_selections_unknown_ids_and_large_files() {
        let directory = Directory::new();
        let game = new_match(Variant::Soft, SchneiderRule::Double);
        let card = game.hand().view(Player::Local).hand[0];
        fs::write(directory.path(), encode(&game, &[card, card]).unwrap()).unwrap();
        assert!(matches!(directory.store().load(), Err(SaveError::Corrupt)));
        for key in ["variant", "profile"] {
            let mut value: serde_json::Value =
                serde_json::from_slice(&encode(&game, &[]).unwrap()).unwrap();
            value["snapshot"]["session"]["config"][key] =
                serde_json::Value::String(String::from("unknown"));
            fs::write(directory.path(), serde_json::to_vec(&value).unwrap()).unwrap();
            assert!(matches!(directory.store().load(), Err(SaveError::Corrupt)));
        }
        fs::write(directory.path(), vec![b' '; MAX_BYTES + 1]).unwrap();
        assert!(matches!(directory.store().load(), Err(SaveError::Corrupt)));
    }
}
