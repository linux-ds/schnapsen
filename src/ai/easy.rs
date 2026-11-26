use super::{choose_card, choose_marriage, wants_exchange, Action, Observation, Strategy};

pub(super) struct Easy;

impl Strategy for Easy {
    fn decide(&self, view: &Observation) -> Option<Action> {
        if view.exchange.is_some_and(wants_exchange) {
            return Some(Action::Exchange);
        }
        if let Some(marriage) = choose_marriage(&view.legal_marriages) {
            return Some(Action::Announce {
                suit: marriage.suit,
                card: marriage.king(),
            });
        }
        if let Some(&(_, lead)) = view.played.first() {
            let decisive = view.legal_cards.iter().copied().find(|&card| {
                view.beats(card, lead)
                    && view.own_eyes + view.own_pending_eyes + lead.points() + card.points()
                        >= view.target_eyes()
            });
            if let Some(card) = decisive {
                return Some(Action::Play(card));
            }
        }
        choose_card(&view.legal_cards).map(Action::Play)
    }
}
