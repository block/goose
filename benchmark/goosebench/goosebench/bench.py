import os
import subprocess
import tempfile
import time
from typing import Optional, List

from goosebench.main import console, EXTENSION_PROMPTS


class Bench:
    def __init__(self):
        self.error_log = []

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

    def test_extension(self, provider: str, model: str, extension: str) -> None:
        """Test all prompts for a given extension."""
        console.rule(f"Testing extension: {extension}")
        return self._run_serial(provider, model, extension)
