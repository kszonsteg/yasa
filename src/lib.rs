use pyo3::{exceptions::PyValueError, prelude::*};

use crate::actions::core::registry::ActionRegistry;
use crate::model::game::GameState;
use serde_json::json;

pub mod actions;
pub mod model;

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
            // Return a standardized JSON object: { "actions": [...] }
            let wrapper = json!({"actions": actions});
            match serde_json::to_string(&wrapper) {
                Ok(s) => Ok(s),
                Err(e) => Err(PyValueError::new_err(format!("{e}"))),
            }
        }
        Err(e) => Err(PyValueError::new_err(format!(
            "Invalid game state provided: {e}"
        ))),
    }
}

/// A Python module implemented in Rust.
#[pymodule]
fn yasa_core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(get_actions, m)?)?;
    Ok(())
}
