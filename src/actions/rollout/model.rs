use crate::model::game::GameState;

#[derive(Debug, Clone)]
pub struct RolloutOutcome {
    pub probability: f64,
    pub resulting_state: GameState,
}

impl RolloutOutcome {
    pub fn new(probability: f64, resulting_state: GameState) -> Self {
        RolloutOutcome {
            probability,
            resulting_state,
        }
    }
}
