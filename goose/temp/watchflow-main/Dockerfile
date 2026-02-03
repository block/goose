FROM python:3.12-slim

# Set environment variables
ENV PYTHONDONTWRITEBYTECODE=1 \
    PYTHONUNBUFFERED=1 \
    PYTHONIOENCODING=utf-8

# Install system dependencies
RUN apt-get update && \
    apt-get install -y \
        gcc \
        curl \
        ca-certificates && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Install UV package manager
RUN pip install --upgrade pip && \
    pip install uv

# Set working directory
WORKDIR /app

# Copy requirements and install dependencies
COPY pyproject.toml ./
RUN uv pip install --system --no-cache-dir -e .

# Copy application source code and scripts
COPY src/ ./src/
COPY scripts/ ./scripts/

# Make scripts executable
RUN chmod +x scripts/*.sh

# Expose port
EXPOSE 8000

# Start the application using production script
CMD ["./scripts/start-prod.sh"]
