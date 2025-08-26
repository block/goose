FROM goose-recipe-scanner:base

# Switch back to root to install Goose and copy files
USER root

# Pre-download and install Goose CLI to avoid network issues during runtime
RUN curl -fsSL https://github.com/block/goose/releases/download/stable/download_cli.sh | \
    CONFIGURE=false GOOSE_BIN_DIR=/usr/local/bin bash && \
    echo "✅ Goose CLI pre-installed: $(/usr/local/bin/goose --version)"

# Copy Goose configuration
COPY config.yaml /home/scanner/.config/goose/config.yaml

# Copy scanning script and base recipe
COPY scan-recipe.sh /usr/local/bin/scan-recipe.sh
COPY base_recipe.yaml /docker/base_recipe.yaml
RUN chmod +x /usr/local/bin/scan-recipe.sh

RUN chown scanner:scanner /home/scanner/.config/goose/config.yaml /docker/base_recipe.yaml

# Switch back to non-root user
USER scanner

# Default entrypoint
ENTRYPOINT ["/usr/local/bin/scan-recipe.sh"]
