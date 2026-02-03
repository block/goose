######### OpenLLMetry Instrumentation Image #########
FROM python:3.11-slim AS openllmetry-builder
WORKDIR /instrumentation

# Copy requirements and install OpenLLMetry packages
COPY requirements.txt .
RUN pip install --no-cache-dir -r requirements.txt --target packages

# Copy the sitecustomize.py for OpenLLMetry
COPY sitecustomize.py packages/sitecustomize.py

# Create final OpenLLMetry image
FROM python:3.11-slim
WORKDIR /instrumentations

# Copy pre-built packages to source location
COPY --from=openllmetry-builder /instrumentation/packages /instrumentations/openllmetry

# Copy the setup script
COPY setup-instrumentation.py /usr/local/bin/setup-instrumentation.py
RUN chmod +x /usr/local/bin/setup-instrumentation.py

ENV INSTRUMENTATION_PROVIDER=openllmetry
CMD ["python3", "/usr/local/bin/setup-instrumentation.py"]