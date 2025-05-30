# Lead/Worker Model Feature Documentation

This directory contains documentation and examples for the Lead/Worker model feature in Goose.

## Files

- **`LEAD_WORKER_FEATURE.md`** - Complete feature documentation with configuration options and examples
- **`example-config.yaml`** - Example YAML configuration file showing lead/worker setup
- **`test_lead_worker_feature.sh`** - Original test script for the lead/worker functionality
- **`test_lead_worker_logging.sh`** - Test script specifically for the logging features

## Quick Start

The Lead/Worker feature allows you to use a more capable "lead" model for initial turns and planning, then switch to a faster/cheaper "worker" model for execution, with automatic fallback on failures.

### Simple Setup (Environment Variables)
```bash
export GOOSE_PROVIDER="openai"
export GOOSE_MODEL="gpt-4o-mini"           # Worker model
export GOOSE_LEAD_MODEL="gpt-4o"          # Lead model
```

### Advanced Setup (YAML Configuration)
See `example-config.yaml` for a complete configuration example.

## Features

- ✅ **Startup logging** - Shows all models being used at startup
- ✅ **Turn-by-turn logging** - Shows which model is active for each turn
- ✅ **Automatic fallback** - Switches back to lead model on worker failures
- ✅ **Smart recovery** - Returns to worker model after successful fallback
- ✅ **Cross-provider support** - Can use different providers for lead and worker

## Testing

Run the test scripts to see the feature in action:

```bash
# Test basic functionality
./test_lead_worker_feature.sh

# Test logging features
./test_lead_worker_logging.sh
```