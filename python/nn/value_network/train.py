"""
Training script for Blood Bowl Value Network.

This script provides a CLI interface for training the value network using
PyTorch Lightning with TensorBoard logging and production-ready checkpointing.
"""

import argparse
import sys
from datetime import datetime
from pathlib import Path

import pytorch_lightning as pl
import torch
from pytorch_lightning.callbacks import (
    EarlyStopping,
    LearningRateMonitor,
    ModelCheckpoint,
    RichProgressBar,
)
from pytorch_lightning.loggers import TensorBoardLogger

sys.path.append("/app/python")

from nn.value_network.dataset import BloodBowlDataModule
from nn.value_network.model import ValueNetworkModule


def parse_args() -> argparse.Namespace:
    """Parse command-line arguments."""
    parser = argparse.ArgumentParser(
        description="Train Blood Bowl Value Network",
        formatter_class=argparse.ArgumentDefaultsHelpFormatter,
    )

    # Data arguments
    data_group = parser.add_argument_group("Data")
    data_group.add_argument(
        "--data-dir",
        type=str,
        default="data",
        help="Directory containing .jsonl log files",
    )
    data_group.add_argument(
        "--batch-size",
        type=int,
        default=32,
        help="Batch size for training",
    )
    data_group.add_argument(
        "--val-split",
        type=float,
        default=0.1,
        help="Fraction of data to use for validation",
    )
    data_group.add_argument(
        "--test-split",
        type=float,
        default=0.0,
        help="Fraction of data to use for testing",
    )
    data_group.add_argument(
        "--num-workers",
        type=int,
        default=0,
        help="Number of data loader workers",
    )

    # Model arguments
    model_group = parser.add_argument_group("Model")
    model_group.add_argument(
        "--learning-rate",
        "--lr",
        type=float,
        default=1e-3,
        help="Initial learning rate",
    )
    model_group.add_argument(
        "--weight-decay",
        type=float,
        default=1e-5,
        help="Weight decay (L2 regularization)",
    )
    model_group.add_argument(
        "--lr-patience",
        type=int,
        default=5,
        help="Patience for learning rate scheduler",
    )
    model_group.add_argument(
        "--lr-factor",
        type=float,
        default=0.5,
        help="Factor for learning rate reduction",
    )

    # Training arguments
    training_group = parser.add_argument_group("Training")
    training_group.add_argument(
        "--max-epochs",
        type=int,
        default=100,
        help="Maximum number of training epochs",
    )
    training_group.add_argument(
        "--early-stopping-patience",
        type=int,
        default=15,
        help="Early stopping patience (0 to disable)",
    )
    training_group.add_argument(
        "--gradient-clip-val",
        type=float,
        default=1.0,
        help="Gradient clipping value (0 to disable)",
    )
    training_group.add_argument(
        "--accumulate-grad-batches",
        type=int,
        default=1,
        help="Number of batches to accumulate gradients over",
    )
    training_group.add_argument(
        "--precision",
        type=str,
        default="32",
        choices=["16-mixed", "bf16-mixed", "32"],
        help="Training precision",
    )

    # Matmul precision for float32 on Tensor Cores
    training_group.add_argument(
        "--matmul-precision",
        type=str,
        default="high",
        choices=["highest", "high", "medium"],
        help=(
            "torch.set_float32_matmul_precision level used when running with CUDA and"
            " precision=32. Using 'high' or 'medium' leverages Tensor Cores for better performance."
        ),
    )

    # Output arguments
    output_group = parser.add_argument_group("Output")
    output_group.add_argument(
        "--output-dir",
        type=str,
        default="artifacts",
        help="Directory for saving models and logs",
    )
    output_group.add_argument(
        "--experiment-name",
        type=str,
        default="value_network",
        help="Name for the experiment (used in logging)",
    )
    output_group.add_argument(
        "--save-top-k",
        type=int,
        default=3,
        help="Number of best checkpoints to keep",
    )

    # Reproducibility
    parser.add_argument(
        "--seed",
        type=int,
        default=42,
        help="Random seed for reproducibility",
    )

    # Resume training
    parser.add_argument(
        "--resume-from",
        type=str,
        default=None,
        help="Path to checkpoint to resume training from",
    )

    return parser.parse_args()


def create_callbacks(
    output_dir: Path,
    early_stopping_patience: int,
    save_top_k: int,
) -> list:
    """Create training callbacks."""
    callbacks = []

    # Checkpoint callback for the best models based on validation loss
    callbacks.append(
        ModelCheckpoint(
            dirpath=output_dir / "checkpoints",
            filename="best-{epoch:03d}-{val/loss:.4f}",
            monitor="val/loss",
            mode="min",
            save_top_k=save_top_k,
            save_last=True,
            auto_insert_metric_name=False,
            save_weights_only=False,
            verbose=True,
        )
    )

    # Checkpoint callback for periodic saves (every 10 epochs)
    callbacks.append(
        ModelCheckpoint(
            dirpath=output_dir / "checkpoints",
            filename="periodic-{epoch:03d}",
            every_n_epochs=10,
            save_top_k=-1,  # Keep all periodic checkpoints
            auto_insert_metric_name=False,
            save_weights_only=False,
        )
    )

    # Learning rate monitor
    callbacks.append(LearningRateMonitor(logging_interval="epoch"))

    # Progress bar
    callbacks.append(RichProgressBar(refresh_rate=1))

    # Early stopping
    if early_stopping_patience > 0:
        callbacks.append(
            EarlyStopping(
                monitor="val/loss",
                patience=early_stopping_patience,
                mode="min",
                verbose=True,
                min_delta=1e-6,
            )
        )

    return callbacks


def main() -> None:
    """Main training function."""
    args = parse_args()

    # Set seed for reproducibility
    pl.seed_everything(args.seed, workers=True)

    # Resolve paths with timestamp for unique run identification
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    data_dir = Path(args.data_dir)
    data_file = data_dir / "merged.jsonl"
    output_dir = Path(args.output_dir) / f"{args.experiment_name}_{timestamp}"
    output_dir.mkdir(parents=True, exist_ok=True)

    # Initialize data module
    data_module = BloodBowlDataModule(
        data_file=data_file,
        batch_size=args.batch_size,
        val_split=args.val_split,
        test_split=args.test_split,
        num_workers=args.num_workers,
        seed=args.seed,
    )

    # Initialize model
    model = ValueNetworkModule(
        learning_rate=args.learning_rate,
        weight_decay=args.weight_decay,
        lr_scheduler_patience=args.lr_patience,
        lr_scheduler_factor=args.lr_factor,
    )

    # Create callbacks
    callbacks = create_callbacks(
        output_dir=output_dir,
        early_stopping_patience=args.early_stopping_patience,
        save_top_k=args.save_top_k,
    )

    # Set up a logger
    logger = TensorBoardLogger(
        save_dir=output_dir,
        name="logs",
        default_hp_metric=False,
        log_graph=True,
    )

    # Log hyperparameters
    logger.log_hyperparams(vars(args))

    # Determine accelerator
    if torch.cuda.is_available():
        accelerator = "gpu"
    elif torch.backends.mps.is_available():
        accelerator = "mps"
    else:
        accelerator = "cpu"

    # Configure matmul precision early to leverage Tensor Cores and silence warnings
    if torch.cuda.is_available() and args.precision == "32":
        try:
            torch.set_float32_matmul_precision(args.matmul_precision)
        except Exception as e:
            print(f"Warning: failed to set float32 matmul precision: {e}")

    # Initialize trainer
    trainer = pl.Trainer(
        max_epochs=args.max_epochs,
        accelerator=accelerator,
        devices=1,
        precision=args.precision,
        gradient_clip_val=args.gradient_clip_val
        if args.gradient_clip_val > 0
        else None,
        accumulate_grad_batches=args.accumulate_grad_batches,
        callbacks=callbacks,
        logger=logger,
        deterministic=True,
        enable_progress_bar=True,
        log_every_n_steps=10,
        val_check_interval=1.0,
        enable_model_summary=True,
    )

    # Print training configuration
    print(f"\n{'=' * 60}")
    print("Blood Bowl Value Network Training")
    print(f"{'=' * 60}")
    print(f"Run ID: {args.experiment_name}_{timestamp}")
    print(f"Data directory: {data_dir.absolute()}")
    print(f"Output directory: {output_dir.absolute()}")
    print(f"Accelerator: {accelerator}")
    print(f"Precision: {args.precision}")
    if torch.cuda.is_available() and args.precision == "32":
        print(f"Float32 matmul precision: {args.matmul_precision}")
    print(f"Batch size: {args.batch_size}")
    print(f"Learning rate: {args.learning_rate}")
    print(f"Max epochs: {args.max_epochs}")
    print(f"Early stopping patience: {args.early_stopping_patience}")
    print(f"{'=' * 60}\n")

    # Train the model
    trainer.fit(model, datamodule=data_module, ckpt_path=args.resume_from)

    # Test if test split is configured
    if args.test_split > 0:
        print("\nRunning test evaluation...")
        trainer.test(model, datamodule=data_module)

    # Print final results
    best_checkpoint = callbacks[0]  # The first callback is the best model checkpoint
    print(f"\n{'=' * 60}")
    print("Training complete!")
    print(f"{'=' * 60}")
    print(f"Best model checkpoint: {best_checkpoint.best_model_path}")
    print(f"Best validation loss: {best_checkpoint.best_model_score:.6f}")
    print(f"Last checkpoint: {output_dir / 'checkpoints' / 'last.ckpt'}")
    print(f"TensorBoard logs: {output_dir / 'logs'}")
    print("\nTo view TensorBoard, run:")
    print(f"  tensorboard --logdir {output_dir / 'logs'}")
    print(f"{'=' * 60}\n")


if __name__ == "__main__":
    main()
