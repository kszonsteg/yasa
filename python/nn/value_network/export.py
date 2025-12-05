"""
Export utilities for Blood Bowl Value Network.

This module provides functionality to export trained Lightning models to various
formats including ONNX, TorchScript, PyTorch state dict, and SafeTensors.
"""

import argparse
from enum import Enum
from pathlib import Path
from typing import Any

import torch

from nn.value_network.model import ValueNetworkModule


class ExportFormat(str, Enum):
    """Supported export formats."""

    ONNX = "onnx"
    TORCHSCRIPT = "torchscript"
    PYTORCH = "pytorch"
    SAFETENSORS = "safetensors"
    ALL = "all"


def export_to_onnx(
    model: ValueNetworkModule,
    output_path: Path,
    opset_version: int = 17,
    dynamic_batch: bool = True,
) -> Path:
    """
    Export model to ONNX format.

    Args:
        model: The ValueNetworkModule to export.
        output_path: Path for the output .onnx file.
        opset_version: ONNX opset version to use.
        dynamic_batch: Whether to use dynamic batch size.

    Returns:
        Path to the exported ONNX model.
    """
    print(f"Exporting to ONNX format (opset {opset_version})...")

    model.eval()
    model.to("cpu")

    # Create dummy inputs with correct shapes (C, W, H)
    dummy_spatial = torch.randn(
        1,
        model.num_spatial_layers,
        model.BOARD_WIDTH,
        model.BOARD_HEIGHT,
        device="cpu",
    )
    dummy_non_spatial = torch.randn(
        1,
        model.num_non_spatial_features,
        device="cpu",
    )

    # Configure dynamic axes
    dynamic_axes: dict[str, dict[int, str]] | None = None
    if dynamic_batch:
        dynamic_axes = {
            "spatial_input": {0: "batch_size"},
            "non_spatial_input": {0: "batch_size"},
            "output": {0: "batch_size"},
        }

    output_path.parent.mkdir(parents=True, exist_ok=True)

    torch.onnx.export(
        model,
        (dummy_spatial, dummy_non_spatial),
        str(output_path),
        input_names=["spatial_input", "non_spatial_input"],
        output_names=["output"],
        dynamic_axes=dynamic_axes,
        opset_version=opset_version,
        do_constant_folding=True,
    )

    print(f"ONNX model saved to: {output_path}")
    return output_path


def export_to_torchscript(
    model: ValueNetworkModule,
    output_path: Path,
    method: str = "trace",
) -> Path:
    """
    Export model to TorchScript format.

    Args:
        model: The ValueNetworkModule to export.
        output_path: Path for the output .pt file.
        method: Export method - 'trace' or 'script'.

    Returns:
        Path to the exported TorchScript model.
    """
    print(f"Exporting to TorchScript format (method: {method})...")

    model.eval()
    model.to("cpu")

    # Create dummy inputs for tracing
    dummy_spatial = torch.randn(
        1,
        model.num_spatial_layers,
        model.BOARD_HEIGHT,
        model.BOARD_WIDTH,
        device="cpu",
    )
    dummy_non_spatial = torch.randn(
        1,
        model.num_non_spatial_features,
        device="cpu",
    )

    output_path.parent.mkdir(parents=True, exist_ok=True)

    if method == "trace":
        scripted_model = torch.jit.trace(model, (dummy_spatial, dummy_non_spatial))
    elif method == "script":
        scripted_model = torch.jit.script(model)
    else:
        raise ValueError(f"Unknown method: {method}. Use 'trace' or 'script'.")

    scripted_model.save(str(output_path))

    print(f"TorchScript model saved to: {output_path}")
    return output_path


def export_to_pytorch(
    model: ValueNetworkModule,
    output_path: Path,
) -> Path:
    """
    Export model to PyTorch state dict format with metadata.

    Args:
        model: The ValueNetworkModule to export.
        output_path: Path for the output .pth file.

    Returns:
        Path to the exported PyTorch model.
    """
    print("Exporting to PyTorch state dict format...")

    model.eval()
    model.to("cpu")

    output_path.parent.mkdir(parents=True, exist_ok=True)

    save_dict: dict[str, Any] = {
        "model_state_dict": model.state_dict(),
        "hparams": dict(model.hparams),
        "model_config": {
            "num_spatial_layers": model.num_spatial_layers,
            "num_non_spatial_features": model.num_non_spatial_features,
            "board_height": model.BOARD_HEIGHT,
            "board_width": model.BOARD_WIDTH,
        },
    }
    torch.save(save_dict, output_path)

    print(f"PyTorch model saved to: {output_path}")
    return output_path


def export_to_safetensors(
    model: ValueNetworkModule,
    output_path: Path,
) -> Path:
    """
    Export model to SafeTensors format for Candle/Rust inference.

    This format is optimized for loading in Rust using the candle library.
    The tensor names are preserved from PyTorch for compatibility.

    Args:
        model: The ValueNetworkModule to export.
        output_path: Path for the output .safetensors file.

    Returns:
        Path to the exported SafeTensors model.
    """
    try:
        from safetensors.torch import save_file
    except ImportError:
        raise ImportError(
            "safetensors package is required for SafeTensors export. "
            "Install with: pip install safetensors"
        )

    print("Exporting to SafeTensors format...")

    model.eval()
    model.to("cpu")

    output_path.parent.mkdir(parents=True, exist_ok=True)

    # Get state dict and convert to safetensors format
    state_dict = model.state_dict()

    # SafeTensors requires all tensors to be contiguous
    tensors = {k: v.contiguous() for k, v in state_dict.items()}

    save_file(tensors, str(output_path))

    print(f"SafeTensors model saved to: {output_path}")
    return output_path


def export_model(
    checkpoint_path: Path,
    output_dir: Path,
    formats: list[ExportFormat],
    model_name: str = "blood_bowl_value_net",
    onnx_opset: int = 17,
    torchscript_method: str = "trace",
) -> dict[ExportFormat, Path]:
    """
    Export a model from a Lightning checkpoint to specified formats.

    Args:
        checkpoint_path: Path to the Lightning .ckpt checkpoint.
        output_dir: Directory for exported models.
        formats: List of export formats.
        model_name: Base name for exported files.
        onnx_opset: ONNX opset version.
        torchscript_method: TorchScript export method.

    Returns:
        Dictionary mapping format to output path.
    """
    # Load model from Lightning checkpoint
    print(f"Loading model from checkpoint: {checkpoint_path}")
    model = ValueNetworkModule.load_for_inference(str(checkpoint_path), device="cpu")

    # Determine formats to export
    if ExportFormat.ALL in formats:
        formats = [
            ExportFormat.ONNX,
            ExportFormat.TORCHSCRIPT,
            ExportFormat.PYTORCH,
            ExportFormat.SAFETENSORS,
        ]

    output_dir.mkdir(parents=True, exist_ok=True)
    exported: dict[ExportFormat, Path] = {}

    for fmt in formats:
        if fmt == ExportFormat.ONNX:
            output_path = output_dir / f"{model_name}.onnx"
            export_to_onnx(model, output_path, opset_version=onnx_opset)
            exported[fmt] = output_path

        elif fmt == ExportFormat.TORCHSCRIPT:
            output_path = output_dir / f"{model_name}.torchscript.pt"
            export_to_torchscript(model, output_path, method=torchscript_method)
            exported[fmt] = output_path

        elif fmt == ExportFormat.PYTORCH:
            output_path = output_dir / f"{model_name}.pth"
            export_to_pytorch(model, output_path)
            exported[fmt] = output_path

        elif fmt == ExportFormat.SAFETENSORS:
            output_path = output_dir / f"{model_name}.safetensors"
            export_to_safetensors(model, output_path)
            exported[fmt] = output_path

    return exported


def parse_args() -> argparse.Namespace:
    """Parse command-line arguments."""
    parser = argparse.ArgumentParser(
        description="Export Blood Bowl Value Network to various formats",
        formatter_class=argparse.ArgumentDefaultsHelpFormatter,
    )

    parser.add_argument(
        "checkpoint",
        type=str,
        help="Path to the Lightning checkpoint (.ckpt file)",
    )

    parser.add_argument(
        "--output-dir",
        type=str,
        default="exports",
        help="Directory for exported models",
    )

    parser.add_argument(
        "--model-name",
        type=str,
        default="blood_bowl_value_net",
        help="Base name for exported files",
    )

    parser.add_argument(
        "--format",
        type=str,
        nargs="+",
        default=["all"],
        choices=["onnx", "torchscript", "pytorch", "safetensors", "all"],
        help="Export format(s)",
    )

    parser.add_argument(
        "--onnx-opset",
        type=int,
        default=17,
        help="ONNX opset version",
    )

    parser.add_argument(
        "--torchscript-method",
        type=str,
        default="trace",
        choices=["trace", "script"],
        help="TorchScript export method",
    )

    return parser.parse_args()


def main() -> None:
    """Main export function."""
    args = parse_args()

    checkpoint_path = Path(args.checkpoint)
    if not checkpoint_path.exists():
        raise FileNotFoundError(f"Checkpoint not found: {checkpoint_path}")

    output_dir = Path(args.output_dir)
    formats = [ExportFormat(f) for f in args.format]

    print(f"\n{'=' * 60}")
    print("Blood Bowl Value Network Export")
    print(f"{'=' * 60}")
    print(f"Checkpoint: {checkpoint_path}")
    print(f"Output directory: {output_dir}")
    print(f"Formats: {[f.value for f in formats]}")
    print(f"{'=' * 60}\n")

    exported = export_model(
        checkpoint_path=checkpoint_path,
        output_dir=output_dir,
        formats=formats,
        model_name=args.model_name,
        onnx_opset=args.onnx_opset,
        torchscript_method=args.torchscript_method,
    )

    print(f"\n{'=' * 60}")
    print("Export complete!")
    print(f"{'=' * 60}")
    for fmt, path in exported.items():
        print(f"  {fmt.value}: {path}")
    print(f"{'=' * 60}\n")


if __name__ == "__main__":
    main()
