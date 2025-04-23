FROM --platform=linux/amd64 python:3.11-slim

WORKDIR /app

# Create a non-root user
RUN useradd -m gooseuser

# Copy the Python example and pre-built library
COPY goose_agent.py /app/
COPY libgoose_ffi.so /app/

# Modify the Python script to use the local library path
RUN sed -i 's|LIB_PATH = os.path.join(os.path.dirname(__file__), "../../..", "target", "release", LIB_NAME)|LIB_PATH = "/app/libgoose_ffi.so"|g' /app/goose_agent.py && \
    chmod +x /app/goose_agent.py

# Set read-only filesystem for security
VOLUME ["/tmp"]
ENV PYTHONUNBUFFERED=1

# Switch to non-root user
USER gooseuser

# Run the example with API key
CMD ["python3", "goose_agent.py"]
