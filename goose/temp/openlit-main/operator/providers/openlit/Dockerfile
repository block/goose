######### OpenLIT Instrumentation Image #########
FROM python:3.11-slim AS openlit-builder
WORKDIR /instrumentation

# Install OpenLIT with all instrumentations
RUN pip install --no-cache-dir 'openlit[all]' --target packages

# Copy the sitecustomize.py for OpenLIT
COPY sitecustomize.py packages/sitecustomize.py

# Create final OpenLIT image
FROM python:3.11-slim
WORKDIR /instrumentations

# Copy pre-built packages to source location
COPY --from=openlit-builder /instrumentation/packages /instrumentations/openlit

# Copy the setup script
COPY setup-instrumentation.py /usr/local/bin/setup-instrumentation.py
RUN chmod +x /usr/local/bin/setup-instrumentation.py

ENV INSTRUMENTATION_PROVIDER=openlit
CMD ["python3", "/usr/local/bin/setup-instrumentation.py"]