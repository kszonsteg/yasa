"""
Blood Bowl Value Network Model.

This module contains the CNN architecture for evaluating Blood Bowl game states.
The network predicts value scores for both home and away teams in the range [-1, 1].
"""

from typing import Any

import pytorch_lightning as pl
import torch
import torch.nn as nn


class ValueNetworkModule(pl.LightningModule):
    """
    PyTorch Lightning module for Blood Bowl game state evaluation.

    This fully connected network evaluates game states and predicts value scores
    for both home and away teams. It combines spatial features (board state) with
    non-spatial features (game context) to produce predictions.

    Architecture:
        - Flattened spatial input concatenated with non-spatial features
        - 2 fully connected layers (256 -> 64) with ReLU and Dropout
        - Output layer with tanh activation for outputs in [-1, 1]
        - Output shape: (N, 2) where [0] is Home value, [1] is Away value

    Attributes:
        BOARD_HEIGHT: Height of the Blood Bowl board (17).
        BOARD_WIDTH: Width of the Blood Bowl board (28).
        DEFAULT_SPATIAL_LAYERS: Default number of spatial input channels (27).
        DEFAULT_NON_SPATIAL_FEATURES: Default number of non-spatial features (15).
    """

    # Board dimensions
    BOARD_HEIGHT: int = 17
    BOARD_WIDTH: int = 28

    # Default feature dimensions
    DEFAULT_SPATIAL_LAYERS: int = 27
    DEFAULT_NON_SPATIAL_FEATURES: int = 15

    def __init__(
        self,
        num_spatial_layers: int = DEFAULT_SPATIAL_LAYERS,
        num_non_spatial_features: int = DEFAULT_NON_SPATIAL_FEATURES,
        learning_rate: float = 1e-3,
        weight_decay: float = 1e-5,
        lr_scheduler_patience: int = 5,
        lr_scheduler_factor: float = 0.5,
        lr_scheduler_min_lr: float = 1e-7,
    ):
        """
        Initialize the ValueNetworkModule.

        Args:
            num_spatial_layers: Number of channels in the spatial input.
            num_non_spatial_features: Number of features in the non-spatial input.
            learning_rate: Initial learning rate for the optimizer.
            weight_decay: Weight decay (L2 regularization) coefficient.
            lr_scheduler_patience: Epochs with no improvement before reducing LR.
            lr_scheduler_factor: Factor by which to reduce LR.
            lr_scheduler_min_lr: Minimum learning rate.
        """
        super().__init__()
        self.save_hyperparameters()

        # Store hyperparameters as instance attributes for easy access
        self.num_spatial_layers = num_spatial_layers
        self.num_non_spatial_features = num_non_spatial_features
        self.learning_rate = learning_rate
        self.weight_decay = weight_decay
        self.lr_scheduler_patience = lr_scheduler_patience
        self.lr_scheduler_factor = lr_scheduler_factor
        self.lr_scheduler_min_lr = lr_scheduler_min_lr

        # Build network architecture
        self._build_network()

        # Loss function
        self.criterion = nn.MSELoss()

        # For tracking best validation loss
        self.best_val_loss = float("inf")

    def _build_network(self) -> None:
        """Build the neural network architecture.

        Architecture optimized for fast Rust inference:
        - Small 1x1 conv to reduce spatial channels (27 -> 16)
        - Single 3x3 conv for local spatial patterns (16 -> 32)
        - Global average pooling to reduce spatial dims (28*17 -> 1)
        - Small FC layers (32 + 15 -> 64 -> 2)

        Total params: ~6k (very fast inference)
        """
        # Spatial feature reduction with 1x1 conv (27 -> 16 channels)
        self.spatial_reduce = nn.Sequential(
            nn.Conv2d(self.num_spatial_layers, 16, kernel_size=1),
            nn.ReLU(inplace=True),
        )

        # Local spatial pattern extraction with 3x3 conv
        self.spatial_conv = nn.Sequential(
            nn.Conv2d(16, 32, kernel_size=3, padding=1),
            nn.ReLU(inplace=True),
        )

        # Global average pooling reduces (32, W, H) -> (32,)
        self.global_pool = nn.AdaptiveAvgPool2d(1)

        # FC head: spatial features (32) + non-spatial (15) -> output
        fc_input_size = 32 + self.num_non_spatial_features
        self.fc_head = nn.Sequential(
            nn.Linear(fc_input_size, 64),
            nn.ReLU(inplace=True),
            nn.Linear(64, 2),
        )

    def forward(
        self, spatial_input: torch.Tensor, non_spatial_input: torch.Tensor
    ) -> torch.Tensor:
        """
        Perform the forward pass through the network.

        Args:
            spatial_input: Tensor of shape (N, C, W, H) representing spatial layers.
                          C = num_spatial_layers, W = BOARD_WIDTH, H = BOARD_HEIGHT.
            non_spatial_input: Tensor of shape (N, F) representing non-spatial features.
                              F = num_non_spatial_features.

        Returns:
            Tensor of shape (N, 2) with value scores in [-1, 1] for [Home, Away].
        """
        # Process spatial features
        x = self.spatial_reduce(spatial_input)  # (N, 16, W, H)
        x = self.spatial_conv(x)  # (N, 32, W, H)
        x = self.global_pool(x)  # (N, 32, 1, 1)
        x = x.view(x.size(0), -1)  # (N, 32)

        # Concatenate with non-spatial features
        combined = torch.cat([x, non_spatial_input], dim=1)  # (N, 47)

        # FC head with tanh output
        return torch.tanh(self.fc_head(combined))

    def _compute_metrics(
        self,
        outputs: torch.Tensor,
        labels: torch.Tensor,
    ) -> dict[str, torch.Tensor]:
        """Compute all metrics for logging."""
        loss = self.criterion(outputs, labels)

        # Mean absolute error (scalar output)
        mae = torch.mean(torch.abs(outputs - labels))

        # Separate metrics for Home and Away
        mae_home = torch.mean(torch.abs(outputs[:, 0] - labels[:, 0]))
        mae_away = torch.mean(torch.abs(outputs[:, 1] - labels[:, 1]))

        return {
            "loss": loss,
            "mae": mae,
            "mae_home": mae_home,
            "mae_away": mae_away,
        }

    def _log_metrics(self, metrics: dict[str, torch.Tensor], stage: str) -> None:
        """Log metrics to the logger."""
        for name, value in metrics.items():
            prog_bar = name == "loss"
            self.log(
                f"{stage}/{name}",
                value,
                prog_bar=prog_bar,
                on_step=False,
                on_epoch=True,
                sync_dist=True,
            )

    def training_step(
        self, batch: tuple[torch.Tensor, torch.Tensor, torch.Tensor], batch_idx: int
    ) -> torch.Tensor:
        """Training step."""
        spatial_input, non_spatial_input, labels = batch
        outputs = self(spatial_input, non_spatial_input)
        metrics = self._compute_metrics(outputs, labels)
        self._log_metrics(metrics, "train")
        return metrics["loss"]

    def validation_step(
        self, batch: tuple[torch.Tensor, torch.Tensor, torch.Tensor], batch_idx: int
    ) -> torch.Tensor:
        """Validation step."""
        spatial_input, non_spatial_input, labels = batch
        outputs = self(spatial_input, non_spatial_input)
        metrics = self._compute_metrics(outputs, labels)
        self._log_metrics(metrics, "val")
        return metrics["loss"]

    def test_step(
        self, batch: tuple[torch.Tensor, torch.Tensor, torch.Tensor], batch_idx: int
    ) -> torch.Tensor:
        """Test step."""
        spatial_input, non_spatial_input, labels = batch
        outputs = self(spatial_input, non_spatial_input)
        metrics = self._compute_metrics(outputs, labels)
        self._log_metrics(metrics, "test")
        return metrics["loss"]

    def configure_optimizers(self) -> dict[str, Any]:
        """Configure optimizer and learning rate scheduler."""
        optimizer = torch.optim.AdamW(
            self.parameters(),
            lr=self.learning_rate,
            weight_decay=self.weight_decay,
        )
        scheduler = torch.optim.lr_scheduler.ReduceLROnPlateau(
            optimizer,
            mode="min",
            factor=self.lr_scheduler_factor,
            patience=self.lr_scheduler_patience,
            min_lr=self.lr_scheduler_min_lr,
        )
        return {
            "optimizer": optimizer,
            "lr_scheduler": {
                "scheduler": scheduler,
                "monitor": "val/loss",
                "interval": "epoch",
                "frequency": 1,
            },
        }

    def on_validation_epoch_end(self) -> None:
        """Track best validation loss."""
        val_loss = self.trainer.callback_metrics.get("val/loss")
        if val_loss is not None and val_loss < self.best_val_loss:
            self.best_val_loss = val_loss.item()

    @classmethod
    def load_for_inference(
        cls,
        checkpoint_path: str,
        device: str | torch.device = "cpu",
    ) -> "ValueNetworkModule":
        """
        Load a model from checkpoint for inference.

        Args:
            checkpoint_path: Path to the .ckpt checkpoint file.
            device: Device to load the model to.

        Returns:
            Model in evaluation mode ready for inference.
        """
        model = cls.load_from_checkpoint(
            checkpoint_path, map_location=device, weights_only=False
        )
        model.eval()
        model.freeze()
        return model

    @torch.inference_mode()
    def predict(
        self,
        spatial_input: torch.Tensor,
        non_spatial_input: torch.Tensor,
    ) -> torch.Tensor:
        """
        Make predictions on input data.

        Args:
            spatial_input: Spatial features tensor.
            non_spatial_input: Non-spatial features tensor.

        Returns:
            Tensor of shape (N, 1) with value in [-1, 1].
        """
        return self(spatial_input, non_spatial_input)
