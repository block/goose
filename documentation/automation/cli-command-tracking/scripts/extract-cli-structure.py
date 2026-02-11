#!/usr/bin/env python3
"""
Extract CLI command structure from goose binary using --help output.

Usage:
    ./extract-cli-structure.py <goose-binary-path> > output/cli-structure.json
    
Example:
    ./extract-cli-structure.py /path/to/goose > output/new-cli-structure.json
"""

import sys
import subprocess
import json
import re
import os
from datetime import datetime, timezone
from typing import Dict, List, Optional, Tuple


def load_skip_commands() -> List[str]:
    """Load the list of commands to skip from config file."""
    script_dir = os.path.dirname(os.path.abspath(__file__))
    config_path = os.path.join(script_dir, '..', 'config', 'skip-commands.json')
    
    try:
        with open(config_path, 'r') as f:
            config = json.load(f)
            return [cmd['name'] for cmd in config.get('skip_commands', [])]
    except (FileNotFoundError, json.JSONDecodeError, KeyError) as e:
        print(f"Warning: Could not load skip-commands.json: {e}", file=sys.stderr)
        return []


SKIP_COMMANDS = load_skip_commands()


def run_help_command(binary_path: str, command_path: List[str], short: bool = False) -> str:
    """
    Run --help or -h on a command and return the output.
    
    Args:
        binary_path: Path to goose binary
        command_path: List of command parts (e.g., ['session', 'list'])
        short: If True, use -h instead of --help
        
    Returns:
        Help text output
    """
    cmd = [binary_path] + command_path + (['-h'] if short else ['--help'])
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=10)
        return result.stdout
    except subprocess.TimeoutExpired:
        print(f"Warning: Command timed out: {' '.join(cmd)}", file=sys.stderr)
        return ""
    except Exception as e:
        print(f"Warning: Failed to run {' '.join(cmd)}: {e}", file=sys.stderr)
        return ""


def parse_usage_line(help_text: str) -> Optional[str]:
    """Extract the usage line from help text."""
    match = re.search(r'^Usage:\s*(.+)$', help_text, re.MULTILINE)
    return match.group(1).strip() if match else None


def parse_about(help_text: str) -> str:
    """Extract the command description (first line before Usage)."""
    lines = help_text.strip().split('\n')
    
    # Find the Usage: line
    usage_index = -1
    for i, line in enumerate(lines):
        if line.strip().startswith('Usage:'):
            usage_index = i
            break
    
    # If Usage is found, look for description before it
    if usage_index > 0:
        for i in range(usage_index):
            line = lines[i].strip()
            if line and not line.startswith('Options:') and not line.startswith('Commands:'):
                return line
    
    return ""


def parse_aliases(help_text: str) -> List[str]:
    """Extract command aliases from help text."""
    # Look for "[aliases: x, y]" pattern in the first few lines
    match = re.search(r'\[aliases?:\s*([^\]]+)\]', help_text[:500])
    if match:
        aliases_str = match.group(1)
        return [a.strip() for a in aliases_str.split(',')]
    return []


def parse_options(help_text: str) -> List[Dict]:
    """
    Parse options from the Options: section of help text.
    
    Returns list of option dicts with: short, long, value_name, help, default, possible_values
    """
    options = []
    
    # Find the Options: section - goes until Commands: section or end of text
    # Note: clap help has blank lines between options, so we can't stop at ^$
    options_match = re.search(r'^Options:\s*\n(.+?)(?=^Commands:\s*$|\Z)', 
                             help_text, re.MULTILINE | re.DOTALL)
    if not options_match:
        return options
    
    options_text = options_match.group(1)
    
    # Split into individual option blocks
    # Each option starts with whitespace followed by a dash (short or long flag)
    # Use lookahead to split at lines that start a new option
    option_blocks = re.split(r'\n(?=\s+-)', options_text)
    
    for block in option_blocks:
        block = block.strip()
        if not block or not block.startswith('-'):
            continue
            
        option = parse_option_block(block)
        if option:
            options.append(option)
    
    return options


def parse_option_block(block: str) -> Optional[Dict]:
    """Parse a single option block into structured data."""
    lines = block.split('\n')
    if not lines:
        return None
    
    # First line has the flags, optional value name, and sometimes inline help (common clap output)
    first_line = lines[0].strip()
    inline_help = None

    # Split on 2+ spaces to separate flags from inline help text.
    # Example: "-o, --output <FILE>  Write output to file"
    parts = re.split(r'\s{2,}', first_line, maxsplit=1)
    flags_part = parts[0]
    if len(parts) == 2:
        inline_help = parts[1].strip() or None
    
    # Extract short flag (e.g., -f)
    short_match = re.search(r'-([a-zA-Z])\b', flags_part)
    short = short_match.group(1) if short_match else None
    
    # Extract long flag (e.g., --format)
    long_match = re.search(r'--([a-z][a-z0-9-]*)', flags_part)
    long = long_match.group(1) if long_match else None
    
    # Extract value_name (e.g., <FORMAT>)
    value_name_match = re.search(r'<([^>]+)>', flags_part)
    value_name = value_name_match.group(1) if value_name_match else None
    
    # Collect help text from subsequent indented lines
    help_lines = []
    for line in lines[1:]:
        line = line.strip()
        if line and not line.startswith('['):
            help_lines.append(line)
        elif line.startswith('['):
            # This might be [default: ...] or [possible values: ...]
            break
    
    help_text = ' '.join(help_lines)

    if inline_help:
        help_text = f"{inline_help} {help_text}".strip() if help_text else inline_help
    
    # Extract default value
    default = None
    default_match = re.search(r'\[default:\s*([^\]]+)\]', block)
    if default_match:
        default = default_match.group(1).strip()
    
    # Extract possible values
    possible_values = None
    possible_match = re.search(r'\[possible values:\s*([^\]]+)\]', block)
    if possible_match:
        values_str = possible_match.group(1)
        possible_values = [v.strip() for v in values_str.split(',')]
    
    return {
        'short': short,
        'long': long,
        'value_name': value_name,
        'help': help_text if help_text else None,
        'default': default,
        'possible_values': possible_values
    }


def parse_subcommands(help_text: str) -> List[Tuple[str, List[str]]]:
    """
    Extract subcommand names and their aliases from the Commands: section.
    
    Returns:
        List of tuples: (command_name, [aliases])
    """
    commands = []
    
    # Find the Commands: section
    commands_match = re.search(r'^Commands:\s*$(.+?)(?:^Options:|\Z)', 
                              help_text, re.MULTILINE | re.DOTALL)
    if not commands_match:
        return commands
    
    commands_text = commands_match.group(1)
    
    # Each command line starts with the command name (not indented or minimally indented)
    for raw_line in commands_text.split('\n'):
        # Preserve indentation to avoid mis-parsing wrapped description lines.
        # In clap help, actual command entries are typically not indented.
        if not raw_line.strip():
            continue

        if raw_line.startswith(' ') or raw_line.startswith('\t'):
            continue

        line = raw_line.strip()
        
        # Extract command name (first word)
        parts = line.split()
        if parts and not parts[0].startswith('-'):
            command_name = parts[0]
            # Skip "help" command as it's auto-generated
            if command_name == 'help':
                continue
            
            # Extract aliases from [aliases: x, y] pattern
            aliases = []
            alias_match = re.search(r'\[aliases?:\s*([^\]]+)\]', line)
            if alias_match:
                aliases_str = alias_match.group(1)
                aliases = [a.strip() for a in aliases_str.split(',')]
            
            commands.append((command_name, aliases))
    
    return commands


def extract_command_structure(binary_path: str, command_path: List[str] = None, 
                            parent_aliases: List[str] = None) -> Dict:
    """
    Recursively extract command structure starting from a command path.
    
    Args:
        binary_path: Path to goose binary
        command_path: Current command path (e.g., ['session', 'list'])
        parent_aliases: Aliases passed from parent (since they appear in parent's help)
        
    Returns:
        Dict with command structure
    """
    if command_path is None:
        command_path = []
    
    # Get both short and long help
    help_text_long = run_help_command(binary_path, command_path, short=False)
    
    if not help_text_long:
        return None
    
    # Parse command info
    command_name = command_path[-1] if command_path else "goose"
    about = parse_about(help_text_long)
    # Use parent_aliases if provided, otherwise try to parse from own help
    aliases = parent_aliases if parent_aliases is not None else parse_aliases(help_text_long)
    usage = parse_usage_line(help_text_long)
    options = parse_options(help_text_long)
    
    # Get subcommands with their aliases and recursively process them
    subcommand_info = parse_subcommands(help_text_long)
    subcommands = []
    
    for subcommand_name, subcommand_aliases in subcommand_info:
        # Skip commands in the skip list
        if subcommand_name in SKIP_COMMANDS:
            print(f"Skipping command: {subcommand_name}", file=sys.stderr)
            continue
        sub_path = command_path + [subcommand_name]
        sub_structure = extract_command_structure(binary_path, sub_path, subcommand_aliases)
        if sub_structure:
            subcommands.append(sub_structure)
    
    return {
        'name': command_name,
        'about': about,
        'aliases': aliases,
        'usage': usage,
        'options': options,
        'subcommands': subcommands
    }


def extract_version(binary_path: str) -> str:
    """Extract version from goose --version."""
    try:
        result = subprocess.run([binary_path, '--version'], 
                              capture_output=True, text=True, timeout=5)
        # Output is typically "goose 1.15.0" or similar
        version_match = re.search(r'(\d+\.\d+\.\d+)', result.stdout)
        return version_match.group(1) if version_match else "unknown"
    except Exception as e:
        print(f"Warning: Could not extract version: {e}", file=sys.stderr)
        return "unknown"


def main():
    if len(sys.argv) < 2:
        print("Usage: extract-cli-structure.py <goose-binary-path> [source-version]", file=sys.stderr)
        print("Example: extract-cli-structure.py /usr/local/bin/goose v1.15.0", file=sys.stderr)
        sys.exit(1)
    
    binary_path = sys.argv[1]
    source_version = sys.argv[2] if len(sys.argv) > 2 else None
    
    # Verify binary exists and is executable
    try:
        result = subprocess.run([binary_path, '--version'], 
                              capture_output=True, timeout=5)
        if result.returncode != 0:
            print(f"Error: {binary_path} is not a valid goose binary", file=sys.stderr)
            sys.exit(1)
    except Exception as e:
        print(f"Error: Cannot execute {binary_path}: {e}", file=sys.stderr)
        sys.exit(1)
    
    print("Extracting CLI structure...", file=sys.stderr)
    
    # Extract version
    version = extract_version(binary_path)
    print(f"Version: {version}", file=sys.stderr)
    
    # Extract root command structure (recursively includes all subcommands)
    root_structure = extract_command_structure(binary_path, [])
    
    # Build output JSON
    # Use timezone-aware UTC datetime (Python 3.7+)
    now = datetime.now(timezone.utc).isoformat().replace('+00:00', 'Z')
    
    output = {
        'version': version,
        'source_version': source_version or version,
        'extracted_at': now,
        'binary_path': binary_path,
        'commands': root_structure['subcommands'] if root_structure else []
    }
    
    # Output JSON
    print(json.dumps(output, indent=2))
    print("Extraction complete!", file=sys.stderr)


if __name__ == '__main__':
    main()
