#!/bin/bash
set -euxo pipefail

# This script runs the training pipeline inside a Docker container with ROCm.

# --- Configuration ---
DOCKER_IMAGE="rocm/pytorch:rocm7.1_ubuntu22.04_py3.10_pytorch_release_2.8.0"
PROJECT_ROOT=$(pwd)

# Export Python deps for the container (host-side)
echo "INFO: Exporting Python requirements for ROCm image..."
uv export --only-group rocm --prune torch --prune numpy --format requirements.txt > python/nn/requirements.txt
mkdir -p artifacts

# --- Docker Command ---
echo "Starting Docker container..."
docker run -it --rm \
    --network=host \
    --device=/dev/kfd \
    --device=/dev/dri \
    --group-add=video \
    --ipc=host \
    --cap-add=SYS_PTRACE \
    --security-opt seccomp=unconfined \
    --shm-size 8G \
    -v "${PROJECT_ROOT}":/app \
    -w /app \
    "${DOCKER_IMAGE}" \
    /bin/bash -c "
        set -euxo pipefail

        # Ensure training data is present
        if [ ! -f data/merged.jsonl ]; then
          echo 'ERROR: No data to train the model!'
          exit 1
        fi
        echo 'INFO: Training data available.'

        echo 'INFO: Installing Python dependencies...'
        pip install -r python/nn/requirements.txt

        echo 'INFO: Running training...'
        python python/nn/value_network/train.py --data-dir data --num-workers 4 --precision 32 --matmul-precision high\
          --batch-size 64 --test-split 0.1 --max-epochs 1000 --early-stopping-patience 0
        echo 'INFO: Training complete.'
    "

echo "INFO: Docker container finished."
