#!/usr/bin/env python3
"""
Goose Error Injection Controller

A simple CLI tool to control error injection in a running goose instance
that's using the error_proxy provider.

Usage:
    # Enable rate limit errors every 3rd call
    ./goose-error-control.py enable rate_limit --pattern every_nth --nth 3

    # Enable random context errors with 30% probability
    ./goose-error-control.py enable context_exceeded --pattern random --probability 0.3

    # Enable burst of 5 server errors
    ./goose-error-control.py enable server_error --pattern burst --burst-count 5

    # Disable error injection
    ./goose-error-control.py disable

    # Show current status
    ./goose-error-control.py status

    # Watch the control file for changes
    ./goose-error-control.py watch
"""

import argparse
import json
import os
import sys
import time
from pathlib import Path
from typing import Optional, Dict, Any

DEFAULT_CONTROL_FILE = "/tmp/goose-error-control.json"

class ErrorController:
    def __init__(self, control_file: str = DEFAULT_CONTROL_FILE):
        self.control_file = Path(control_file)
        
    def read_config(self) -> Dict[str, Any]:
        """Read the current configuration."""
        if not self.control_file.exists():
            return {"enabled": False}
        
        try:
            with open(self.control_file, 'r') as f:
                return json.load(f)
        except (json.JSONDecodeError, IOError):
            return {"enabled": False}
    
    def write_config(self, config: Dict[str, Any]) -> None:
        """Write configuration to the control file."""
        # Ensure directory exists
        self.control_file.parent.mkdir(parents=True, exist_ok=True)
        
        with open(self.control_file, 'w') as f:
            json.dump(config, f, indent=2)
        
        print(f"✓ Configuration written to {self.control_file}")
    
    def enable(self, error_type: str, pattern: str = "every_nth", **kwargs) -> None:
        """Enable error injection with specified parameters."""
        config = {
            "enabled": True,
            "error_type": error_type,
            "pattern": pattern,
        }
        
        # Add pattern-specific parameters
        if pattern == "every_nth":
            config["nth"] = kwargs.get("nth", 3)
        elif pattern == "random":
            config["probability"] = kwargs.get("probability", 0.5)
        elif pattern == "burst":
            config["burst_count"] = kwargs.get("burst_count", 3)
        
        # Add optional parameters
        if "retry_after" in kwargs:
            config["retry_after_seconds"] = kwargs["retry_after"]
        if "target_models" in kwargs:
            config["target_models"] = kwargs["target_models"]
        if "message" in kwargs:
            config["custom_message"] = kwargs["message"]
        
        self.write_config(config)
        print(f"✓ Error injection enabled: {error_type} with {pattern} pattern")
    
    def disable(self) -> None:
        """Disable error injection."""
        config = {"enabled": False}
        self.write_config(config)
        print("✓ Error injection disabled")
    
    def status(self) -> None:
        """Show current configuration status."""
        config = self.read_config()
        
        print("=== Goose Error Injection Status ===")
        print(f"Control file: {self.control_file}")
        print()
        
        if not config.get("enabled", False):
            print("Status: DISABLED")
        else:
            print("Status: ENABLED")
            print(f"Error type: {config.get('error_type', 'unknown')}")
            print(f"Pattern: {config.get('pattern', 'unknown')}")
            
            # Show pattern-specific parameters
            if config.get("pattern") == "every_nth":
                print(f"  Every: {config.get('nth', 3)} calls")
            elif config.get("pattern") == "random":
                print(f"  Probability: {config.get('probability', 0.5):.1%}")
            elif config.get("pattern") == "burst":
                print(f"  Burst count: {config.get('burst_count', 3)}")
            
            # Show optional parameters
            if "retry_after_seconds" in config:
                print(f"Retry after: {config['retry_after_seconds']} seconds")
            if "target_models" in config:
                print(f"Target models: {', '.join(config['target_models'])}")
            if "custom_message" in config:
                print(f"Custom message: {config['custom_message']}")
        
        print("\nRaw configuration:")
        print(json.dumps(config, indent=2))
    
    def watch(self, interval: float = 1.0) -> None:
        """Watch the control file for changes."""
        print(f"Watching {self.control_file} for changes (Ctrl+C to stop)...")
        last_config = None
        
        try:
            while True:
                config = self.read_config()
                if config != last_config:
                    print(f"\n[{time.strftime('%H:%M:%S')}] Configuration changed:")
                    print(json.dumps(config, indent=2))
                    last_config = config
                time.sleep(interval)
        except KeyboardInterrupt:
            print("\nStopped watching")

def main():
    parser = argparse.ArgumentParser(
        description="Control error injection in goose error_proxy provider",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__
    )
    
    parser.add_argument(
        "--control-file", "-f",
        default=os.environ.get("ERROR_PROXY_CONTROL_FILE", DEFAULT_CONTROL_FILE),
        help=f"Path to control file (default: {DEFAULT_CONTROL_FILE})"
    )
    
    subparsers = parser.add_subparsers(dest="command", help="Commands")
    
    # Enable command
    enable_parser = subparsers.add_parser("enable", help="Enable error injection")
    enable_parser.add_argument(
        "error_type",
        choices=["rate_limit", "context_exceeded", "server_error", "auth_error", "timeout"],
        help="Type of error to inject"
    )
    enable_parser.add_argument(
        "--pattern", "-p",
        choices=["every_nth", "random", "burst", "continuous", "once"],
        default="every_nth",
        help="Pattern for error injection (default: every_nth)"
    )
    enable_parser.add_argument(
        "--nth", "-n",
        type=int,
        default=3,
        help="For every_nth pattern: inject error every N calls (default: 3)"
    )
    enable_parser.add_argument(
        "--probability", "-r",
        type=float,
        default=0.5,
        help="For random pattern: probability of error 0.0-1.0 (default: 0.5)"
    )
    enable_parser.add_argument(
        "--burst-count", "-b",
        type=int,
        default=3,
        help="For burst pattern: number of consecutive errors (default: 3)"
    )
    enable_parser.add_argument(
        "--retry-after",
        type=int,
        help="For rate_limit errors: retry after N seconds"
    )
    enable_parser.add_argument(
        "--target-models",
        nargs="+",
        help="Only inject errors for specific models"
    )
    enable_parser.add_argument(
        "--message", "-m",
        help="Custom error message"
    )
    
    # Disable command
    subparsers.add_parser("disable", help="Disable error injection")
    
    # Status command
    subparsers.add_parser("status", help="Show current status")
    
    # Watch command
    watch_parser = subparsers.add_parser("watch", help="Watch control file for changes")
    watch_parser.add_argument(
        "--interval", "-i",
        type=float,
        default=1.0,
        help="Check interval in seconds (default: 1.0)"
    )
    
    # Quick presets
    preset_parser = subparsers.add_parser("preset", help="Use a preset configuration")
    preset_parser.add_argument(
        "preset_name",
        choices=["flaky", "overloaded", "broken", "slow"],
        help="Preset configuration to use"
    )
    
    args = parser.parse_args()
    
    if not args.command:
        parser.print_help()
        return 1
    
    controller = ErrorController(args.control_file)
    
    if args.command == "enable":
        kwargs = {}
        if args.pattern == "every_nth":
            kwargs["nth"] = args.nth
        elif args.pattern == "random":
            kwargs["probability"] = args.probability
        elif args.pattern == "burst":
            kwargs["burst_count"] = args.burst_count
        
        if args.retry_after:
            kwargs["retry_after"] = args.retry_after
        if args.target_models:
            kwargs["target_models"] = args.target_models
        if args.message:
            kwargs["message"] = args.message
        
        controller.enable(args.error_type, args.pattern, **kwargs)
    
    elif args.command == "disable":
        controller.disable()
    
    elif args.command == "status":
        controller.status()
    
    elif args.command == "watch":
        controller.watch(args.interval)
    
    elif args.command == "preset":
        presets = {
            "flaky": {
                "error_type": "server_error",
                "pattern": "random",
                "probability": 0.2,
                "message": "Simulating flaky service"
            },
            "overloaded": {
                "error_type": "rate_limit",
                "pattern": "every_nth",
                "nth": 5,
                "retry_after": 30,
                "message": "Simulating overloaded API"
            },
            "broken": {
                "error_type": "server_error",
                "pattern": "continuous",
                "message": "Simulating broken service"
            },
            "slow": {
                "error_type": "timeout",
                "pattern": "random",
                "probability": 0.3,
                "message": "Simulating slow/timeout responses"
            }
        }
        
        preset = presets[args.preset_name]
        controller.enable(**preset)
        print(f"Applied preset: {args.preset_name}")
    
    return 0

if __name__ == "__main__":
    sys.exit(main())
