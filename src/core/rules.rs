//! Game identity and fixed rules for the supported Austrian two-player game.

use serde::{Deserialize, Serialize};

use super::{
    cards::{full_deck, Card, Suit},
    game::{Phase, Player, Reply, Score, TalonClosure},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GameVariant {
    #[default]
    AustrianSchnapsen,
}

pub(super) struct Deal {
    pub hands: [Vec<Card>; 2],
    pub talon: Vec<Card>,
    pub trump_suit: Suit,
}

impl GameVariant {
    pub const ALL: [Self; 1] = [Self::AustrianSchnapsen];

    pub const fn profiles(self) -> &'static [RuleProfile] {
        match self {
            Self::AustrianSchnapsen => &[
                RuleProfile::Soft,
                RuleProfile::Sharp,
                RuleProfile::ClassicSoft,
                RuleProfile::ClassicSharp,
                RuleProfile::TournamentSoft,
                RuleProfile::TournamentSharp,
            ],
        }
    }

    pub const fn player_count(self) -> usize {
        match self {
            Self::AustrianSchnapsen => 2,
        }
    }

    pub const fn hand_size(self) -> usize {
        match self {
            Self::AustrianSchnapsen => 5,
        }
    }

    pub fn deck(self) -> Vec<Card> {
        match self {
            Self::AustrianSchnapsen => full_deck().into(),
        }
    }

    pub(super) fn deal(self, leader: Player, mut choose_index: impl FnMut(usize) -> usize) -> Deal {
        match self {
            Self::AustrianSchnapsen => {
                let mut deck = self.deck();
                for upper in (2..=deck.len()).rev() {
                    deck[..upper].swap(upper - 1, choose_index(upper));
                }

                let mut hands = std::array::from_fn(|_| Vec::with_capacity(self.hand_size()));
                let forehand = leader as usize;
                let dealer = leader.other() as usize;
                hands[forehand].extend_from_slice(&deck[..3]);
                hands[dealer].extend_from_slice(&deck[3..6]);
                let trump = deck[6];
                hands[forehand].extend_from_slice(&deck[7..9]);
                hands[dealer].extend_from_slice(&deck[9..11]);
                let mut talon = deck[11..].to_vec();
                talon.push(trump);
                Deal {
                    hands,
                    talon,
                    trump_suit: trump.suit,
                }
            }
        }
    }

    pub(super) const fn card_points(self, card: Card) -> u8 {
        match self {
            Self::AustrianSchnapsen => card.points(),
        }
    }

    pub(super) fn beats(self, challenger: Card, lead: Card, trump: Suit) -> bool {
        match self {
            Self::AustrianSchnapsen if challenger.suit == lead.suit => {
                // Eyes follow the same order as card rank in this variant.
                self.card_points(challenger) > self.card_points(lead)
            }
            Self::AustrianSchnapsen => challenger.suit == trump,
        }
    }

    pub(super) fn reply_priority(self, card: Card, lead: Card, trump: Suit) -> Reply {
        if card.suit == lead.suit {
            if self.beats(card, lead, trump) {
                Reply::Beat
            } else {
                Reply::Follow
            }
        } else if card.suit == trump {
            Reply::Trump
        } else {
            Reply::Discard
        }
    }

    pub(super) fn forced_replies(
        self,
        hand: &[Card],
        lead: Card,
        trump: Suit,
    ) -> (Vec<Card>, Reply) {
        let priority = |card| self.reply_priority(card, lead, trump);
        let best = hand
            .iter()
            .copied()
            .map(priority)
            .min()
            .unwrap_or(Reply::Discard);
        (
            hand.iter()
                .copied()
                .filter(|&card| priority(card) == best)
                .collect(),
            best,
        )
    }

    pub(super) fn enforce_reply_obligations(self, phase: Phase) -> bool {
        match self {
            Self::AustrianSchnapsen => matches!(phase, Phase::EmptyTalon | Phase::ClosedTalon(_)),
        }
    }

    pub(super) fn draw_order(self, winner: Player, phase: Phase) -> Option<[Player; 2]> {
        match self {
            Self::AustrianSchnapsen if phase == Phase::OpenTalon => Some([winner, winner.other()]),
            Self::AustrianSchnapsen => None,
        }
    }

    pub(super) const fn last_trick_points(self) -> u8 {
        match self {
            Self::AustrianSchnapsen => 1,
        }
    }

    pub(super) const fn start_points(self) -> u8 {
        match self {
            Self::AustrianSchnapsen => 7,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleProfile {
    #[default]
    Soft,
    Sharp,
    ClassicSoft,
    ClassicSharp,
    TournamentSoft,
    TournamentSharp,
}

#[derive(Clone, Copy)]
pub(super) struct TalonActionRule {
    pub after_tricks: usize,
    pub minimum_cards: usize,
}

#[derive(Clone, Copy)]
enum GoingOut {
    Automatic,
    Manual,
}

#[derive(Clone, Copy)]
enum ClosingScore {
    AtClosing,
    AtEnd,
}

pub(super) struct ProfileRules {
    pub allow_ober_marriage: bool,
    pub require_trick_to_announce: bool,
    pub require_trick_for_announcement_eyes: bool,
    pub exchange: TalonActionRule,
    pub close: TalonActionRule,
    pub target_eyes: u8,
    going_out: GoingOut,
    closing_score: ClosingScore,
    allow_double_schneider: bool,
    review_after_finish: bool,
    show_own_eyes: bool,
    pub open_first_opponent_trick: bool,
    own_trick_review_limit: usize,
    opponent_trick_review_limit: usize,
    allow_legal_card_help: bool,
    allow_opponent_eyes_help: bool,
}

const HOUSE_TALON_ACTION: TalonActionRule = TalonActionRule {
    after_tricks: 1,
    minimum_cards: 4,
};
const CLASSIC_CLOSE: TalonActionRule = TalonActionRule {
    after_tricks: 0,
    minimum_cards: 4,
};
const SOFT: ProfileRules = ProfileRules {
    allow_ober_marriage: true,
    require_trick_to_announce: false,
    require_trick_for_announcement_eyes: true,
    exchange: HOUSE_TALON_ACTION,
    close: HOUSE_TALON_ACTION,
    target_eyes: 66,
    going_out: GoingOut::Automatic,
    closing_score: ClosingScore::AtClosing,
    allow_double_schneider: true,
    review_after_finish: false,
    show_own_eyes: true,
    open_first_opponent_trick: true,
    own_trick_review_limit: usize::MAX,
    opponent_trick_review_limit: 1,
    allow_legal_card_help: true,
    allow_opponent_eyes_help: true,
};

const SHARP: ProfileRules = ProfileRules {
    allow_ober_marriage: false,
    require_trick_to_announce: true,
    open_first_opponent_trick: false,
    own_trick_review_limit: 0,
    opponent_trick_review_limit: 0,
    ..SOFT
};

const CLASSIC_SOFT: ProfileRules = ProfileRules {
    exchange: TalonActionRule {
        after_tricks: 0,
        minimum_cards: 2,
    },
    close: CLASSIC_CLOSE,
    going_out: GoingOut::Manual,
    review_after_finish: true,
    show_own_eyes: false,
    open_first_opponent_trick: false,
    opponent_trick_review_limit: 0,
    allow_legal_card_help: false,
    allow_opponent_eyes_help: false,
    ..SOFT
};

const CLASSIC_SHARP: ProfileRules = ProfileRules {
    exchange: HOUSE_TALON_ACTION,
    allow_ober_marriage: false,
    require_trick_to_announce: true,
    own_trick_review_limit: 0,
    ..CLASSIC_SOFT
};

const TOURNAMENT_SOFT: ProfileRules = ProfileRules {
    closing_score: ClosingScore::AtEnd,
    allow_double_schneider: false,
    ..CLASSIC_SOFT
};

const TOURNAMENT_SHARP: ProfileRules = ProfileRules {
    closing_score: ClosingScore::AtEnd,
    allow_double_schneider: false,
    ..CLASSIC_SHARP
};

impl RuleProfile {
    pub(super) const fn rules(self) -> &'static ProfileRules {
        match self {
            Self::Soft => &SOFT,
            Self::Sharp => &SHARP,
            Self::ClassicSoft => &CLASSIC_SOFT,
            Self::ClassicSharp => &CLASSIC_SHARP,
            Self::TournamentSoft => &TOURNAMENT_SOFT,
            Self::TournamentSharp => &TOURNAMENT_SHARP,
        }
    }

    pub const fn manual_claim(self) -> bool {
        matches!(self.rules().going_out, GoingOut::Manual)
    }

    pub const fn allows_finished_trick_review(self) -> bool {
        self.rules().review_after_finish
    }

    pub const fn allows_own_eyes(self) -> bool {
        self.rules().show_own_eyes
    }

    pub const fn allows_double_schneider(self) -> bool {
        self.rules().allow_double_schneider
    }

    pub const fn scores_closure_at_end(self) -> bool {
        matches!(self.rules().closing_score, ClosingScore::AtEnd)
    }

    pub const fn allows_legal_card_help(self) -> bool {
        self.rules().allow_legal_card_help
    }

    pub const fn allows_opponent_eyes_help(self) -> bool {
        self.rules().allow_opponent_eyes_help
    }
}

impl ProfileRules {
    pub(super) fn review_limit(&self, owner: Player, finished: bool) -> usize {
        if finished && self.review_after_finish {
            return usize::MAX;
        }
        match owner {
            Player::Local => self.own_trick_review_limit,
            Player::Opponent => self.opponent_trick_review_limit,
        }
    }

    pub(super) fn winning_points(&self, opponent: Score) -> u8 {
        match opponent {
            Score { eyes: 33.., .. } => 1,
            Score { tricks: 0, .. } => 3,
            _ => 2,
        }
    }

    pub(super) fn successful_closure_points(&self, closure: TalonClosure, at_end: Score) -> u8 {
        self.winning_points(match self.closing_score {
            ClosingScore::AtClosing => closure.opponent_at_closing,
            ClosingScore::AtEnd => at_end,
        })
    }

    pub(super) fn false_claim_points(&self, opponent: Score, closure: Option<TalonClosure>) -> u8 {
        let penalty = if opponent.tricks == 0 { 3 } else { 2 };
        penalty.max(closure.map_or(0, |closed| self.failed_closure_points(closed)))
    }

    pub(super) fn failed_closure_points(&self, closure: TalonClosure) -> u8 {
        if closure.opponent_at_closing.tricks == 0 {
            3
        } else {
            2
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SchneiderRule {
    #[default]
    Double,
    Single,
}

impl SchneiderRule {
    pub(super) fn bummerls(self, loser_points: u8, start_points: u8) -> u8 {
        if self == Self::Double && loser_points == start_points {
            2
        } else {
            1
        }
    }
}

/// Stable technical choices for the strategy fixed to a match.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Difficulty {
    #[default]
    Easy,
    Medium,
    Hard,
}

impl Difficulty {
    pub const ALL: [Self; 3] = [Self::Easy, Self::Medium, Self::Hard];

    pub const fn id(self) -> &'static str {
        match self {
            Self::Easy => "easy",
            Self::Medium => "medium",
            Self::Hard => "hard",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|difficulty| difficulty.id() == id)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MatchConfig {
    pub variant: GameVariant,
    pub profile: RuleProfile,
    pub schneider: SchneiderRule,
    #[serde(default)]
    pub difficulty: Difficulty,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_variant_and_default_configuration_match_the_existing_game() {
        let config = MatchConfig::default();
        assert_eq!(GameVariant::ALL, [GameVariant::AustrianSchnapsen]);
        assert_eq!(config.variant, GameVariant::AustrianSchnapsen);
        assert_eq!(config.profile, RuleProfile::Soft);
        assert_eq!(config.schneider, SchneiderRule::Double);
        assert_eq!(config.difficulty, Difficulty::Easy);
        assert_eq!(
            config.variant.profiles(),
            [
                RuleProfile::Soft,
                RuleProfile::Sharp,
                RuleProfile::ClassicSoft,
                RuleProfile::ClassicSharp,
                RuleProfile::TournamentSoft,
                RuleProfile::TournamentSharp
            ]
        );
        assert_eq!(config.variant.deck(), full_deck());
        assert_eq!(config.variant.player_count(), 2);
        assert_eq!(config.variant.hand_size(), 5);
        assert_eq!(config.variant.start_points(), 7);
    }

    #[test]
    fn built_in_profiles_preserve_house_rules_and_visibility() {
        for profile in [RuleProfile::Soft, RuleProfile::Sharp] {
            let rules = profile.rules();
            let soft = profile == RuleProfile::Soft;
            assert_eq!(rules.allow_ober_marriage, soft);
            assert_eq!(rules.require_trick_to_announce, !soft);
            assert!(rules.require_trick_for_announcement_eyes);
            assert_eq!(rules.exchange.after_tricks, 1);
            assert_eq!(rules.exchange.minimum_cards, 4);
            assert_eq!(rules.target_eyes, 66);
            assert_eq!(rules.open_first_opponent_trick, soft);
            assert_eq!(
                rules.review_limit(Player::Local, false),
                if soft { usize::MAX } else { 0 }
            );
            assert_eq!(
                rules.review_limit(Player::Opponent, false),
                usize::from(soft)
            );
            assert!(profile.allows_legal_card_help());
            assert!(profile.allows_opponent_eyes_help());
        }
    }
}
