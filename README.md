# YASA (Yet Another Search Algorithm)

**YASA** is a hybrid **Rust/Python** AI agent designed for the board game **Blood Bowl** (specifically for the [`botbowl`](https://github.com/njustesen/botbowl) framework). It leverages a **Monte Carlo Tree Search (MCTS)** algorithm implemented in Rust coupled with a **Neural Network (Value Network)** for evaluating game states.

- [YASA (Yet Another Search Algorithm)](#yasa-yet-another-search-algorithm)
  - [Tech Stack](#tech-stack)
  - [Repository Structure](#repository-structure)
  - [Development](#development)
    - [Prerequisites](#prerequisites)
    - [Setup](#setup)
    - [Running Tests](#running-tests)
    - [Code Quality \& Pre-commit Hooks](#code-quality--pre-commit-hooks)
    - [Commit Messages](#commit-messages)
  - [Roadmap](#roadmap)

## Tech Stack

- **Core Logic (Rust):** MCTS engine, game state management, and fast evaluation.
  - **Core Libraries:** `pyo3` (Python bindings), `tract-onnx` (Inference).
- **Machine Learning and framework interaction (Python):** Value Network for state evaluation and interaction with Rust and botbowl.
  - **Frameworks:** `PyTorch Lightning`, `torch`, `safetensors`, `onnx`, `botbowl`.
- **Build & Tooling:**
  - **Build System:** `maturin` (builds Rust extension for Python).
  - **Dependency Management:** `uv` (Python), `cargo` (Rust).

## Repository Structure

The project is structured to separate the Rust core from the Python machine learning and interface layers.

- **`src/` (Rust Core)**
  - `mcts/`: Monte Carlo Tree Search implementation.
  - `model/`: Rust representations of Blood Bowl concepts (Game, Ball, Player, Team).
  - `actions/`: Logic for generating and executing game actions.
  - `pathfinding/`: A* pathfinding.
  - `lib.rs`: Entry point for the `yasa_core` Python extension.

- **`python/` (Python Interface & ML)**
  - `yasa/`: Main Python package and `botbowl` interface.
  - `nn/value_network/`: Neural Network training, modeling, and data generation.
  - `bots/`: Other bots implementations.

- **`benches/`**: Rust benchmarks for critical paths.
- **`tests/`**: Integration and unit tests.

## Development

### Prerequisites

- **Rust:** Stable toolchain (`cargo`).
- **Python:** 3.10.
- **Tools:**
  - [`uv`](https://github.com/astral-sh/uv) (for fast Python package management).
  - `maturin` (usually installed via `uv`).

### Setup

1. **Install Dependencies:**

    ```bash
    uv sync --group dev
    ```

2. **Build Rust Extension (Development Mode):**
    This builds the Rust code and installs it into the current virtual environment. Run this whenever you modify code in `src/`.

    ```bash
    uv run maturin develop
    ```

### Running Tests

- **Rust Unit and Integration Tests:**

    ```bash
    cargo test
    ```

- **Python Tests:**
    Ensure the extension is built first.

    ```bash
    uv run maturin develop && uv run pytest python/tests
    ```

### Code Quality & Pre-commit Hooks

This project uses `pre-commit` to ensure code quality and consistency.

1. **Install Hooks:**

    ```bash
    uv run pre-commit install --hook-type pre-commit --hook-type commit-msg
    ```

2. **Hooks Configured:**
    - **Formatting:** `cargo fmt` (Rust), `ruff-format` (Python).
    - **Linting:** `cargo clippy` (Rust), `ruff-check` (Python).
    - **Testing:** `cargo test` runs on commit.
    - **Conventions:** Enforces **Conventional Commits** messages.

### Commit Messages

We adhere to [Conventional Commits](https://www.conventionalcommits.org/). Please format your commit messages as follows:

```text
<type>(<scope>): <description>

[optional body]
```

Examples:

- `feat(mcts): add node expansion logic`
- `fix(rules): correct passing interference modifier`
- `chore: update dependencies`

## Roadmap

Based on the current backlog, the following features are planned:

- **Ball Mechanics:** Pushing a player to a square with a ball should result in a bounce and generally ball bounces on pick-ups, handoffs etc.
- **Knock Downs:** Implement full effects of knockdowns beyond just marking as knocked out.
- **Skills:** Implement the base skill effects.
- **Field Bounds:** Handle pushing players out of bounds correctly.
- **Handoff**: Allow agent to perform handoff actions
- **Pass**: Allow agent to perform pass actions
- **Foul**: Allow agent to perform foul actions
