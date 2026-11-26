//! Rule-relevant history for the current match. The strategy observation has a
//! separate interface and never includes these developer records.

use serde::{Deserialize, Serialize};

use super::{
    cards::Card,
    game::{Announcement, Exchange, Outcome, Player, TalonClosure},
    rules::MatchConfig,
};

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HistoryCoverage {
    Complete,
    #[default]
    Partial,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PublicEvent {
    MatchStarted {
        config: MatchConfig,
        initial_bummerls: [u32; 2],
    },
    HandStarted {
        forehand: Player,
        trump: Card,
        hand_size: usize,
        talon_count: usize,
    },
    CardPlayed {
        player: Player,
        card: Card,
    },
    MarriageAnnounced {
        announcement: Announcement,
    },
    TrumpExchanged {
        exchange: Exchange,
    },
    TalonClosed {
        closure: TalonClosure,
    },
    TrickResolved {
        winner: Player,
    },
    EyesChanged {
        player: Player,
        eyes: u8,
        activated_marriage_eyes: u8,
    },
    CardsDrawn {
        order: [Player; 2],
        exposed_trump: Option<Card>,
    },
    TalonExhausted,
    HandFinished {
        outcome: Outcome,
    },
    GamePointsAwarded {
        winner: Player,
        points: u8,
        remaining: [u8; 2],
    },
    MatchFinished {
        winner: Player,
    },
    BummerlsAwarded {
        player: Player,
        amount: u8,
        total: u32,
    },
    BummerlsReset,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PrivateEvent {
    Deal {
        hands: [Vec<Card>; 2],
        hidden_talon: Vec<Card>,
        shuffle_choices: Vec<usize>,
    },
    Draw {
        cards: [Card; 2],
    },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HistoryEntry {
    pub public: PublicEvent,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub private: Option<PrivateEvent>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct History {
    coverage: HistoryCoverage,
    entries: Vec<HistoryEntry>,
}

impl History {
    pub(super) fn start(config: MatchConfig, initial_bummerls: [u32; 2]) -> Self {
        let mut history = Self {
            coverage: HistoryCoverage::Complete,
            entries: Vec::new(),
        };
        history.push(
            PublicEvent::MatchStarted {
                config,
                initial_bummerls,
            },
            None,
        );
        history
    }

    pub fn coverage(&self) -> HistoryCoverage {
        self.coverage
    }

    pub fn entries(&self) -> &[HistoryEntry] {
        &self.entries
    }

    pub fn public_events(&self) -> impl ExactSizeIterator<Item = &PublicEvent> {
        self.entries.iter().map(|entry| &entry.public)
    }

    pub(super) fn push(&mut self, public: PublicEvent, private: Option<PrivateEvent>) {
        self.entries.push(HistoryEntry { public, private });
    }

    pub(super) fn check_header(&self, config: MatchConfig) -> bool {
        // With 7 points on each side, 13 hands is the longest legal match.
        // The fixed cap comfortably covers all events in those hands.
        if self.entries.len() > 1024 {
            return false;
        }
        match (self.coverage, self.entries.first()) {
            (HistoryCoverage::Complete, Some(entry)) => {
                matches!(
                    entry,
                    HistoryEntry {
                        public: PublicEvent::MatchStarted { config: recorded, .. },
                        private: None,
                    } if *recorded == config
                ) && matches!(
                    self.entries.get(1),
                    Some(HistoryEntry {
                        public: PublicEvent::HandStarted { .. },
                        private: Some(PrivateEvent::Deal { .. }),
                    })
                )
            }
            (HistoryCoverage::Partial, first) => !matches!(
                first,
                Some(HistoryEntry {
                    public: PublicEvent::MatchStarted { .. },
                    ..
                })
            ),
            _ => false,
        }
    }
}
