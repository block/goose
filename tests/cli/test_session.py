import os
from typing import Union
from unittest.mock import MagicMock, mock_open, patch

import pytest
from exchange import Message, ToolResult, ToolUse
from goose.cli.prompt.goose_prompt_session import GoosePromptSession
from goose.cli.prompt.overwrite_session_prompt import OverwriteSessionPrompt
from goose.cli.prompt.user_input import PromptAction, UserInput
from goose.cli.session import Session
from prompt_toolkit import PromptSession

SPECIFIED_SESSION_NAME = "mySession"
SESSION_NAME = "test"


@pytest.fixture(scope="module", autouse=True)
def set_openai_api_key():
    key = "OPENAI_API_KEY"
    value = "test_api_key"

    original_api_key = os.environ.get(key)
    os.environ[key] = value

    yield

    if original_api_key is None:
        os.environ.pop(key, None)
    else:
        os.environ[key] = original_api_key


@pytest.fixture
@patch.object(PromptSession, "prompt", return_value=SPECIFIED_SESSION_NAME)
def mock_specified_session_name(specified_session_name):
    yield specified_session_name


@pytest.fixture
@patch("goose.cli.session.create_exchange", name="mock_exchange")
@patch("goose.cli.session.load_profile", name="mock_load_profile")
@patch("goose.cli.session.SessionNotifier", name="mock_session_notifier")
@patch("goose.cli.session.load_provider", name="mock_load_provider")
def create_session_with_mock_configs(
    mock_load_provider,
    mock_session_notifier,
    mock_load_profile,
    mock_exchange,
    mock_sessions_path,
    exchange_factory,
    profile_factory,
):
    mock_load_provider.return_value = "provider"
    mock_session_notifier.return_value = MagicMock()
    mock_load_profile.return_value = profile_factory()
    mock_exchange.return_value = exchange_factory()

    def create_session(session_attributes: dict = {}):
        return Session(**session_attributes)

    return create_session


@pytest.fixture
def session_factory(create_session_with_mock_configs):
    def factory(
        name=SESSION_NAME,
        overwrite_prompt=None,
        is_existing_session=None,
        get_initial_messages=None,
        file_opener=open,
    ):
        session = create_session_with_mock_configs({"name": name})
        session.overwrite_prompt = overwrite_prompt or OverwriteSessionPrompt()
        session.is_existing_session = is_existing_session or (lambda _: False)
        session._get_initial_messages = get_initial_messages or (lambda: [])
        session.file_opener = file_opener
        return session

    return factory


def test_session_does_not_extend_last_user_text_message_on_init(
    create_session_with_mock_configs, mock_sessions_path, create_session_file
):
    messages = [Message.user("Hello"), Message.assistant("Hi"), Message.user("Last should be removed")]
    create_session_file(messages, mock_sessions_path / f"{SESSION_NAME}.jsonl")

    session = create_session_with_mock_configs({"name": SESSION_NAME})
    print("Messages after session init:", session.exchange.messages)  # Debugging line
    assert len(session.exchange.messages) == 2
    assert [message.text for message in session.exchange.messages] == ["Hello", "Hi"]


def test_session_adds_resume_message_if_last_message_is_tool_result(
    create_session_with_mock_configs, mock_sessions_path, create_session_file
):
    messages = [
        Message.user("Hello"),
        Message(role="assistant", content=[ToolUse(id="1", name="first_tool", parameters={})]),
        Message(role="user", content=[ToolResult(tool_use_id="1", output="output")]),
    ]
    create_session_file(messages, mock_sessions_path / f"{SESSION_NAME}.jsonl")

    session = create_session_with_mock_configs({"name": SESSION_NAME})
    print("Messages after session init:", session.exchange.messages)  # Debugging line
    assert len(session.exchange.messages) == 4
    assert session.exchange.messages[-1].role == "assistant"
    assert session.exchange.messages[-1].text == "I see we were interrupted. How can I help you?"


def test_session_removes_tool_use_and_adds_resume_message_if_last_message_is_tool_use(
    create_session_with_mock_configs, mock_sessions_path, create_session_file
):
    messages = [
        Message.user("Hello"),
        Message(role="assistant", content=[ToolUse(id="1", name="first_tool", parameters={})]),
    ]
    create_session_file(messages, mock_sessions_path / f"{SESSION_NAME}.jsonl")

    session = create_session_with_mock_configs({"name": SESSION_NAME})
    print("Messages after session init:", session.exchange.messages)  # Debugging line
    assert len(session.exchange.messages) == 2
    assert [message.text for message in session.exchange.messages] == [
        "Hello",
        "I see we were interrupted. How can I help you?",
    ]


def test_process_first_message_return_message(create_session_with_mock_configs):
    session = create_session_with_mock_configs()
    with patch.object(
        GoosePromptSession, "get_user_input", return_value=UserInput(action=PromptAction.CONTINUE, text="Hello")
    ):
        message = session.process_first_message()

        assert message.text == "Hello"
        assert len(session.exchange.messages) == 0


def test_process_first_message_to_exit(create_session_with_mock_configs):
    session = create_session_with_mock_configs()
    with patch.object(GoosePromptSession, "get_user_input", return_value=UserInput(action=PromptAction.EXIT)):
        message = session.process_first_message()

        assert message is None


def test_process_first_message_return_last_exchange_message(create_session_with_mock_configs):
    session = create_session_with_mock_configs()
    session.exchange.messages.append(Message.user("Hi"))

    message = session.process_first_message()

    assert message.text == "Hi"
    assert len(session.exchange.messages) == 0


def test_log_log_cost(create_session_with_mock_configs):
    session = create_session_with_mock_configs()
    mock_logger = MagicMock()
    cost_message = "You have used 100 tokens"
    with (
        patch("exchange.Exchange.get_token_usage", return_value={}),
        patch("goose.cli.session.get_total_cost_message", return_value=cost_message),
        patch("goose.cli.session.get_logger", return_value=mock_logger),
    ):
        session._log_cost()
        mock_logger.info.assert_called_once_with(cost_message)


@patch("goose.cli.session.droid", return_value="generated_session_name")
@patch("goose.cli.session.load_provider")
def test_set_generated_session_name(
    mock_load_provider, mock_droid, create_session_with_mock_configs, mock_sessions_path
):
    mock_provider = MagicMock()
    mock_load_provider.return_value = mock_provider

    session = create_session_with_mock_configs({"name": None})

    assert session.name == "generated_session_name"


@patch("goose.cli.session.is_existing_session", name="mock_is_existing")
@patch("goose.cli.session.Session._prompt_overwrite_session", name="mock_prompt")
def test_existing_session_prompt(
    mock_prompt,
    mock_is_existing,
    create_session_with_mock_configs,
):
    session = create_session_with_mock_configs({"name": SESSION_NAME})

    def check_prompt_behavior(
        is_existing: bool,
        new_session: Union[bool, None],
        should_prompt: bool,
    ) -> None:
        mock_is_existing.return_value = is_existing
        if new_session is None:
            session.run()
        else:
            session.run(new_session=new_session)

        if should_prompt:
            mock_prompt.assert_called_once()
        else:
            mock_prompt.assert_not_called()
        mock_prompt.reset_mock()

    check_prompt_behavior(is_existing=True, new_session=None, should_prompt=True)
    check_prompt_behavior(is_existing=False, new_session=None, should_prompt=False)
    check_prompt_behavior(is_existing=True, new_session=True, should_prompt=True)
    check_prompt_behavior(is_existing=False, new_session=False, should_prompt=False)


def test_prompt_overwrite_session(session_factory):
    def check_overwrite_behavior(choice: str, expected_messages: list[Message]) -> None:
        session = session_factory()

        with (
            patch.object(OverwriteSessionPrompt, "ask", return_value=choice),
            patch.object(session, "is_existing_session", return_value=True),
            patch.object(
                session,
                "_get_initial_messages",
                return_value=[Message.user(text="duck duck"), Message.user(text="goose")],
            ),
            patch("rich.prompt.Prompt.ask", return_value="new_session_name"),
            patch("builtins.open", mock_open()) as mock_file,
        ):
            session._prompt_overwrite_session()

            if choice in ["y", "yes"]:
                mock_file.assert_called_once_with(session.session_file_path, "w")
                mock_file().write.assert_called_once_with("")
            elif choice in ["n", "no"]:
                assert session.name == "new_session_name"
            elif choice in ["r", "resume"]:
                # this is tested comparing the contents of the array
                pass

            # because the messages are created with an id and creation date, we only want to check the text
            actual_messages = [message.text for message in session.exchange.messages]
            expected_messages = [message.text for message in expected_messages]
            assert actual_messages == expected_messages

    check_overwrite_behavior(choice="yes", expected_messages=[])
    check_overwrite_behavior(choice="y", expected_messages=[])
    check_overwrite_behavior(choice="no", expected_messages=[])
    check_overwrite_behavior(choice="n", expected_messages=[])
    check_overwrite_behavior(
        choice="resume",
        expected_messages=[Message.user(text="duck duck"), Message.user(text="goose")],
    )
    check_overwrite_behavior(
        choice="r",
        expected_messages=[Message.user(text="duck duck"), Message.user(text="goose")],
    )
