#!/usr/bin/env python3
"""
Migration script to convert logging.getLogger() to structlog.get_logger()
"""

import re
from pathlib import Path


def migrate_file(file_path: Path) -> bool:
    """Migrate a single file from logging to structlog"""
    content = file_path.read_text(encoding="utf-8")
    original = content

    # Replace import statements
    content = re.sub(r"^import logging$", "import structlog", content, flags=re.MULTILINE)

    # Replace getLogger calls
    content = re.sub(r"logging\.getLogger\(__name__\)", "structlog.get_logger()", content)
    content = re.sub(r'logging\.getLogger\("([^"]+)"\)', r"structlog.get_logger()", content)
    content = re.sub(r"logging\.getLogger\('([^']+)'\)", r"structlog.get_logger()", content)

    # Write back if changed
    if content != original:
        file_path.write_text(content, encoding="utf-8")
        return True
    return False


def main():
    """Run migration on all Python files in src/"""
    src_dir = Path(__file__).parent.parent / "src"

    files_to_migrate = [
        "rules/utils/codeowners.py",
        "integrations/github/api.py",
        "integrations/github/rules_service.py",
        "webhooks/handlers/issue_comment.py",
        "main.py",
        "agents/acknowledgment_agent/test_agent.py",
        "agents/acknowledgment_agent/agent.py",
        "agents/feasibility_agent/agent.py",
        "agents/engine_agent/agent.py",
        "api/dependencies.py",
        "rules/utils/validation.py",
        "event_processors/base.py",
        "event_processors/deployment_protection_rule.py",
        "event_processors/pull_request.py",
        "event_processors/violation_acknowledgment.py",
        "integrations/github/rule_loader.py",
        "agents/feasibility_agent/nodes.py",
        "webhooks/handlers/deployment_status.py",
        "webhooks/handlers/deployment_review.py",
        "agents/base.py",
        "rules/validators.py",
        "rules/utils/contributors.py",
        "core/utils/logging.py",
        "core/utils/caching.py",
        "event_processors/push.py",
        "event_processors/rule_creation.py",
        "event_processors/deployment_status.py",
        "event_processors/deployment_review.py",
        "event_processors/deployment.py",
        "event_processors/check_run.py",
        "core/utils/timeout.py",
        "core/utils/retry.py",
        "core/utils/metrics.py",
        "agents/factory.py",
        "agents/engine_agent/nodes.py",
    ]

    migrated = 0
    for rel_path in files_to_migrate:
        file_path = src_dir / rel_path
        if file_path.exists():
            if migrate_file(file_path):
                print(f"[OK] Migrated: {rel_path}")
                migrated += 1
            else:
                print(f"[SKIP] No changes: {rel_path}")
        else:
            print(f"[ERROR] Not found: {rel_path}")

    print(f"\n[DONE] Migration complete! {migrated} files updated.")


if __name__ == "__main__":
    main()
