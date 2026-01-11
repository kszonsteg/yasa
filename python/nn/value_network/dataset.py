"""
Blood Bowl Dataset for Value Network Training.

This module provides the dataset and data module classes for loading
and preprocessing Blood Bowl game states for a single scalar value network
output in the range [-1, 1].
"""

import json
from pathlib import Path
from typing import Any

import numpy as np
import pytorch_lightning as pl
import torch
from torch.utils.data import DataLoader, Dataset, random_split


class BloodBowlDataset(Dataset):
    """
    Dataset for loading Blood Bowl game states for a value network.

    This dataset is designed to train a network that predicts value scores
    for both home and away teams in [-1, 1].

    Labels (per state):
        - Index 0: Home team value (mixed outcome + heuristic)
        - Index 1: Away team value (mixed outcome + heuristic)

    Additional features:
        - Absolute Representation: The board is represented from a fixed perspective,
          with home team features always in the same channels and away team in others.
        - Loads from merged.jsonl file for efficient storage and loading.
    """

    # Board dimensions
    BOARD_HEIGHT = 17
    BOARD_WIDTH = 28

    # Number of spatial layers per team
    LAYERS_PER_TEAM = 13

    # Weather types for one-hot encoding (must match Rust WeatherType enum names)
    WEATHER_TYPES = [
        "NICE",
        "VERY_SUNNY",
        "POURING_RAIN",
        "BLIZZARD",
        "SWELTERING_HEAT",
    ]

    def __init__(self, data_file: Path):
        """
        Initialize the dataset.

        Args:
            data_file: Path to the merged.jsonl file containing labeled game states.
        """
        self.samples: list[dict] = []
        print(f"Loading dataset from {data_file}...")
        with open(data_file) as f:
            for line in f:
                line = line.strip()
                if line:
                    self.samples.append(json.loads(line))
        print(f"Dataset loaded. Found {len(self.samples)} training samples.")

    def __len__(self) -> int:
        return len(self.samples)

    def __getitem__(self, idx: int) -> tuple[torch.Tensor, torch.Tensor, torch.Tensor]:
        state = self.samples[idx]

        spatial_input, non_spatial_input = self.parse_game_state(state)

        # Load dual labels
        label_home = state.get("label_home", 0.0)
        label_away = state.get("label_away", 0.0)

        # Shape (2,)
        label = torch.tensor([label_home, label_away], dtype=torch.float32)

        return spatial_input, non_spatial_input, label

    @classmethod
    def parse_game_state(
        cls, game_state: dict[str, Any]
    ) -> tuple[torch.Tensor, torch.Tensor]:
        """
        Parse a game state into spatial and non-spatial tensors.

        The board is represented from a fixed "home vs away" perspective.
        Spatial tensor shape is (C, W, H) where:
        - C = 27 channels
        - W = 28 (BOARD_WIDTH, x-axis)
        - H = 17 (BOARD_HEIGHT, y-axis)

        Args:
            game_state: Dictionary containing the game state.

        Returns:
            Tuple of (spatial_input, non_spatial_input) tensors.
        """
        # Shape: (C, W, H) - Width first, then Height
        spatial_layers = np.zeros(
            (27, cls.BOARD_WIDTH, cls.BOARD_HEIGHT), dtype=np.float32
        )

        # Layer 0: Ball Position
        if game_state.get("balls") and game_state["balls"][0].get("position"):
            ball_pos = game_state["balls"][0]["position"]
            # Index as [channel, x, y] for (C, W, H) layout
            spatial_layers[0, ball_pos["x"], ball_pos["y"]] = 1

        def process_team(team_data: dict[str, Any], layer_offset: int) -> None:
            """Process a team's players into spatial layers.

            Uses (x, y) indexing for (C, W, H) layout.
            """
            for player in team_data.get("players_by_id", {}).values():
                if player.get("position"):
                    x, y = player["position"]["x"], player["position"]["y"]

                    # Player position - index as [channel, x, y]
                    spatial_layers[layer_offset + 0, x, y] = 1

                    # Attributes (normalized would be better, but kept as-is for compatibility)
                    spatial_layers[layer_offset + 1, x, y] = player["ma"]
                    spatial_layers[layer_offset + 2, x, y] = player["st"]
                    spatial_layers[layer_offset + 3, x, y] = player["ag"]
                    spatial_layers[layer_offset + 4, x, y] = player["av"]

                    # State flags
                    state = player.get("state", {})
                    spatial_layers[layer_offset + 5, x, y] = (
                        1 if state.get("up", False) else 0
                    )
                    spatial_layers[layer_offset + 6, x, y] = (
                        1 if state.get("used", False) else 0
                    )
                    spatial_layers[layer_offset + 7, x, y] = (
                        1 if state.get("stunned", False) else 0
                    )

                    # Skills
                    skills = player.get("skills", [])
                    spatial_layers[layer_offset + 8, x, y] = (
                        1 if "BLOCK" in skills else 0
                    )
                    spatial_layers[layer_offset + 9, x, y] = (
                        1 if "DODGE" in skills else 0
                    )
                    spatial_layers[layer_offset + 10, x, y] = (
                        1 if "SURE_HANDS" in skills else 0
                    )
                    spatial_layers[layer_offset + 11, x, y] = (
                        1 if "CATCH" in skills else 0
                    )
                    spatial_layers[layer_offset + 12, x, y] = (
                        1 if "PASS" in skills else 0
                    )

        # Process teams (home at offset 1, away at offset 14)
        process_team(game_state.get("home_team", {}), layer_offset=1)
        process_team(game_state.get("away_team", {}), layer_offset=14)

        # Non-spatial features
        home_team = game_state.get("home_team", {})
        away_team = game_state.get("away_team", {})

        non_spatial_features = [
            float(game_state.get("half", 1)),
            float(game_state.get("round", 0)),
            float(home_team.get("rerolls", 0)),
            float(home_team.get("score", 0)),
            float(away_team.get("rerolls", 0)),
            float(away_team.get("score", 0)),
        ]

        # Turn state features
        turn_state = game_state.get("turn_state")
        if turn_state:
            non_spatial_features.extend(
                [
                    1.0 if turn_state.get("blitz_available", False) else 0.0,
                    1.0 if turn_state.get("pass_available", False) else 0.0,
                    1.0 if turn_state.get("handoff_available", False) else 0.0,
                    1.0 if turn_state.get("foul_available", False) else 0.0,
                ]
            )
        else:
            non_spatial_features.extend([0.0, 0.0, 0.0, 0.0])

        # Weather one-hot encoding
        weather = game_state.get("weather", "NICE")
        weather_encoding = [1.0 if w == weather else 0.0 for w in cls.WEATHER_TYPES]
        non_spatial_features.extend(weather_encoding)

        spatial_input = torch.from_numpy(spatial_layers)
        non_spatial_input = torch.tensor(non_spatial_features, dtype=torch.float32)

        return spatial_input, non_spatial_input


class BloodBowlDataModule(pl.LightningDataModule):
    """
    PyTorch Lightning DataModule for Blood Bowl game state data.

    Handles data loading, splitting, and DataLoader creation with a proper configuration for training efficiency.
    """

    def __init__(
        self,
        data_file: str | Path,
        batch_size: int = 32,
        val_split: float = 0.1,
        test_split: float = 0.0,
        num_workers: int = 0,
        seed: int = 42,
    ):
        """
        Initialize the data module.

        Args:
            data_file: Path to the merged.jsonl file.
            batch_size: Batch size for DataLoaders.
            val_split: Fraction of data to use for validation (0 to 1).
            test_split: Fraction of data to use for testing (0 to 1).
            num_workers: Number of workers for DataLoaders.
            seed: Random seed for reproducible splits.
        """
        super().__init__()
        self.save_hyperparameters()

        self.data_file = Path(data_file)
        self.batch_size = batch_size
        self.val_split = val_split
        self.test_split = test_split
        self.num_workers = num_workers
        self.seed = seed

        self.train_dataset: Dataset | None = None
        self.val_dataset: Dataset | None = None
        self.test_dataset: Dataset | None = None

    def setup(self, stage: str | None = None) -> None:
        """
        Set up datasets for each stage.

        Args:
            stage: Either 'fit', 'validate', 'test', or 'predict'.
        """
        if self.train_dataset is not None:
            return  # Already set up

        # Create a full dataset
        full_dataset = BloodBowlDataset(data_file=self.data_file)

        # Calculate split sizes
        total_size = len(full_dataset)
        test_size = int(total_size * self.test_split)
        val_size = int(total_size * self.val_split)
        train_size = total_size - val_size - test_size

        # Ensure at least 1 sample in each non-empty split
        if val_size > 0:
            val_size = max(1, val_size)
        if test_size > 0:
            test_size = max(1, test_size)
        train_size = total_size - val_size - test_size

        # Perform split
        generator = torch.Generator().manual_seed(self.seed)

        if test_size > 0 and val_size > 0:
            self.train_dataset, self.val_dataset, self.test_dataset = random_split(
                full_dataset, [train_size, val_size, test_size], generator=generator
            )
        elif val_size > 0:
            self.train_dataset, self.val_dataset = random_split(
                full_dataset, [train_size, val_size], generator=generator
            )
            self.test_dataset = None
        else:
            self.train_dataset = full_dataset
            self.val_dataset = None
            self.test_dataset = None

        print(f"Dataset splits: train={train_size}, val={val_size}, test={test_size}")

    def train_dataloader(self) -> DataLoader:
        """Create training DataLoader."""
        return DataLoader(
            self.train_dataset,
            batch_size=self.batch_size,
            shuffle=True,
            num_workers=self.num_workers,
            pin_memory=True,
            persistent_workers=self.num_workers > 0,
        )

    def val_dataloader(self) -> DataLoader | None:
        """Create validation DataLoader."""
        if self.val_dataset is None:
            return None
        return DataLoader(
            self.val_dataset,
            batch_size=self.batch_size,
            shuffle=False,
            num_workers=self.num_workers,
            pin_memory=True,
            persistent_workers=self.num_workers > 0,
        )

    def test_dataloader(self) -> DataLoader | None:
        """Create test DataLoader."""
        if self.test_dataset is None:
            return None
        return DataLoader(
            self.test_dataset,
            batch_size=self.batch_size,
            shuffle=False,
            num_workers=self.num_workers,
            pin_memory=True,
            persistent_workers=self.num_workers > 0,
        )
