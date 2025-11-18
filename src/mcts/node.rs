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

#[derive(Debug, Clone)]
pub struct MCTSNode {
    pub state: GameState,
    pub node_type: NodeType,
    pub parent: Option<usize>,
    pub children: HashMap<Action, usize>,
    pub chance_children: Vec<usize>,
    pub visits: u32,
    pub total_score: f64,
    pub untried_actions: Vec<Action>,
    pub is_terminal: bool,
    pub chance_probability: f64,
}

impl MCTSNode {
    pub fn add_visit(&mut self, score: f64) {
        self.visits += 1;
        self.total_score += score;
    }

    pub fn get_ucb1_value(&self, exploration_constant: f64, parent_visits: u32) -> f64 {
        if self.visits == 0 {
            f64::INFINITY
        } else {
            let exploitation = self.total_score / self.visits as f64;
            let exploration =
                exploration_constant * ((parent_visits as f64).ln() / self.visits as f64).sqrt();
            exploitation + exploration
        }
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
            children: HashMap::new(),
            chance_children: Vec::new(),
            visits: 0,
            total_score: 0.0,
            untried_actions: Vec::new(),
            is_terminal: false,
            chance_probability: probability,
        }
    }

    pub fn new_decision_node(state: GameState, parent: Option<usize>) -> Result<Self, String> {
        let mut node = MCTSNode {
            state: state.clone(),
            node_type: NodeType::Decision,
            parent,
            children: HashMap::new(),
            chance_children: Vec::new(),
            visits: 0,
            total_score: 0.0,
            untried_actions: Vec::new(),
            is_terminal: false,
            chance_probability: 1.0,
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
