use crate::actions::core::registry::ActionRegistry;
use crate::mcts::evaluation::HeuristicValuePolicy;
use crate::mcts::node::{MCTSNode, NodeType};
use crate::model::action::Action;
use crate::model::enums::{ActionType, Procedure};
use crate::model::game::GameState;

pub struct MCTSTree {
    pub nodes: Vec<MCTSNode>,
    pub root_index: usize,
    pub exploration_constant: f64,
    evaluator: HeuristicValuePolicy,
    action_registry: ActionRegistry,
}

impl MCTSTree {
    pub fn new(initial_state: GameState, exploration_constant: f64) -> Result<Self, String> {
        let mut state = initial_state.clone();
        let action_registry = ActionRegistry::new();
        action_registry.discover_actions(&mut state)?;
        let root_node = MCTSNode::new_decision_node(state, None, 1.0)?;
        let evaluator = HeuristicValuePolicy::new();
        match evaluator {
            Err(_) => Err("Failed to initialize evaluator".to_string()),
            Ok(ev) => Ok(MCTSTree {
                nodes: vec![root_node],
                root_index: 0,
                exploration_constant,
                evaluator: ev,
                action_registry,
            }),
        }
    }

    pub fn get_best_action(&self) -> Result<Action, String> {
        let root = &self.nodes[self.root_index];

        if root.decision_children.is_empty() {
            return Err("No children available".to_string());
        }

        // Try to get the child with the highest average score (excluding nodes with 0 visits)
        let best_with_visits = root
            .decision_children
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
            let random_index = fastrand::usize(0..root.decision_children.len());
            let random_action = root.decision_children.keys().nth(random_index).unwrap();
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

                for (action, &child_index) in &node.decision_children {
                    // Skip EndTurn actions if the child node already has at least 1 visit
                    // UNLESS it's the only child available
                    let is_end_turn = action.action_type() == ActionType::EndTurn;
                    let should_skip = is_end_turn
                        && self.nodes[child_index].visits >= 1
                        && node.decision_children.len() > 1;

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

                // Fallback to the last child if probabilities don't sum to 1.0
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
        let mut current_state: GameState = self.nodes[node_index].state.clone();
        self.action_registry
            .execute_action(&mut current_state, &action)?;

        // If the action has a random outcome, create chance nodes instead of a decision node
        if self.is_random_procedure(&current_state.procedure)? {
            let chance_node = MCTSNode::new_chance_node(current_state, Some(node_index), 1.0);
            let child_index = self.nodes.len();
            self.nodes.push(chance_node);
            self.nodes[node_index]
                .decision_children
                .insert(action, child_index);
            Ok(child_index)
        } else {
            self.action_registry.discover_actions(&mut current_state)?;
            let child_node = MCTSNode::new_decision_node(current_state, Some(node_index), 1.0)?;
            let child_index = self.nodes.len();
            self.nodes.push(child_node);
            self.nodes[node_index]
                .decision_children
                .insert(action, child_index);
            Ok(child_index)
        }
    }

    fn expand_chance_node(&mut self, chance_node_index: usize) -> Result<usize, String> {
        // Get procedure outcomes from the action registry
        let state = self.nodes[chance_node_index].state.clone();
        let outcomes = self.action_registry.rollout_chance_outcomes(&state)?;

        if outcomes.is_empty() {
            return Err("No outcomes from procedure execution".to_string());
        }

        let mut first_child_index = None;

        for outcome in outcomes {
            let mut resulting_state = outcome.resulting_state;
            let child_node: MCTSNode = if self.is_random_procedure(&resulting_state.procedure)? {
                MCTSNode::new_chance_node(
                    resulting_state,
                    Some(chance_node_index),
                    outcome.probability,
                )
            } else {
                // Discover available actions for the resulting state before creating decision node
                self.action_registry
                    .discover_actions(&mut resulting_state)?;
                MCTSNode::new_decision_node(
                    resulting_state,
                    Some(chance_node_index),
                    outcome.probability,
                )?
            };

            let child_index = self.nodes.len();
            self.nodes.push(child_node);
            self.nodes[chance_node_index]
                .chance_children
                .push(child_index);

            if first_child_index.is_none() {
                first_child_index = Some(child_index);
            }
        }

        first_child_index.ok_or("Failed to create any child nodes".to_string())
    }

    pub fn evaluate(&self, node_index: usize) -> Result<f64, String> {
        let node = &self.nodes[node_index];

        match node.node_type {
            NodeType::Decision => {
                // For decision nodes, if no children yet, use evaluator on current state
                if node.decision_children.is_empty() {
                    self.evaluator.evaluate(&node.state)
                } else {
                    // Aggregate child evaluations (simple average)
                    let mut total = 0.0;
                    let mut count = 0usize;
                    for &child_index in node.decision_children.values() {
                        let child_score = self.evaluate(child_index)?;
                        total += child_score;
                        count += 1;
                    }
                    if count == 0 {
                        self.evaluator.evaluate(&node.state)
                    } else {
                        Ok(total / count as f64)
                    }
                }
            }
            NodeType::Chance => {
                // For chance nodes, weight the simulation by outcome probabilities
                if node.chance_children.is_empty() {
                    // If no children, use evaluator on the current state
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
            self.nodes[node_index].add_visit(score);

            if let Some(parent_index) = self.nodes[node_index].parent {
                node_index = parent_index;
            } else {
                break; // Reached root
            }
        }
    }

    fn is_random_procedure(&self, procedure: &Option<Procedure>) -> Result<bool, String> {
        match procedure {
            Some(Procedure::BlockRoll) | Some(Procedure::GFI) | Some(Procedure::Dodge) => Ok(true),
            None => Err("Missing procedure in MCTS expansion".to_string()),
            _ => Ok(false),
        }
    }

    /// Generates a Mermaid flowchart representation of the MCTS tree.
    ///
    /// # Arguments
    /// * `depth` - Maximum depth to traverse (0 means unlimited depth)
    ///
    /// # Returns
    /// A string containing the Mermaid flowchart syntax
    pub fn generate_mermaid_graph(&self, depth: usize) -> String {
        let mut graph = String::from("flowchart LR\n");
        let mut visited = std::collections::HashSet::new();

        self.generate_mermaid_node(&mut graph, &mut visited, self.root_index, 0, depth);

        graph
    }

    /// Recursively generates Mermaid nodes and connections
    fn generate_mermaid_node(
        &self,
        graph: &mut String,
        visited: &mut std::collections::HashSet<usize>,
        node_index: usize,
        current_depth: usize,
        max_depth: usize,
    ) {
        // Check depth limit (0 means unlimited)
        if max_depth > 0 && current_depth >= max_depth {
            return;
        }

        // Avoid infinite loops in the case of cycles
        if visited.contains(&node_index) {
            return;
        }
        visited.insert(node_index);

        let node = &self.nodes[node_index];

        // Generate node definition with the appropriate shape and information
        let node_label = self.format_node_label(node, node_index);
        let node_shape = match node.node_type {
            NodeType::Decision => format!("    {}[\"{}\"]", self.node_id(node_index), node_label),
            NodeType::Chance => format!("    {}([\"{}\"])", self.node_id(node_index), node_label),
        };
        graph.push_str(&node_shape);
        graph.push('\n');

        // Add terminal node styling if needed
        if node.is_terminal {
            graph.push_str(&format!(
                "    {} --> Terminal{{Terminal}}\n",
                self.node_id(node_index)
            ));
        }

        // Process decision node children (connected by actions)
        for (action, &child_index) in &node.decision_children {
            let action_label = self.format_action_label(action);
            graph.push_str(&format!(
                "    {} -->|\"{}\"| {}\n",
                self.node_id(node_index),
                action_label,
                self.node_id(child_index)
            ));

            self.generate_mermaid_node(graph, visited, child_index, current_depth + 1, max_depth);
        }

        // Process chance node children (connected by probabilities)
        for &child_index in &node.chance_children {
            let child_node = &self.nodes[child_index];
            let prob_label = format!("{:.1}", child_node.chance_probability * 100.0);
            graph.push_str(&format!(
                "    {} -->|\"{}%\"| {}\n",
                self.node_id(node_index),
                prob_label,
                self.node_id(child_index)
            ));

            self.generate_mermaid_node(graph, visited, child_index, current_depth + 1, max_depth);
        }
    }

    /// Formats the node label with relevant information
    fn format_node_label(&self, node: &MCTSNode, node_index: usize) -> String {
        let node_type_str = match node.node_type {
            NodeType::Decision => "Decision",
            NodeType::Chance => "Chance",
        };

        let avg_score = if node.visits > 0 {
            format!("{:.3}", node.total_score / node.visits as f64)
        } else {
            "N/A".to_string()
        };

        if node_index == self.root_index {
            format!(
                "Root {} Node Visits: {} Avg Score: {} Procedure {:?}",
                node_type_str,
                node.visits,
                avg_score,
                node.state.procedure.unwrap()
            )
        } else {
            format!(
                "{} Node Visits: {} Avg Score: {} Procedure {:?}",
                node_type_str,
                node.visits,
                avg_score,
                node.state.procedure.unwrap()
            )
        }
    }

    /// Formats action labels for edge descriptions
    fn format_action_label(&self, action: &Action) -> String {
        let action_name = format!("{:?}", action.action_type());

        let player_info = match action.player() {
            Some(uuid) => {
                // Show the last 8 characters of UUID for readability
                if uuid.len() > 8 {
                    format!("Player:{}", &uuid[uuid.len().saturating_sub(8)..])
                } else {
                    format!("Player:{uuid}")
                }
            }
            None => "No Player".to_string(),
        };

        let position_info = match action.position() {
            Some(square) => format!("Pos:({}, {})", square.x, square.y),
            None => "No Position".to_string(),
        };

        format!("{action_name} | {player_info} | {position_info}")
    }

    /// Generates a unique node ID for Mermaid
    fn node_id(&self, node_index: usize) -> String {
        format!("N{node_index}")
    }
}
