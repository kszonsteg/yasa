use crate::mcts::evaluation::GameEvaluator;
use crate::mcts::node::{MCTSNode, NodeType};
use crate::model::action::Action;
use crate::model::enums::ActionType;
use crate::model::game::GameState;

pub struct MCTSTree {
    pub nodes: Vec<MCTSNode>,
    pub root_index: usize,
    pub exploration_constant: f64,
    evaluator: GameEvaluator,
}

impl MCTSTree {
    pub fn new(initial_state: GameState, exploration_constant: f64) -> Result<Self, String> {
        let root_node = MCTSNode::new_decision_node(initial_state, None)?;

        Ok(MCTSTree {
            nodes: vec![root_node],
            root_index: 0,
            exploration_constant,
            evaluator: GameEvaluator::new(),
        })
    }

    pub fn get_best_action(&self) -> Result<Action, String> {
        let root = &self.nodes[self.root_index];

        if root.children.is_empty() {
            return Err("No children available".to_string());
        }

        // Try to get the child with highest average score (excluding nodes with 0 visits)
        let best_with_visits = root
            .children
            .iter()
            .filter(|(_, &child_index)| self.nodes[child_index].visits > 0)
            .max_by(|(_, &child_index_a), (_, &child_index_b)| {
                let node_a = &self.nodes[child_index_a];
                let node_b = &self.nodes[child_index_b];

                let avg_score_a = node_a.total_score / node_a.visits as f64;
                let avg_score_b = node_b.total_score / node_b.visits as f64;

                avg_score_a
                    .partial_cmp(&avg_score_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

        if let Some((action, _)) = best_with_visits {
            // Found a child with visits > 0, return the best scoring one
            Ok(action.clone())
        } else {
            // All children have 0 visits, pick a random action
            let random_index = fastrand::usize(0..root.children.len());
            let random_action = root.children.keys().nth(random_index).unwrap();
            Ok(random_action.clone())
        }
    }

    pub fn select(&self, node_index: usize) -> usize {
        let node = &self.nodes[node_index];

        if node.is_terminal {
            return node_index;
        }

        match node.node_type {
            NodeType::Decision => {
                if !node.is_fully_expanded() {
                    return node_index;
                }

                // Select child with highest UCB1 value
                let mut best_child_index = node_index;
                let mut best_ucb1 = f64::NEG_INFINITY;

                for (action, &child_index) in &node.children {
                    // Skip EndTurn actions if the child node already has at least 1 visit
                    // UNLESS it's the only child available
                    let is_end_turn = action.action_type() == ActionType::EndTurn;
                    let should_skip = is_end_turn
                        && self.nodes[child_index].visits >= 1
                        && node.children.len() > 1;

                    if should_skip {
                        continue;
                    }

                    let ucb1 = self.nodes[child_index]
                        .get_ucb1_value(self.exploration_constant, node.visits);
                    if ucb1 > best_ucb1 {
                        best_ucb1 = ucb1;
                        best_child_index = child_index;
                    }
                }

                self.select(best_child_index)
            }
            NodeType::Chance => {
                // For chance nodes, we need to select based on the random outcome
                // In practice, this would be determined by the actual dice roll
                // For MCTS purposes, we select the outcome probabilistically
                let node = &self.nodes[node_index];

                if node.chance_children.is_empty() {
                    return node_index; // Need expansion
                }

                let random_value = fastrand::f64();
                let mut cumulative_prob = 0.0;

                for &child_index in &node.chance_children {
                    cumulative_prob += self.nodes[child_index].chance_probability;
                    if random_value <= cumulative_prob {
                        return self.select(child_index);
                    }
                }

                // Fallback to last child if probabilities don't sum to 1.0
                let last_child = *node.chance_children.last().unwrap();
                self.select(last_child)
            }
        }
    }

    pub fn expand(&mut self, node_index: usize) -> Result<usize, String> {
        let node = &self.nodes[node_index];

        match node.node_type {
            NodeType::Decision => self.expand_decision_node(node_index),
            NodeType::Chance => self.expand_chance_node(node_index),
        }
    }

    fn expand_decision_node(&mut self, node_index: usize) -> Result<usize, String> {
        // Check if expansion is possible before borrowing
        if self.nodes[node_index].untried_actions.is_empty() {
            return Err(format!(
                "Cannot expand fully expanded node - index: {}, is_terminal: {}, untried_actions: {}, procedure: {:?}",
                node_index,
                self.nodes[node_index].is_terminal,
                self.nodes[node_index].untried_actions.len(),
                self.nodes[node_index].state.procedure
            ));
        }

        if self.nodes[node_index].is_terminal {
            return Err(format!(
                "Cannot expand terminal node - index: {}, procedure: {:?}",
                node_index, self.nodes[node_index].state.procedure
            ));
        }

        let action = self.nodes[node_index].untried_actions.remove(0);
        let current_state: GameState = self.nodes[node_index].state.clone();

        // TODO: Apply the action to create the new state
        let child_node = MCTSNode::new_decision_node(current_state, Some(node_index))?;
        let child_index = self.nodes.len();
        self.nodes.push(child_node);

        // Add the child to the parent's children map
        self.nodes[node_index].children.insert(action, child_index);

        Ok(child_index)
    }

    fn expand_chance_node(&mut self, _chance_node_index: usize) -> Result<usize, String> {
        todo!()
    }

    pub fn evaluate(&self, node_index: usize) -> Result<f64, String> {
        let node = &self.nodes[node_index];

        match node.node_type {
            NodeType::Decision => self.evaluator.evaluate(&node.state),
            NodeType::Chance => {
                // For chance nodes, weight the simulation by outcome probabilities
                if node.chance_children.is_empty() {
                    // If no children, use evaluator on current state
                    self.evaluator.evaluate(&node.state)
                } else {
                    let mut weighted_score = 0.0;
                    for &child_index in &node.chance_children {
                        let child = &self.nodes[child_index];
                        let child_score = self.evaluate(child_index)?;
                        weighted_score += child_score * child.chance_probability;
                    }
                    Ok(weighted_score)
                }
            }
        }
    }

    pub fn backpropagate(&mut self, mut node_index: usize, score: f64) {
        loop {
            let adjusted_score = match self.nodes[node_index].node_type {
                NodeType::Decision => score,
                NodeType::Chance => score * self.nodes[node_index].chance_probability,
            };

            self.nodes[node_index].add_visit(adjusted_score);

            if let Some(parent_index) = self.nodes[node_index].parent {
                node_index = parent_index;
            } else {
                break; // Reached root
            }
        }
    }
}
