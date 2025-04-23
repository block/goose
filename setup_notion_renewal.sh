#!/bin/bash

# Create necessary directories
mkdir -p logs
mkdir -p config

# Create environment file
cat << EOF > .env
NOTION_TOKEN=your_notion_token_here
SMTP_SERVER=your_smtp_server
SMTP_PORT=587
SMTP_USERNAME=your_smtp_username
SMTP_PASSWORD=your_smtp_password
EOF

# Create service configuration
cat << EOF > notion_renewal.service
[Unit]
Description=Notion Renewal Notification Service
After=network.target

[Service]
ExecStart=/usr/bin/python3 /Users/kylewoolstenhulme/notion_renewal_production.py
WorkingDirectory=/Users/kylewoolstenhulme
User=kylewoolstenhulme
Restart=always
RestartSec=3600

[Install]
WantedBy=multi-user.target
EOF

# Set up log rotation
cat << EOF > notion_renewal_logrotate
/Users/kylewoolstenhulme/logs/notion_renewal.log {
    daily
    rotate 7
    compress
    delaycompress
    missingok
    notifempty
}
EOF

# Make scripts executable
chmod +x notion_renewal_production.py

echo "Environment setup completed!"
echo "Please update the .env file with your actual credentials"