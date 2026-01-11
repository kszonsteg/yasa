use pyo3::{exceptions::PyValueError, prelude::*};

use crate::actions::core::registry::ActionRegistry;
use crate::mcts::evaluation::HeuristicValuePolicy;
use crate::mcts::search::MCTSSearch;
use crate::model::game::GameState;
use serde_json::json;

pub mod actions;
pub mod mcts;
pub mod model;
pub mod pathfinding;

#[pyfunction]
fn get_actions(state: &str) -> PyResult<String> {
    let game_state = GameState::from_json(state);
    match game_state {
        Ok(mut game_state) => {
            let action_registry = ActionRegistry::new();
            let result = action_registry.discover_actions(&mut game_state);
            match result {
                Ok(_) => (),
                Err(e) => return Err(PyValueError::new_err(e.to_string())),
            }
            let actions = game_state.available_actions;
            let wrapper = json!({"actions": actions});
            match serde_json::to_string(&wrapper) {
                Ok(s) => Ok(s),
                Err(e) => Err(PyValueError::new_err(e.to_string())),
            }
        }
        Err(e) => Err(PyValueError::new_err(format!(
            "Invalid game state provided: {e}"
        ))),
    }
}

#[pyfunction]
fn get_mcts_action(state: &str, time_limit: u64, terminal: bool) -> PyResult<String> {
    let game_state = GameState::from_json(state);
    match game_state {
        Ok(game_state) => {
            let mut mcts = MCTSSearch::with_config(1.4, time_limit);
            let result = if terminal {
                mcts.search_terminal(game_state)
            } else {
                mcts.search(game_state)
            };

            match result {
                Ok(action) => {
                    let wrapper = json!({"action": action});
                    match serde_json::to_string(&wrapper) {
                        Ok(s) => Ok(s),
                        Err(e) => Err(PyValueError::new_err(e.to_string())),
                    }
                }
                Err(e) => Err(PyValueError::new_err(e.to_string())),
            }
        }
        Err(e) => Err(PyValueError::new_err(e.to_string())),
    }
}

#[pyfunction]
fn evaluate_state_heuristic(state: &str) -> PyResult<(f64, f64)> {
    let mut game_state = GameState::from_json(state)
        .map_err(|e| PyValueError::new_err(format!("Invalid game state: {e}")))?;

    let heuristic = HeuristicValuePolicy::new().map_err(PyValueError::new_err)?;

    let home_id = game_state
        .home_team
        .as_ref()
        .map(|t| t.team_id.clone())
        .ok_or_else(|| PyValueError::new_err("No home team"))?;

    let away_id = game_state
        .away_team
        .as_ref()
        .map(|t| t.team_id.clone())
        .ok_or_else(|| PyValueError::new_err("No away team"))?;

    // Evaluate for Home
    game_state.current_team_id = Some(home_id);
    let home_val = heuristic
        .evaluate(&game_state)
        .map_err(PyValueError::new_err)?;

    // Evaluate for Away
    game_state.current_team_id = Some(away_id);
    let away_val = heuristic
        .evaluate(&game_state)
        .map_err(PyValueError::new_err)?;

    Ok((home_val, away_val))
}

/// A Python module implemented in Rust.
#[pymodule]
fn yasa_core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(get_actions, m)?)?;
    m.add_function(wrap_pyfunction!(get_mcts_action, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_state_heuristic, m)?)?;
    Ok(())
}
