#!/usr/bin/env python3
import dataclasses
import os
import subprocess
import tempfile
import time
from enum import Enum
from typing import List, Optional

import typer
from rich.console import Console
from rich.theme import Theme
from typing_extensions import Annotated

# Initialize typer app and rich console
app = typer.Typer(help="Goose CLI Integration Tests")
console = Console(theme=Theme({
    "info": "cyan",
    "warning": "yellow",
    "error": "red",
    "success": "green"
}))


# Define workflow types
class Workflow(str, Enum):
    SERIAL = "serial"
    CONVERSATIONAL = "conversational"


@dataclasses.dataclass
class Topic:
    initial_prompt: str
    follow_ups: List[str]


@dataclasses.dataclass
class Conversation:
    topics: List[Topic]


# Extension configurations
EXTENSIONS = ['developer', 'computercontroller', 'google_drive', 'memory']

EXTENSION_PROMPTS = {
    'developer': [
        "List the contents of the current directory.",
        "Create a new file called test.txt with the content 'Hello, World!'",
        "Read the contents of test.txt"
    ],
    'computercontroller': [
        "What are the headlines on hackernews? Organize the list into categories.",
        "Make a ding sound"
    ],
    'google_drive': [
        "List the files in my Google Drive.",
        "Search for documents containing 'meeting notes'"
    ],
    'memory': [
        "Save this fact: The capital of France is Paris.",
        "What is the capital of France?"
    ]
}

CONV_EXTENSION_PROMPTS = {
    k: Conversation(topics=[
        Topic(val, ["summarize"])
        for val in v
    ])
    for k, v in EXTENSION_PROMPTS.items()
}


class Bench:
    def __init__(self, workflow: Workflow):
        self.error_log = []
        self.workflow = workflow

    def log_error(self, provider: str, model: str, extension: str, error: str) -> None:
        """Log an error message."""
        self.error_log.append(
            f"Provider: {provider}, Model: {model}, Extension: {extension}\n{error}\n"
        )

    def evaluate(self,
                 provider: str,
                 model: str,
                 extension: str,
                 prompt: str,
                 follow_ups: Optional[List[str]] = None) -> None:
        """Run a single test with the given parameters using pexpect."""
        console.print(f"Testing: {provider}/{model} with {extension}", style="info")
        console.print(f"Prompt: {prompt}", style="info")
        console.print(f"Workflow: {self.workflow.value}", style="info")

        follow_ups = follow_ups or []

        # Create temporary file for prompt
        with tempfile.NamedTemporaryFile(mode='w', delete=False) as temp:
            temp.write(prompt)
            temp_path = temp.name

        try:
            # Run goose with timeout
            cmd = ['goose', 'run', '--with-builtin', extension, '-t', prompt]
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                timeout=30
            )

            if result.returncode != 0:
                self.log_error(provider, model, extension,
                               result.stdout + result.stderr)
                console.print("✗ Test failed", style="error")

            else:
                console.print("✓ Test passed")

        except subprocess.TimeoutExpired:
            self.log_error(provider, model, extension,
                           "Test timed out after 30 seconds")
            console.print("✗ Test timed out", style="error")
        except Exception as e:
            self.log_error(provider, model, extension, str(e))
            console.print("✗ Test failed with unexpected error", style="error")
        finally:
            os.unlink(temp_path)

    def _run_serial(self, provider: str, model: str, extension: str) -> None:
        prompts = EXTENSION_PROMPTS.get(extension, [])
        for prompt in prompts:
            self.evaluate(provider, model, extension, prompt)
            time.sleep(2)  # brief pause between tests

    def _run_conversational(self, provider: str, model: str, extension: str) -> None:
        conv = CONV_EXTENSION_PROMPTS.get(extension, [])
        for t in conv.topics:
            self.evaluate(
                provider, model, extension, t.initial_prompt, t.follow_ups
            )
            time.sleep(2)  # brief pause between tests

    def test_extension(self, provider: str, model: str, extension: str) -> None:
        """Test all prompts for a given extension."""
        console.rule(f"Testing extension: {extension}")

        if self.workflow == Workflow.CONVERSATIONAL:
            return self._run_conversational(provider, model, extension)

        return self._run_serial(provider, model, extension)


def parse_provider_model(ctx: typer.Context, provider_models: List[str]) -> List[
    tuple[str, str]]:
    """Parse provider:model strings into tuples."""
    result = []
    for pm in provider_models:
        try:
            provider, models = pm.split(':')
            for model in models.split(','):
                result.append((provider.strip(), model.strip()))
        except ValueError:
            raise typer.BadParameter(
                f"Invalid format: {pm}. Use format 'provider:model' or 'provider:model1,model2'"
            )
    return result


@app.command()
def main(
        provider_models: Annotated[
            Optional[List[str]],
            typer.Option(
                '--provider-model', '-pm',
                help="Provider and model in format 'provider:model' or 'provider:model1,model2'"
            )
        ] = None,
        workflow: Annotated[
            Workflow,
            typer.Option(
                '--workflow', '-w',
                help="Workflow type: serial or conversational"
            )
        ] = Workflow.SERIAL,
        verbose: Annotated[
            bool,
            typer.Option('--verbose', '-v', help="Enable verbose output")
        ] = False,
):
    """
    Run Goose CLI Integration Tests.
    
    Example usage:
    
    python main.py  # Uses default: databricks:goose with serial workflow
    python main.py -pm anthropic:claude
    python main.py -pm anthropic:claude,claude2
    python main.py -pm anthropic:claude -pm databricks:goose
    python main.py --workflow conversational  # Use conversational workflow
    """
    console.print("Starting Goose CLI Integration Tests", style="bold")

    runner = Bench(workflow)

    # Use default if no provider-models specified
    if not provider_models:
        provider_models = ['databricks:goose']

    # Parse provider-model pairs
    try:
        provider_model_pairs = parse_provider_model(typer.Context, provider_models)
    except typer.BadParameter as e:
        console.print(f"Error: {str(e)}", style="error")
        raise typer.Exit(1)

    for provider, model in provider_model_pairs:
        console.rule(f"Testing provider: {provider}")
        console.print(f"Testing model: {model}", style="bold")

        for extension in EXTENSIONS:
            runner.test_extension(provider, model, extension)

    # Print summary
    if not runner.error_log:
        console.print("\nAll tests completed successfully!", style="success")
    else:
        console.print("\nTest Summary - Errors Found:", style="error")
        console.rule("Errors")
        for error in runner.error_log:
            console.print(error, style="error")
        raise typer.Exit(1)


if __name__ == "__main__":
    app()
