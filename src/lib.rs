use pyo3::{
    exceptions::{PyNotImplementedError, PyValueError},
    prelude::*,
};

use crate::model::game::GameState;

pub mod model;

#[pyfunction]
fn get_actions(state: &str) -> PyResult<String> {
    let game_state = GameState::from_json(state);
    match game_state {
        Ok(_game_state) => Err(PyNotImplementedError::new_err("Not implemented")),
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
