use crate::model::game::GameState;

pub struct GameEvaluator;

impl GameEvaluator {
    pub fn new() -> Self {
        GameEvaluator
    }

    /// Evaluate a game state from the perspective of the current team.
    /// Returns a score in the range [-1.0, 1.0] where:
    /// - 1.0 = definitely winning
    /// - 0.0 = draw
    /// - -1.0 = definitely losing
    pub fn evaluate(&self, _state: &GameState) -> Result<f64, String> {
        Ok(0.0)
    }
}

impl Default for GameEvaluator {
    fn default() -> Self {
        Self::new()
    }
}
