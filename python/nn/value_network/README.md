# Blood Bowl Value Network

A production-ready PyTorch Lightning implementation of a CNN-based value network for evaluating Blood Bowl game states. Designed for use with Monte Carlo Tree Search (MCTS) agents.

## Overview

The value network evaluates Blood Bowl game states and predicts the probability of each team scoring a touchdown. This prediction guides the MCTS algorithm towards more promising game states.

## Project Structure

```
value_network/
├── __init__.py          # Package exports
├── model.py             # ValueNetworkModule (PyTorch Lightning)
├── dataset.py           # Dataset and DataModule for loading game states
├── train.py             # Training CLI with production checkpointing
├── export.py            # Model export utilities (ONNX, TorchScript, PyTorch)
├── generate_data.py     # Script to generate training data from bot games
└── README.md            # This file
```

## Installation

```bash
# Using uv (recommended)
uv sync --group nn

# Or using pip
pip install -e ".[nn]"
```

## Quick Start

### 1. Generate Training Data

```bash
python -m nn.value_network.generate_data --num-games 100
```

### 2. Train the Model

```bash
python -m nn.value_network.train \
    --data-dir logs \
    --batch-size 64 \
    --max-epochs 100 \
    --learning-rate 0.001
```

### 3. Monitor Training

```bash
tensorboard --logdir artifacts/
```

### 4. Export the Model

```bash
# Export to all formats
python -m nn.value_network.export artifacts/value_network_*/checkpoints/best-*.ckpt --format all

# Export to specific format
python -m nn.value_network.export checkpoint.ckpt --format onnx
```

## Checkpointing

The training script implements production-ready checkpointing:

| Checkpoint Type | Location | Description |
|-----------------|----------|-------------|
| Best Models | `checkpoints/best-*.ckpt` | Top K models by validation loss |
| Last Model | `checkpoints/last.ckpt` | Most recent checkpoint |
| Periodic | `checkpoints/periodic-*.ckpt` | Every 10 epochs |

### Loading Checkpoints

```python
from nn import ValueNetworkModule

# For inference (frozen, eval mode)
model = ValueNetworkModule.load_for_inference("checkpoint.ckpt", device="cuda")
predictions = model.predict(spatial_input, non_spatial_input)

# For fine-tuning or inspection
model = ValueNetworkModule.load_from_checkpoint("checkpoint.ckpt")
```

### Resuming Training

```bash
python -m nn.value_network.train --resume-from artifacts/value_network_*/checkpoints/last.ckpt
```

## Architecture

### Input

| Input | Shape | Description |
|-------|-------|-------------|
| Spatial | `(N, 27, 17, 28)` | Board state as feature layers |
| Non-spatial | `(N, 15)` | Global game context |

### Network Structure

```
Spatial Input (27 × 17 × 28)
    │
    └── Conv Block
        ├── Conv2d(27→64) + BatchNorm + ReLU
        ├── Conv2d(64→128) + BatchNorm + ReLU
        └── Conv2d(128→256) + BatchNorm + ReLU
        └── Flatten → (121856,)
                │
                ├── Concatenate ← Non-Spatial (15,)
                │
                └── FC Block
                    ├── Linear(121871→1024) + ReLU + Dropout(0.1)
                    └── Linear(1024→256) + ReLU + Dropout(0.1)
                    │
                    └── Output Layer
                        └── Linear(256→2) + Sigmoid
                              │
                              └── [home_td_prob, away_td_prob]
```

### Output

Tensor of shape `(N, 2)` with probabilities in `[0, 1]`:
- Index 0: Home team touchdown probability
- Index 1: Away team touchdown probability

## Training Configuration

| Argument | Default | Description |
|----------|---------|-------------|
| `--data-dir` | `logs` | Directory with .jsonl training data |
| `--batch-size` | `32` | Training batch size |
| `--val-split` | `0.1` | Validation data fraction |
| `--learning-rate` | `0.001` | Initial learning rate |
| `--weight-decay` | `1e-5` | L2 regularization |
| `--max-epochs` | `100` | Maximum training epochs |
| `--early-stopping-patience` | `15` | Early stopping patience |
| `--gradient-clip-val` | `1.0` | Gradient clipping value |
| `--precision` | `32` | Training precision (16-mixed, bf16-mixed, 32) |
| `--save-top-k` | `3` | Number of best checkpoints to keep |

## Export Formats

| Format | Extension | Use Case |
|--------|-----------|----------|
| ONNX | `.onnx` | Cross-platform inference, tract-onnx (Rust) |
| SafeTensors | `.safetensors` | Candle (Rust) native loading |
| TorchScript | `.torchscript.pt` | C++ deployment, mobile |
| PyTorch | `.pth` | Python inference, fine-tuning |

### Rust Integration

The model can be used from Rust with two backend options:

```bash
# Export for Rust inference
python -m nn.value_network.export checkpoint.ckpt --format onnx safetensors
```

#### Using tract-onnx (ONNX backend)
```rust
use yasa_core::mcts::evaluation::TractValuePolicy;

let policy = TractValuePolicy::from_path("model.onnx")?;
let probability = policy.evaluate(&game_state)?;
```

#### Using candle (native Rust backend)
```rust
use yasa_core::mcts::evaluation::CandleValuePolicy;

let policy = CandleValuePolicy::from_path("model.safetensors", &Device::Cpu)?;
let probability = policy.evaluate(&game_state)?;
```

#### Performance Benchmark
```bash
cargo bench --bench value_policy_benchmark
```

## Programmatic Usage

```python
from nn import ValueNetworkModule, BloodBowlDataModule

# Training
model = ValueNetworkModule(learning_rate=0.001, weight_decay=1e-5)
data_module = BloodBowlDataModule(data_dir="logs", batch_size=32)

trainer = pl.Trainer(max_epochs=100)
trainer.fit(model, datamodule=data_module)

# Inference
model = ValueNetworkModule.load_for_inference("best.ckpt")
predictions = model.predict(spatial_input, non_spatial_input)
# predictions: [[home_prob, away_prob], ...]
```

## Metrics Logged

| Metric | Description |
|--------|-------------|
| `train/loss`, `val/loss` | MSE loss |
| `train/home_loss`, `val/home_loss` | MSE for home team predictions |
| `train/away_loss`, `val/away_loss` | MSE for away team predictions |
| `train/mae`, `val/mae` | Mean absolute error |
