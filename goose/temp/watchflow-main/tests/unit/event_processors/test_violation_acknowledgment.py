from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from src.core.models import Severity, Violation
from src.event_processors.violation_acknowledgment import ViolationAcknowledgmentProcessor
from src.integrations.github.check_runs import CheckRunManager


@pytest.fixture
def mock_task():
    # ViolationAck processor uses payload fields, not task attributes (unlike PR processor)
    # But we mock task just in case
    task = MagicMock()
    task.event_type = "issue_comment"
    task.payload = {
        "repository": {"full_name": "owner/repo"},
        "installation": {"id": 123},
        "issue": {"number": 1},
        "comment": {"body": '@watchflow ack "I know what I am doing"', "user": {"login": "dev"}},
    }
    return task


@pytest.fixture
def mock_github_client():
    client = MagicMock()
    client.get_installation_access_token = AsyncMock(return_value="token")
    client.get_pull_request = AsyncMock(return_value={"head": {"sha": "sha123"}})
    client.get_pull_request_files = AsyncMock(return_value=[])
    client.get_pull_request_reviews = AsyncMock(return_value=[])
    client.create_issue_comment = AsyncMock()
    client.create_check_run = AsyncMock(return_value={"id": 1})
    return client


@pytest.fixture
def mock_agents():
    engine = AsyncMock()

    # Create real Violation objects
    violation = Violation(rule_description="Rule 1", severity=Severity.HIGH, message="Bad code", how_to_fix="Fix it")

    # Mock engine execution result (returns list of violations)
    result = MagicMock()
    # Need to verify struct of AgentResult/EvaluationResult
    result.data = {"evaluation_result": MagicMock(violations=[violation])}
    engine.execute.return_value = result

    ack_agent = AsyncMock()
    ack_result = MagicMock()
    ack_result.success = True

    ack_result.data = {
        "is_valid": True,
        "reasoning": "Valid reason",
        "acknowledgable_violations": [violation],
        "require_fixes": [],
        "confidence": 0.9,
    }
    ack_agent.evaluate_acknowledgment.return_value = ack_result

    return engine, ack_agent


@pytest.fixture
def mock_rule_provider():
    provider = AsyncMock()
    rule = MagicMock()
    rule.event_types = ["pull_request"]
    provider.get_rules.return_value = [rule]
    return provider


@pytest.fixture
def processor(mock_github_client, mock_agents, mock_rule_provider):
    engine, ack = mock_agents

    # Patch dependencies
    with (
        patch("src.event_processors.violation_acknowledgment.get_agent") as mock_get_agent,
        patch("src.event_processors.base.github_client", mock_github_client),
    ):
        # Configure get_agent side effects
        def get_agent_side_effect(name):
            if name == "engine":
                return engine
            if name == "acknowledgment":
                return ack
            return MagicMock()

        mock_get_agent.side_effect = get_agent_side_effect

        proc = ViolationAcknowledgmentProcessor()
        proc.rule_provider = mock_rule_provider
        proc.github_client = mock_github_client
        proc.engine_agent = engine
        proc.acknowledgment_agent = ack
        # Mock CheckRunManager
        proc.check_run_manager = AsyncMock(spec=CheckRunManager)

        return proc


@pytest.mark.asyncio
async def test_process_valid_acknowledgment_success(processor, mock_task, mock_github_client):
    result = await processor.process(mock_task)

    assert result.success is True
    assert len(result.violations) == 0

    # Verify comment created (Ack accepted)
    mock_github_client.create_issue_comment.assert_called()
    call_args = mock_github_client.create_issue_comment.call_args[1]
    assert "Violations Acknowledged" in call_args["comment"]

    # Verify check run manager called
    processor.check_run_manager.create_acknowledgment_check_run.assert_awaited_once()


@pytest.mark.asyncio
async def test_process_invalid_acknowledgment(processor, mock_task, mock_github_client, mock_agents):
    _, ack_agent = mock_agents
    # Setup invalid ack
    v1 = Violation(rule_description="Rule 1", severity=Severity.HIGH, message="Bad code", how_to_fix="Fix it")

    ack_agent.evaluate_acknowledgment.return_value.data = {
        "is_valid": False,
        "reasoning": "Bad reason",
        "acknowledgable_violations": [],
        "require_fixes": [v1],
        "confidence": 0.9,
    }

    result = await processor.process(mock_task)

    assert result.success is True  # Process succeeded, even if ack rejected
    assert len(result.violations) == 1  # Returns violations requiring fixes
    assert isinstance(result.violations[0], Violation)

    # Verify comment created (Ack rejected)
    mock_github_client.create_issue_comment.assert_called()
    call_args = mock_github_client.create_issue_comment.call_args[1]
    assert "Acknowledgment Rejected" in call_args["comment"]

    # Verify check run updated with failure (require fixes)
    # The processor calls create_acknowledgment_check_run if valid?
    # Wait, in the code:
    # if valid: calls create_acknowledgment_check_run
    # else: calls _reject_acknowledgment ... wait, does it update check run if rejected?
    # In my code:
    # if valid: ... create_acknowledgment_check_run ...
    # else: ... _reject_acknowledgment ...
    # The original code didn't explicitly update check run on rejection?
    # Let's check the code I wrote.
    # It seems I did NOT add a check run update call in the else block of "is_valid".
    # This means strict check run status might depend on the NEXT push or just stay stale?
    # Actually, if the user tries to ack and fails, the check run status probably shouldn't verify to success.
    # So assertions on check run manager calls should reflect that.

    processor.check_run_manager.create_acknowledgment_check_run.assert_not_awaited()
    # Processor doesn't call create_check_run on rejection in the current logic (checked code)


@pytest.mark.asyncio
async def test_process_no_reason_in_comment(processor, mock_task, mock_github_client):
    mock_task.payload["comment"]["body"] = "Just a comment"

    result = await processor.process(mock_task)

    assert result.success is True

    # Verify help comment posted
    mock_github_client.create_issue_comment.assert_called()
    call_args = mock_github_client.create_issue_comment.call_args[1]
    assert "Acknowledgment Failed" in call_args["comment"]
    assert "Valid formats" in call_args["comment"]


@pytest.mark.asyncio
async def test_extract_acknowledgment_reason(processor):
    # Test regex patterns
    assert processor._extract_acknowledgment_reason('@watchflow ack "Reason"') == "Reason"
    assert processor._extract_acknowledgment_reason("@watchflow acknowledge 'Reason'") == "Reason"
    assert processor._extract_acknowledgment_reason("/override Reason here") == "Reason here"
    assert processor._extract_acknowledgment_reason("No match") == ""
