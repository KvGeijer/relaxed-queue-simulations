# Use a specific version of the Rust slim image
FROM rust:1.82-slim

# Avoid prompts during package installation
ARG DEBIAN_FRONTEND=noninteractive

# Update the package list and install Python3 and required packages
RUN apt-get update && apt-get install -y \
    python3 \
    python3-pip \
    python3-venv \
    build-essential \
    bc \
    && rm -rf /var/lib/apt/lists/*

# Copy the current directory contents into the container at /app
COPY . /app

# Set the working directory to /app
WORKDIR /app

# Create a Python virtual environment
RUN python3 -m venv /app/venv
RUN /app/venv/bin/pip install numpy==1.26.3 matplotlib==3.8.2 scipy==1.12.0 seaborn==0.13.2
ENV PATH="/app/venv/bin:$PATH"

# Define a volume to access generated files outside the container
VOLUME ["/app/results"]

# Default command to keep the container running
CMD ["bash", "recreate-ppopp.sh"]
