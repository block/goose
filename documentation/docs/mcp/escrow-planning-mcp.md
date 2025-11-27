---
title: Escrow Planning MCP Server
description: Add Escrow Planning MCP Server as a goose Extension
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';
import GooseDesktopInstaller from '@site/src/components/GooseDesktopInstaller';
import CLIExtensionInstructions from '@site/src/components/CLIExtensionInstructions';

This tutorial covers how to add the [Escrow Planning MCP Server](https://github.com/nelsonjingusc/goose-mcp-escrow-server) as a goose extension. This extension introduces a transactional execution model that separates intent formation, safety verification, deterministic planning, and commit or rollback execution, enabling goose to operate with greater predictability and control in production environments.

:::tip TLDR
<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  [Launch the installer](goose://extension?cmd=uvx&arg=goose-mcp-escrow-server&id=escrow_planning&name=Escrow%20Planning&description=Transactional%20planning%20with%20safety%20verification%20and%20rollback)
  </TabItem>
  <TabItem value="cli" label="goose CLI">
  **Command**
  ```sh
  uvx goose-mcp-escrow-server
  ```
  </TabItem>
</Tabs>
:::

## What is Escrow-Style Planning?

Traditional agentic workflows often execute actions immediately after planning, making it difficult to verify safety, track state, or recover from errors. The Escrow Planning model addresses this by introducing a **commit-or-rollback execution pattern** inspired by database transactions.

### Key Architectural Principles

1. **Intent Parsing**: User requests are parsed into structured, machine-verifiable plans
2. **Safety Verification**: Pre-execution validation checks for potentially destructive operations
3. **Deterministic Planning**: Plans are serializable and reproducible across sessions
4. **Atomic Execution**: Operations execute as transactional units with rollback support

This approach is particularly valuable for:
- Production systems requiring auditability
- Multi-step workflows where partial failures must be handled gracefully
- Scenarios where human oversight is needed before execution
- Teams building reliable agentic systems with predictable behavior

## Configuration

:::info
This extension requires Python 3.10+ and uses `uvx` for execution. The server will automatically install dependencies on first run.
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
  <GooseDesktopInstaller
    extensionId="escrow_planning"
    extensionName="Escrow Planning"
    description="Transactional planning with safety verification and rollback"
    command="uvx"
    args={["goose-mcp-escrow-server"]}
  />
  </TabItem>
  <TabItem value="cli" label="goose CLI">
    <CLIExtensionInstructions
      name="Escrow Planning"
      description="Transactional planning with safety verification and rollback"
      command="uvx goose-mcp-escrow-server"
    />
  </TabItem>
</Tabs>

## Example Usage

This example demonstrates the conceptual behavior of an escrow-style planner. 
It does not execute real database operations.

In this example, I'll demonstrate how goose can safely execute a multi-step database migration with automatic rollback if any step fails. The Escrow Planning extension ensures that destructive operations are validated before execution and can be reverted if issues are detected.

This pattern is essential for production scenarios where partial failures could leave systems in inconsistent states.

:::info LLM
This example works with any model supporting MCP tool calls. Anthropic's Claude 3.5 Sonnet was used for testing.
:::

<Tabs groupId="interface">
  <TabItem value="ui" label="goose Desktop" default>
   1. Open a new session in goose Desktop
  </TabItem>
  <TabItem value="cli" label="goose CLI">

  1. Open a terminal and start a new goose session:

  ```sh
  goose session
  ```

  </TabItem>
</Tabs>

### Step 1: Create a Multi-Step Migration Plan

#### goose Prompt #1
```
goose, I need to migrate our user authentication system:

1. Backup the current users table
2. Add new columns for OAuth tokens
3. Migrate existing password hashes to new format
4. Update the authentication service
5. Run integration tests

Create an escrow plan for this migration with safety checks.
```

#### goose Output

```
I'll create a structured escrow plan for your authentication migration with built-in safety verification.

[... Using escrow_create_plan ...]

Escrow Plan Created (ID: auth_migration_2024)

┌─────────────────────────────────────────────────┐
│ Authentication System Migration                  │
│ Status: PLANNING                                 │
│ Safety Level: HIGH_RISK                          │
└─────────────────────────────────────────────────┘

Execution Steps:

  1. ✓ database_backup
     Action: pg_dump users table
     Verification: Backup file created, checksum validated
     Rollback: Restore from backup

  2. ⚠ schema_modification
     Action: ALTER TABLE users ADD COLUMN oauth_token
     Verification: Column exists, no data loss
     Rollback: DROP COLUMN oauth_token

  3. ⚠ data_migration
     Action: Migrate password hashes (bcrypt → argon2)
     Verification: All rows migrated, integrity checks passed
     Rollback: Restore original hash format from backup

  4. ⚠ service_update
     Action: Deploy new authentication service
     Verification: Health checks passed, no 5xx errors
     Rollback: Revert to previous deployment

  5. ✓ integration_tests
     Action: Run auth test suite
     Verification: 100% pass rate
     Rollback: Fail the migration, trigger full rollback

Safety Analysis:

  ⚠ Detected 3 high-risk operations (schema changes, data migrations)
  ✓ All operations have defined rollback procedures
  ✓ Backup strategy verified
  ⚠ Estimated rollback time: 15 minutes

  Recommendation: Execute in maintenance window with manual approval

The plan is ready for review. Use:
  - escrow_commit_plan(auth_migration_2024) to execute
  - escrow_modify_plan(auth_migration_2024, ...) to adjust
  - escrow_rollback_plan(auth_migration_2024) to cancel
```

### Step 2: Review and Execute with Safety Checks

#### goose Prompt #2
```
The plan looks good. Commit it and execute step by step.
```

#### goose Output

```
I'll now commit and execute the escrow plan with step-by-step verification.

[... Using escrow_commit_plan ...]

Executing: Authentication System Migration

Step 1/5: database_backup
  ▶ Running pg_dump users table
  ✓ Backup created: users_backup_20240126.sql
  ✓ Checksum: 7a8f9e2c... (validated)
  ✓ Rollback procedure registered

Step 2/5: schema_modification
  ▶ Executing: ALTER TABLE users ADD COLUMN oauth_token VARCHAR(512)
  ✓ Column created successfully
  ✓ Verification: 0 data loss, table accessible
  ✓ Rollback procedure registered

Step 3/5: data_migration
  ▶ Migrating 10,247 password hashes
  ✓ Progress: 100% (10,247/10,247)
  ✓ Integrity check: All hashes valid
  ✓ No duplicate entries detected
  ✓ Rollback procedure registered

Step 4/5: service_update
  ▶ Deploying authentication-service v2.3.0
  ✓ Container started successfully
  ✓ Health endpoint responding (200 OK)
  ✓ Error rate: 0.00% (baseline: 0.01%)
  ✓ Rollback procedure registered

Step 5/5: integration_tests
  ▶ Running auth test suite (47 tests)
  ✓ All tests passed (47/47)
  ✓ OAuth flow: ✓
  ✓ Password auth: ✓
  ✓ Token refresh: ✓

┌─────────────────────────────────────────────────┐
│ Migration Complete                               │
│ Status: COMMITTED                                │
│ Duration: 8m 42s                                 │
│ Rollback: Available for 24h                      │
└─────────────────────────────────────────────────┘

All operations executed successfully. The system is now using the new
authentication scheme. If any issues arise in the next 24 hours, you can
still perform a full rollback using:

  escrow_rollback_plan(auth_migration_2024)

The rollback will restore:
  - Original database schema
  - Previous password hash format
  - Previous service version
```

### Step 3: Handle Failure with Automatic Rollback

#### goose Prompt #3
```
What happens if step 4 fails with a 500 error?
```

#### goose Output

```
If the service_update step fails, the escrow system automatically triggers
a full rollback of all previously executed steps. Here's what would happen:

Failure Detected at Step 4/5: service_update
  ✗ Health endpoint returned 500 Internal Server Error
  ⚠ Initiating automatic rollback sequence

┌─────────────────────────────────────────────────┐
│ Rollback Initiated                               │
│ Reason: service_update health check failed       │
│ Status: ROLLING_BACK                             │
└─────────────────────────────────────────────────┘

Rollback Step 4: service_update
  ▶ Reverting to authentication-service v2.2.1
  ✓ Previous version restored
  ✓ Health checks passing

Rollback Step 3: data_migration
  ▶ Restoring original password hash format
  ✓ 10,247 rows restored from backup
  ✓ Integrity verified

Rollback Step 2: schema_modification
  ▶ Executing: ALTER TABLE users DROP COLUMN oauth_token
  ✓ Column removed
  ✓ Table schema reverted

Rollback Step 1: database_backup
  ▶ Cleanup: Retaining backup for audit
  ✓ Backup preserved at users_backup_20240126.sql

┌─────────────────────────────────────────────────┐
│ Rollback Complete                                │
│ Status: REVERTED                                 │
│ System State: Restored to pre-migration          │
│ Duration: 4m 18s                                 │
└─────────────────────────────────────────────────┘

The system is now in the exact state it was before the migration attempt.
No data was lost, and all services are operational.

Error details and rollback logs are available for debugging:
  - Migration attempt: auth_migration_2024
  - Failure reason: Service deployment health check timeout
  - Rollback log: /var/log/escrow/auth_migration_2024_rollback.log
```

## Why Use Escrow Planning?

Traditional agentic execution lacks built-in safety mechanisms and state management. The Escrow Planning extension solves this by:

1. **Preventing Partial Failures**: Multi-step operations execute atomically—either fully complete or fully rollback
2. **Enabling Human Oversight**: Review and approve plans before execution in production scenarios
3. **Providing Auditability**: Every step is logged with verification points for compliance and debugging
4. **Supporting Complex Workflows**: Chain dependent operations with confidence that failures won't corrupt state

This makes goose suitable for scenarios that were previously too risky for autonomous agents:
- Database migrations and schema changes
- Multi-service deployments
- Infrastructure provisioning
- Batch data processing with validation requirements

## Advanced Features

### Safety Levels

The extension automatically classifies operations by risk:

| Level | Examples | Behavior |
|-------|----------|----------|
| **LOW** | Read-only operations, GET requests | Auto-approve by default |
| **MEDIUM** | Non-destructive writes, idempotent updates | Require verification checks |
| **HIGH** | Schema changes, deletions, deployments | Require explicit approval + rollback plan |
| **CRITICAL** | Production data deletion, security changes | Require multi-step approval + audit trail |

### Custom Verification Hooks

You can define custom verification logic for domain-specific safety checks:

```python
# Example: Custom verification for financial transactions
def verify_balance_consistency(plan_context):
    """Ensure all account balances sum to zero after transfers"""
    return sum(account.balance for account in plan_context.accounts) == 0

escrow_create_plan(
    steps=[...],
    verifications={"balance_check": verify_balance_consistency}
)
```

### Plan Persistence

All escrow plans are persisted to disk and survive process restarts. This enables:
- Long-running migrations that span maintenance windows
- Cross-session collaboration (one developer creates plan, another reviews and executes)
- Post-mortem analysis of failed operations

Plans are stored in `~/.goose/escrow/plans/` with full execution history.

## Technical Implementation

The Escrow Planning MCP Server is built using:
- **MCP SDK**: Native integration with goose's extension system
- **State Machine**: Formal plan lifecycle (PLANNING → COMMITTED → EXECUTING → COMPLETED/REVERTED)
- **Transactional Logging**: Write-ahead log for crash recovery
- **Idempotency**: All operations can be safely retried

Source code and contribution guidelines: https://github.com/nelsonjingusc/goose-mcp-escrow-server

## Use Cases

### 1. Infrastructure as Code Deployments
Execute Terraform/CloudFormation changes with automatic rollback on provider errors

### 2. Data Pipeline Orchestration
Run ETL workflows where data quality checks gate each stage

### 3. Multi-Repository Refactoring
Coordinate changes across services with rollback if integration tests fail

### 4. Security Patch Deployment
Apply patches with verification that services remain operational

## Best Practices

1. **Define Clear Verification Points**: Each step should have measurable success criteria
2. **Test Rollback Procedures**: Verify rollback logic works before executing in production
3. **Use Idempotent Operations**: Design steps to be safely re-executable
4. **Set Appropriate Timeouts**: Give operations enough time to complete but fail fast on hangs
5. **Preserve Audit Trails**: Keep plan logs for compliance and post-mortems

## Troubleshooting

**Q: What happens if the process crashes mid-execution?**
A: The server uses write-ahead logging. On restart, it detects incomplete plans and offers to continue or rollback.

**Q: Can I inspect a plan without executing it?**
A: Yes, use `escrow_get_plan(plan_id)` to view the full execution strategy and rollback procedures.

**Q: How long are rollback operations available?**
A: By default, rollback is available for 24 hours after successful execution. This is configurable per plan.

**Q: Can multiple plans execute concurrently?**
A: Yes, but the server enforces resource locks to prevent conflicts (e.g., two plans modifying the same database table).

## Contributing

This is a community-maintained extension. Contributions are welcome:
- Feature requests and bug reports: [GitHub Issues](https://github.com/nelsonjingusc/goose-mcp-escrow-server/issues)
- Pull requests: [Contributing Guide](https://github.com/nelsonjingusc/goose-mcp-escrow-server/blob/main/CONTRIBUTING.md)
- Discussions: [GitHub Discussions](https://github.com/nelsonjingusc/goose-mcp-escrow-server/discussions)
