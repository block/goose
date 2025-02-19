#!/usr/bin/env python3
from typing import List, Optional

import typer
from rich.console import Console
from rich.theme import Theme
from typing_extensions import Annotated

from goosebench.bench import Bench

# Initialize typer app and rich console
app = typer.Typer(help="Goose CLI Integration Tests")
console = Console(theme=Theme({
    "info": "cyan",
    "warning": "yellow",
    "error": "red",
    "success": "green"
}))

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
        verbose: Annotated[
            bool,
            typer.Option('--verbose', '-v', help="Enable verbose output")
        ] = False,
):
    """
    Run Goose CLI Integration Tests.
    
    Example usage:
    
    python main.py  # Uses default: databricks:goose
    python main.py -pm anthropic:claude
    python main.py -pm anthropic:claude,claude2
    python main.py -pm anthropic:claude -pm databricks:goose
    """
    console.print("Starting Goose CLI Integration Tests", style="bold")

    runner = Bench()

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
