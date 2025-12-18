use super::tree::MCTSTree;
use crate::model::action::Action;
use crate::model::game::GameState;
use std::time::{Duration, Instant};

pub struct MCTSSearch {
    pub exploration_constant: f64,
    pub time_limit: Duration,
    pub iterations: usize,
}

impl Default for MCTSSearch {
    fn default() -> Self {
        Self::new()
    }
}

impl MCTSSearch {
    pub fn new() -> Self {
        MCTSSearch {
            exploration_constant: 1.4, // Standard UCB1 exploration constant
            time_limit: Duration::from_millis(1000), // 1-second default
            iterations: 0,
        }
    }

    pub fn with_config(exploration_constant: f64, time_limit_ms: u64) -> Self {
        MCTSSearch {
            exploration_constant,
            time_limit: Duration::from_millis(time_limit_ms),
            iterations: 0,
        }
    }

    pub fn search(&mut self, initial_state: GameState) -> Result<Action, String> {
        let mut tree = MCTSTree::new(initial_state, self.exploration_constant)?;
        let start_time = Instant::now();

        while start_time.elapsed() < self.time_limit {
            let selected_node = tree.select(tree.root_index);
            if tree.nodes[selected_node].is_terminal {
                let score = tree.evaluate(selected_node)?;
                tree.backpropagate(selected_node, score);
            } else {
                let expanded_node = tree.expand(selected_node)?;
                let score = tree.evaluate(expanded_node)?;
                tree.backpropagate(expanded_node, score);
            }
            self.iterations += 1;
        }

        tree.get_best_action()
    }
}
