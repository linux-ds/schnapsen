//! Versioned developer fixtures and deterministic replay through the engine.

use std::{
    fmt,
    io::{Read, Write},
};

use serde::{Deserialize, Serialize};

use crate::{
    ai::Action,
    core::{
        cards::{Card, Suit},
        game::Player,
        history::{History, HistoryCoverage, HistoryEntry, PrivateEvent, PublicEvent},
        match_play::Match,
    },
};

use super::{
    simulation::{shuffle, MatchSetup},
    validation::{validate, validate_transition},
};

const VERSION: u32 = 1;
const MAX_BYTES: usize = 32 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum RecordedAction {
    Play {
        player: Player,
        card: Card,
    },
    Announce {
        player: Player,
        suit: Suit,
        card: Card,
    },
    Exchange {
        player: Player,
    },
    Close {
        player: Player,
    },
    Claim {
        player: Player,
    },
    ContinueClaim {
        player: Player,
    },
    ResolveTrick,
    NextHand,
    ResetBummerls,
}

impl RecordedAction {
    pub fn from_ai(player: Player, action: Action) -> Self {
        match action {
            Action::Play(card) => Self::Play { player, card },
            Action::Announce { suit, card } => Self::Announce { player, suit, card },
            Action::Exchange => Self::Exchange { player },
            Action::Close => Self::Close { player },
            Action::Claim => Self::Claim { player },
            Action::ContinueClaim => Self::ContinueClaim { player },
        }
    }

    pub fn apply(
        &self,
        state: &mut Match,
        shuffle: &mut impl FnMut(usize) -> usize,
    ) -> Result<(), String> {
        match *self {
            Self::Play { player, card } => state
                .play_card(player, card)
                .map_err(|error| format!("{error:?}")),
            Self::Announce { player, suit, card } => state
                .announce_and_play(player, suit, card)
                .map(|_| ())
                .map_err(|error| format!("{error:?}")),
            Self::Exchange { player } => state
                .exchange_trump(player)
                .map(|_| ())
                .map_err(|error| format!("{error:?}")),
            Self::Close { player } => state
                .close_talon(player)
                .map_err(|error| format!("{error:?}")),
            Self::Claim { player } => state.claim(player).map_err(|error| format!("{error:?}")),
            Self::ContinueClaim { player } => state
                .continue_claim(player)
                .map_err(|error| format!("{error:?}")),
            Self::ResolveTrick => state
                .resolve_trick()
                .map(|_| ())
                .map_err(|error| format!("{error:?}")),
            Self::NextHand => state
                .next_hand(shuffle)
                .map_err(|error| format!("{error:?}")),
            Self::ResetBummerls => {
                state.reset_bummerls();
                Ok(())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CaseStart {
    Seed {
        setup: MatchSetup,
        seed: u64,
    },
    Snapshot {
        state: Box<Match>,
        continuation_seed: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Checkpoint {
    /// One-based index into TestCase::actions.
    pub after_action: usize,
    pub state: Box<Match>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TestCase {
    version: u32,
    /// Application version that originally produced the test case.
    pub created_with: String,
    pub start: CaseStart,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub actions: Vec<RecordedAction>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub checkpoints: Vec<Checkpoint>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_history: Option<History>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_end: Option<Box<Match>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayError {
    /// One-based action index, or None for format and initial-state errors.
    pub action: Option<usize>,
    pub message: String,
}

impl ReplayError {
    fn initial(message: impl Into<String>) -> Self {
        Self {
            action: None,
            message: message.into(),
        }
    }

    fn at(index: usize, message: impl Into<String>) -> Self {
        Self {
            action: Some(index),
            message: message.into(),
        }
    }
}

impl fmt::Display for ReplayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(action) = self.action {
            write!(f, "action {action}: ")?;
        }
        f.write_str(&self.message)
    }
}

impl std::error::Error for ReplayError {}

impl TestCase {
    pub fn new(start: CaseStart) -> Self {
        Self {
            version: VERSION,
            created_with: env!("CARGO_PKG_VERSION").to_owned(),
            start,
            description: None,
            actions: Vec::new(),
            checkpoints: Vec::new(),
            expected_history: None,
            expected_end: None,
        }
    }

    pub fn read(reader: impl Read) -> Result<Self, ReplayError> {
        let mut bytes = Vec::new();
        reader
            .take(MAX_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| ReplayError::initial(error.to_string()))?;
        if bytes.len() > MAX_BYTES {
            return Err(ReplayError::initial("test case exceeds the size limit"));
        }
        #[derive(Deserialize)]
        struct Header {
            version: u32,
        }
        let header: Header = serde_json::from_slice(&bytes)
            .map_err(|error| ReplayError::initial(format!("invalid test case: {error}")))?;
        if header.version != VERSION {
            return Err(ReplayError::initial(format!(
                "unsupported test-case version {}",
                header.version
            )));
        }
        let case: Self = serde_json::from_slice(&bytes)
            .map_err(|error| ReplayError::initial(format!("invalid test case: {error}")))?;
        case.check_format()?;
        Ok(case)
    }

    pub fn write(&self, writer: impl Write) -> Result<(), serde_json::Error> {
        serde_json::to_writer_pretty(writer, self)
    }

    fn check_format(&self) -> Result<(), ReplayError> {
        if self.version != VERSION {
            return Err(ReplayError::initial(format!(
                "unsupported test-case version {}",
                self.version
            )));
        }
        let mut previous = 0;
        for checkpoint in &self.checkpoints {
            if checkpoint.after_action <= previous || checkpoint.after_action > self.actions.len() {
                return Err(ReplayError::initial(
                    "checkpoints must refer to ascending action indices",
                ));
            }
            previous = checkpoint.after_action;
        }
        Ok(())
    }

    pub fn replay(&self) -> Result<Match, ReplayError> {
        self.check_format()?;
        let seed = match &self.start {
            CaseStart::Seed { seed, .. } => *seed,
            CaseStart::Snapshot {
                continuation_seed, ..
            } => *continuation_seed,
        };
        let mut deal = shuffle(seed);
        let mut state = match &self.start {
            CaseStart::Seed { setup, .. } => {
                validate_profile(setup.engine_config(false))?;
                Match::with_config(setup.forehand, setup.engine_config(false), &mut deal)
            }
            CaseStart::Snapshot { state, .. } => (**state).clone(),
        };

        validate(&state).map_err(|error| ReplayError::initial(error.to_string()))?;
        let expected = self
            .expected_history
            .as_ref()
            .or_else(|| self.expected_end.as_ref().map(|end| end.history()));
        let mut compared = 0;
        if let Some(history) = expected {
            compare_history_prefix(&state, history, &mut compared).map_err(ReplayError::initial)?;
        }
        let mut checkpoints = self.checkpoints.iter().peekable();
        for (index, action) in self.actions.iter().enumerate() {
            let step = index + 1;
            let previous = state.clone();
            action
                .apply(&mut state, &mut deal)
                .map_err(|error| ReplayError::at(step, format!("{action:?} rejected: {error}")))?;
            validate_transition(&previous, action, &state)
                .map_err(|error| ReplayError::at(step, error.to_string()))?;
            if let Some(history) = expected {
                compare_history_prefix(&state, history, &mut compared)
                    .map_err(|error| ReplayError::at(step, error))?;
            }
            if checkpoints
                .peek()
                .is_some_and(|checkpoint| checkpoint.after_action == step)
            {
                let checkpoint = checkpoints.next().expect("checkpoint exists");
                if state != *checkpoint.state {
                    return Err(ReplayError::at(
                        step,
                        "checkpoint differs from the expected state",
                    ));
                }
            }
        }
        let last = self.actions.len();
        if let Some(history) = expected {
            if state.history() != history {
                return Err(ReplayError::at(
                    last,
                    format!("history differs after {compared} events"),
                ));
            }
        }
        if self.expected_end.as_ref().is_some_and(|end| state != **end) {
            return Err(ReplayError::at(last, "end state or scoring differs"));
        }
        Ok(state)
    }
}

fn validate_profile(config: crate::core::rules::MatchConfig) -> Result<(), ReplayError> {
    if !config.variant.profiles().contains(&config.profile) {
        return Err(ReplayError::initial("incompatible variant and profile"));
    }
    if config.schneider == crate::core::rules::SchneiderRule::Double
        && !config.profile.allows_double_schneider()
    {
        return Err(ReplayError::initial(
            "profile requires single Schneider scoring",
        ));
    }
    Ok(())
}

fn compare_history_prefix(
    state: &Match,
    expected: &History,
    compared: &mut usize,
) -> Result<(), String> {
    let actual = state.history().entries();
    for (index, entry) in actual.iter().enumerate().skip(*compared) {
        if expected.entries().get(index) != Some(entry) {
            return Err(format!(
                "history event {} differs: expected {:?}, actual {:?}",
                index + 1,
                expected.entries().get(index),
                entry
            ));
        }
    }
    *compared = actual.len();
    Ok(())
}

fn replay_deal(
    entry: Option<&HistoryEntry>,
    expected_size: usize,
) -> Result<impl FnMut(usize) -> usize + '_, ReplayError> {
    let Some(HistoryEntry {
        public: PublicEvent::HandStarted { .. },
        private: Some(PrivateEvent::Deal {
            shuffle_choices, ..
        }),
    }) = entry
    else {
        return Err(ReplayError::initial("expected a full recorded deal"));
    };
    if shuffle_choices.len() != expected_size.saturating_sub(1)
        || shuffle_choices
            .iter()
            .zip((2..=expected_size).rev())
            .any(|(&choice, upper)| choice >= upper)
    {
        return Err(ReplayError::initial("recorded shuffle choices are invalid"));
    }
    let mut choices = shuffle_choices.iter().copied();
    Ok(move |upper| {
        choices
            .next()
            .filter(|&choice| choice < upper)
            .unwrap_or(upper - 1)
    })
}

/// Replay a complete live-game history from its recorded shuffle choices.
/// This works with user sessions whose original randomness was not seeded by
/// this tool. Partial histories require an explicit starting snapshot.
pub fn replay_history(history: &History) -> Result<Match, ReplayError> {
    if history.coverage() != HistoryCoverage::Complete {
        return Err(ReplayError::initial(
            "history is partial; use a starting snapshot and explicit subsequent actions",
        ));
    }
    let events = history.entries();
    let Some(PublicEvent::MatchStarted {
        config,
        initial_bummerls,
        ..
    }) = events.first().map(|entry| &entry.public)
    else {
        return Err(ReplayError::initial("missing match start"));
    };
    let Some(PublicEvent::HandStarted { forehand, .. }) = events.get(1).map(|entry| &entry.public)
    else {
        return Err(ReplayError::initial("missing first hand"));
    };

    validate_profile(*config)?;
    let deck_size = config.variant.deck().len();
    let mut state = Match::with_initial_bummerls(
        *forehand,
        *config,
        *initial_bummerls,
        replay_deal(events.get(1), deck_size)?,
    );
    validate(&state).map_err(|error| ReplayError::initial(error.to_string()))?;
    let mut compared = 0;
    compare_history_prefix(&state, history, &mut compared).map_err(ReplayError::initial)?;
    let mut action_count = 0;

    while let Some(entry) = events.get(compared) {
        action_count += 1;
        let action = match entry.public {
            PublicEvent::CardPlayed { player, card } => {
                if let Some(PublicEvent::MarriageAnnounced { announcement }) =
                    events.get(compared + 1).map(|entry| &entry.public)
                {
                    RecordedAction::Announce {
                        player,
                        card,
                        suit: announcement.marriage.suit,
                    }
                } else {
                    RecordedAction::Play { player, card }
                }
            }
            PublicEvent::TrumpExchanged { exchange } => RecordedAction::Exchange {
                player: exchange.player,
            },
            PublicEvent::TalonClosed { closure } => RecordedAction::Close {
                player: closure.closer,
            },
            PublicEvent::TrickResolved { .. } => RecordedAction::ResolveTrick,
            PublicEvent::Claimed { player, .. } => RecordedAction::Claim { player },
            PublicEvent::ClaimDeclined { player } => RecordedAction::ContinueClaim { player },
            PublicEvent::HandStarted { .. } => RecordedAction::NextHand,
            PublicEvent::BummerlsReset => RecordedAction::ResetBummerls,
            _ => {
                return Err(ReplayError::at(
                    action_count,
                    format!("unexpected standalone history event {}", compared + 1),
                ))
            }
        };
        let previous = state.clone();
        let mut fallback = |upper| upper - 1;
        if action == RecordedAction::NextHand {
            let mut deal = replay_deal(Some(entry), deck_size)
                .map_err(|error| ReplayError::at(action_count, error.to_string()))?;
            action.apply(&mut state, &mut deal)
        } else {
            action.apply(&mut state, &mut fallback)
        }
        .map_err(|error| ReplayError::at(action_count, format!("{action:?} rejected: {error}")))?;

        validate_transition(&previous, &action, &state)
            .map_err(|error| ReplayError::at(action_count, error.to_string()))?;
        let previous_event = compared;
        compare_history_prefix(&state, history, &mut compared)
            .map_err(|error| ReplayError::at(action_count, error))?;
        if compared <= previous_event {
            return Err(ReplayError::at(
                action_count,
                "action produced no recorded events",
            ));
        }
    }
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::rules::MatchConfig;

    fn play_trick(state: &mut Match, mut record: impl FnMut(RecordedAction)) {
        for _ in 0..2 {
            let player = state.hand().next_player().expect("player at turn");
            let card = state.hand().legal_cards(player)[0];
            state.play_card(player, card).expect("legal card");
            record(RecordedAction::Play { player, card });
        }
        state.resolve_trick().expect("complete trick");
        record(RecordedAction::ResolveTrick);
    }

    #[test]
    fn seeded_case_round_trips_and_reports_first_divergent_checkpoint_or_action() {
        let setup = MatchSetup::default();
        let mut state = Match::with_config(setup.forehand, setup.engine_config(false), shuffle(17));
        let mut case = TestCase::new(CaseStart::Seed { setup, seed: 17 });
        play_trick(&mut state, |action| case.actions.push(action));
        case.expected_end = Some(Box::new(state.clone()));
        case.expected_history = Some(state.history().clone());
        case.checkpoints.push(Checkpoint {
            after_action: 3,
            state: Box::new(state.clone()),
        });

        let mut bytes = Vec::new();
        case.write(&mut bytes).unwrap();
        let mut loaded = TestCase::read(bytes.as_slice()).unwrap();
        assert_eq!(loaded, case);
        assert_eq!(loaded.replay().unwrap(), state);

        let original = loaded.actions[1];
        loaded.actions[1] = RecordedAction::NextHand;
        let error = loaded.replay().unwrap_err();
        assert_eq!(error.action, Some(2));
        assert!(error.message.contains("rejected"));

        loaded.actions[1] = original;
        loaded.checkpoints[0].after_action = 2;
        let error = loaded.replay().unwrap_err();
        assert_eq!(error.action, Some(2));
        assert!(error.message.contains("checkpoint"));

        assert!(TestCase::read(br#"{"version":99,"future":[]}"#.as_slice())
            .unwrap_err()
            .message
            .contains("unsupported test-case version"));
    }

    #[test]
    fn manual_claim_and_decline_replay_without_drawing_twice() {
        use crate::core::{game::FinishReason, rules::RuleProfile};

        let setup = MatchSetup {
            profile: RuleProfile::ClassicSoft,
            ..MatchSetup::default()
        };
        let mut state = Match::with_config(setup.forehand, setup.engine_config(false), shuffle(17));
        let mut case = TestCase::new(CaseStart::Seed { setup, seed: 17 });
        play_trick(&mut state, |action| case.actions.push(action));
        let winner = state.claim_player().unwrap();
        assert_eq!(state.hand().view(winner).hidden_talon_count, 9);
        assert_eq!(state.hand().view(winner).hand.len(), 4);
        assert!(state.hand().eyes(winner) <= 22);
        let initial = state.clone();
        for action in [
            RecordedAction::Claim { player: winner },
            RecordedAction::ContinueClaim { player: winner },
        ] {
            let mut state = initial.clone();
            action.apply(&mut state, &mut shuffle(99)).unwrap();
            validate_transition(&initial, &action, &state).unwrap();
            let mut case = case.clone();
            case.actions.push(action);
            case.expected_history = Some(state.history().clone());
            case.expected_end = Some(Box::new(state.clone()));
            assert_eq!(case.replay().unwrap(), state);
            assert_eq!(replay_history(state.history()).unwrap(), state);

            if matches!(action, RecordedAction::Claim { .. }) {
                let outcome = state.hand().result().unwrap().outcome;
                assert_eq!(outcome.reason, FinishReason::FalseClaim);
                assert_eq!(outcome.winner, winner.other());
                assert_eq!(outcome.game_points, 3);
                assert_eq!(state.hand().view(winner).hidden_talon_count, 9);
            } else {
                assert_eq!(state.hand().view(winner).hidden_talon_count, 7);
                assert_eq!(state.hand().view(winner).hand.len(), 5);
                assert_eq!(state.hand().next_player(), Some(winner));
                assert!(state.continue_claim(winner).is_err());
            }
        }
    }

    #[test]
    fn complete_history_replays_multiple_deals_and_rejects_tampering() {
        let mut deal = shuffle(73);
        let mut state = Match::with_initial_bummerls(
            Player::Opponent,
            MatchConfig::default(),
            [1, 2],
            &mut deal,
        );
        for _ in 0..2 {
            while state.hand().result().is_none() {
                play_trick(&mut state, |_| {});
            }
            state.next_hand(&mut deal).unwrap();
        }
        state.reset_bummerls();
        assert_eq!(replay_history(state.history()).unwrap(), state);

        let mut broken = serde_json::to_value(state.history()).unwrap();
        let events = broken["entries"].as_array_mut().unwrap();
        let action = events
            .iter_mut()
            .find(|event| event["public"]["kind"] == "card_played")
            .unwrap();
        action["public"]["player"] = "local".into();
        let history: History = serde_json::from_value(broken).unwrap();
        assert_eq!(replay_history(&history).unwrap_err().action, Some(1));
        assert!(replay_history(&History::default())
            .unwrap_err()
            .message
            .contains("partial"));
    }
}
