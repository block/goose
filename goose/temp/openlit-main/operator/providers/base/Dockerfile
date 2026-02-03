######### Base Instrumentation Image #########
# Minimal base image for custom instrumentation setups
# Users add everything they need via CUSTOM_PACKAGES
FROM python:3.11-slim AS base-builder
WORKDIR /instrumentation

# Copy requirements and install minimal core packages
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt --target packages

# Copy the sitecustomize.py for base setup
COPY sitecustomize.py packages/sitecustomize.py

# Create the final minimal base image
FROM python:3.11-slim
WORKDIR /instrumentations

# Copy base packages to source location
COPY --from=base-builder /instrumentation/packages /instrumentations/base

# Copy the setup script
COPY setup-instrumentation.py /usr/local/bin/setup-instrumentation.py
RUN chmod +x /usr/local/bin/setup-instrumentation.py

ENV INSTRUMENTATION_PROVIDER=base
CMD ["python3", "/usr/local/bin/setup-instrumentation.py"]