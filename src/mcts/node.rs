use crate::model::action::Action;
use crate::model::enums::{ActionType, Procedure};
use crate::model::game::GameState;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum NodeType {
    Decision,
    Chance,
}

#[derive(Debug, Clone)]
pub struct ChanceOutcome {
    pub probability: f64,
    pub resulting_state: GameState,
}

/// Represents a node within the Monte Carlo Tree Search (MCTS) structure.
///
/// Each `MCTSNode` contains information about the state within the game tree,
/// as well as metadata required for performing the MCTS algorithm, such as visits,
/// total score, and connections to parent and child nodes.
///
/// # Fields
/// - `state` (GameState): The game state associated with this node.
/// - `node_type` (NodeType): The type of node (Decision or Chance), which determines
///   how the node is treated during the MCTS process.
/// - `parent` (Option<usize>): The index of the parent node, if any. A value of `None` indicates
///   that this is the root node.
/// - `decision_children` (HashMap<Action, usize>): A mapping from actions to the indices of child nodes,
///   representing the explored moves from this state.
/// - `untried_actions` (Vec<Action>): A list of actions that have not yet been explored from
///   this node. Expanding these actions results in new child nodes.
/// - `chance_children` (Vec<usize>): A list of child indices for chance nodes, used to handle
///   probabilistic outcomes from random actions, such as dice rolls.
/// - `chance_probability` (f64): The probability of reaching to this node.
/// - `visits` (u32): The number of times this node has been visited during simulations. This
///   value is used to calculate action-selection criteria such as UCT.
/// - `total_score` (f64): The total reward accumulated through this node across all its visits.
///   This value is used to compute the average score during the decision-making process.
/// - `is_terminal` (bool): A flag indicating whether this node represents a terminal state in
///   the search.
#[derive(Debug, Clone)]
pub struct MCTSNode {
    pub state: GameState,
    pub node_type: NodeType,
    pub parent: Option<usize>,
    pub decision_children: HashMap<Action, usize>,
    pub untried_actions: Vec<Action>,
    pub chance_children: Vec<usize>,
    pub chance_probability: f64,
    pub visits: u32,
    pub total_score: f64,
    pub is_terminal: bool,
}

impl MCTSNode {
    pub fn add_visit(&mut self, score: f64) {
        self.visits += 1;
        self.total_score += score;
    }

    pub fn get_ucb1_value(&self, exploration_constant: f64, parent_visits: u32) -> f64 {
        if self.visits == 0 {
            return f64::INFINITY;
        }

        let parent_visits_f = parent_visits as f64;
        let exploration_term = if parent_visits == 0 {
            exploration_constant * (1.0f64 / (self.visits as f64)).sqrt()
        } else {
            exploration_constant * ((parent_visits_f.ln() / self.visits as f64).sqrt())
        };

        let exploitation = self.total_score / self.visits as f64;
        exploitation + exploration_term
    }

    pub fn is_fully_expanded(&self) -> bool {
        match self.node_type {
            NodeType::Decision => self.is_terminal || self.untried_actions.is_empty(),
            NodeType::Chance => !self.chance_children.is_empty(), // Chance nodes expand all outcomes at once
        }
    }

    pub fn new_chance_node(state: GameState, parent: Option<usize>, probability: f64) -> Self {
        MCTSNode {
            state,
            node_type: NodeType::Chance,
            parent,
            decision_children: HashMap::new(),
            untried_actions: Vec::new(),
            chance_children: Vec::new(),
            chance_probability: probability,
            visits: 0,
            total_score: 0.0,
            is_terminal: false,
        }
    }

    pub fn new_decision_node(
        state: GameState,
        parent: Option<usize>,
        probability: f64,
    ) -> Result<Self, String> {
        let mut node = MCTSNode {
            state: state.clone(),
            node_type: NodeType::Decision,
            parent,
            decision_children: HashMap::new(),
            untried_actions: Vec::new(),
            chance_probability: probability,
            chance_children: Vec::new(),
            visits: 0,
            total_score: 0.0,
            is_terminal: false,
        };

        node.untried_actions = state
            .available_actions
            .iter()
            // tree pruning with not supported actions.
            // TODO: add support for Blitz and Handoff at minimum.
            .filter(|action| {
                ![
                    ActionType::StartBlitz,
                    ActionType::StartPass,
                    ActionType::StartHandoff,
                    ActionType::StartFoul,
                ]
                .contains(&action.action_type())
            })
            .cloned()
            .collect();
        node.is_terminal = node.untried_actions.is_empty()
            || state.game_over
            // The end of turn is terminal.
            || state.procedure == Some(Procedure::EndTurn)
            || state.procedure == Some(Procedure::Touchdown)
            || state.procedure == Some(Procedure::Turnover);

        Ok(node)
    }
}
