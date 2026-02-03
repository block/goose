######### OpenInference Instrumentation Image #########
FROM python:3.11-slim AS openinference-builder
WORKDIR /instrumentation

# Copy requirements and install comprehensive OpenInference packages
# Using pure OpenTelemetry approach - works with OpenLIT!
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt --target packages

# Copy the sitecustomize.py for OpenInference
COPY sitecustomize.py packages/sitecustomize.py

# Create final OpenInference image
FROM python:3.11-slim
WORKDIR /instrumentations

# Copy pre-built packages to source location
COPY --from=openinference-builder /instrumentation/packages /instrumentations/openinference

# Copy the setup script
COPY setup-instrumentation.py /usr/local/bin/setup-instrumentation.py
RUN chmod +x /usr/local/bin/setup-instrumentation.py

ENV INSTRUMENTATION_PROVIDER=openinference
CMD ["python3", "/usr/local/bin/setup-instrumentation.py"]