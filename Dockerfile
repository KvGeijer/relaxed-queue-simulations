# Use a specific version of the Rust slim image
FROM rust:1.82-slim

# Avoid prompts during package installation
ARG DEBIAN_FRONTEND=noninteractive

# Update the package list and install Python3, pip, and required tools
RUN apt-get update && apt-get install -y \
    python3 \
    python3-pip \
    python3-setuptools \
    build-essential \
    bc \
    && rm -rf /var/lib/apt/lists/*

# Bypass the "externally managed environment" restriction to upgrade pip
RUN python3 -m pip install --upgrade pip --break-system-packages --root-user-action=ignore

# Install the required Python packages globally
RUN pip install numpy==1.26.3 matplotlib==3.8.2 scipy==1.12.0 seaborn==0.13.2 --break-system-packages --root-user-action=ignore

# Copy the current directory contents into the container at /app
COPY . /app

# Set the working directory to /app
WORKDIR /app

# Define a volume to access generated files outside the container
VOLUME ["/app/results"]

# Default command to keep the container running
CMD ["bash", "recreate-ppopp.sh"]

