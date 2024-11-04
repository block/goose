import pytest

from exchange.providers.base import MissingProviderEnvVariableError, Provider


def test_missing_provider_env_variable_error_without_instructions_url():
    env_variable = "API_KEY"
    provider = "TestProvider"
    error = MissingProviderEnvVariableError(env_variable, provider)

    assert error.env_variable == env_variable
    assert error.provider == provider
    assert error.instructions_url is None
    assert error.message == "Missing environment variables: API_KEY for provider TestProvider."


def test_missing_provider_env_variable_error_with_instructions_url():
    env_variable = "API_KEY"
    provider = "TestProvider"
    instructions_url = "http://example.com/instructions"
    error = MissingProviderEnvVariableError(env_variable, provider, instructions_url)

    assert error.env_variable == env_variable
    assert error.provider == provider
    assert error.instructions_url == instructions_url
    assert error.message == (
        "Missing environment variables: API_KEY for provider TestProvider.\n"
        "Please see http://example.com/instructions for instructions"
    )


class TestProvider(Provider):
    PROVIDER_NAME = "test_provider"
    REQUIRED_ENV_VARS = []

    def complete(self, model, system, messages, tools, **kwargs):
        pass


class TestProviderBaseURL(Provider):
    PROVIDER_NAME = "test_provider_base_url"
    BASE_URL_ENV_VAR = "TEST_PROVIDER_BASE_URL"
    REQUIRED_ENV_VARS = []

    def complete(self, model, system, messages, tools, **kwargs):
        pass


class TestProviderBaseURLDefault(Provider):
    PROVIDER_NAME = "test_provider_base_url_default"
    BASE_URL_ENV_VAR = "TEST_PROVIDER_BASE_URL_DEFAULT"
    BASE_URL_DEFAULT = "http://localhost:11434/"
    REQUIRED_ENV_VARS = []

    def complete(self, model, system, messages, tools, **kwargs):
        pass


def test_check_env_vars_no_base_url():
    TestProvider.check_env_vars()


def test_check_env_vars_base_url_valid_http(monkeypatch):
    monkeypatch.setenv(TestProviderBaseURL.BASE_URL_ENV_VAR, "http://localhost:11434/")

    TestProviderBaseURL.check_env_vars()


def test_check_env_vars_base_url_valid_https(monkeypatch):
    monkeypatch.setenv(TestProviderBaseURL.BASE_URL_ENV_VAR, "https://localhost:11434/v1")

    TestProviderBaseURL.check_env_vars()


def test_check_env_vars_base_url_default():
    TestProviderBaseURLDefault.check_env_vars()


def test_check_env_vars_base_url_throw_error_when_empty(monkeypatch):
    monkeypatch.setenv(TestProviderBaseURL.BASE_URL_ENV_VAR, "")

    with pytest.raises(KeyError, match="TEST_PROVIDER_BASE_URL"):
        TestProviderBaseURL.check_env_vars()


def test_check_env_vars_base_url_throw_error_when_missing_schemes(monkeypatch):
    monkeypatch.setenv(TestProviderBaseURL.BASE_URL_ENV_VAR, "localhost:11434")

    with pytest.raises(
        ValueError, match="Expected TEST_PROVIDER_BASE_URL to be a 'http' or 'https' url: localhost:11434"
    ):
        TestProviderBaseURL.check_env_vars()


def test_check_env_vars_base_url_throw_error_when_invalid_scheme(monkeypatch):
    monkeypatch.setenv(TestProviderBaseURL.BASE_URL_ENV_VAR, "ftp://localhost:11434/v1")

    with pytest.raises(
        ValueError, match="Expected TEST_PROVIDER_BASE_URL to be a 'http' or 'https' url: ftp://localhost:11434/v1"
    ):
        TestProviderBaseURL.check_env_vars()
