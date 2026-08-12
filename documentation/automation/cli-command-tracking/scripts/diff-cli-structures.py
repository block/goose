#!/usr/bin/env python3
"""
Compare two CLI structure JSON files and output detected changes.

Usage:
    ./diff-cli-structures.py <old-file> <new-file> > output/cli-changes.json
    
Example:
    ./diff-cli-structures.py output/cli-structure-v1.14.0.json \
                            output/cli-structure-v1.15.0.json \
                            > output/cli-changes.json
"""

import sys
import json
from datetime import datetime, timezone
from typing import Dict, List


def get_command_path(command: Dict, parent_path: str = "") -> str:
    """Get the full path of a command (e.g., 'session list')."""
    if parent_path:
        return f"{parent_path} {command['name']}"
    return command['name']


def flatten_commands(commands: List[Dict], parent_path: str = "") -> Dict[str, Dict]:
    """
    Flatten nested command structure into a dict keyed by full command path.
    
    Returns:
        Dict mapping command path to command data
    """
    result = {}
    
    for cmd in commands:
        cmd_path = get_command_path(cmd, parent_path)
        # Store command without subcommands to avoid recursion in comparisons
        cmd_copy = cmd.copy()
        subcommands = cmd_copy.pop('subcommands', [])
        result[cmd_path] = cmd_copy
        
        # Recursively flatten subcommands
        if subcommands:
            result.update(flatten_commands(subcommands, cmd_path))
    
    return result


def compare_options(old_opts: List[Dict], new_opts: List[Dict]) -> Dict:
    """
    Compare two lists of options and detect changes.
    
    Returns dict with: added, removed, modified
    """
    # Create dicts keyed by long flag (or short if no long)
    old_opts_dict = {opt.get('long') or opt.get('short'): opt for opt in old_opts}
    new_opts_dict = {opt.get('long') or opt.get('short'): opt for opt in new_opts}
    
    old_keys = set(old_opts_dict.keys())
    new_keys = set(new_opts_dict.keys())
    
    added = []
    removed = []
    modified = []
    
    # Find added options
    for key in new_keys - old_keys:
        added.append(new_opts_dict[key])
    
    # Find removed options
    for key in old_keys - new_keys:
        removed.append(old_opts_dict[key])
    
    # Find modified options
    for key in old_keys & new_keys:
        old_opt = old_opts_dict[key]
        new_opt = new_opts_dict[key]
        
        changes = {}
        
        # Check each field for changes
        if old_opt.get('short') != new_opt.get('short'):
            changes['short'] = {'old': old_opt.get('short'), 'new': new_opt.get('short')}
        
        if old_opt.get('long') != new_opt.get('long'):
            changes['long'] = {'old': old_opt.get('long'), 'new': new_opt.get('long')}
        
        if old_opt.get('value_name') != new_opt.get('value_name'):
            changes['value_name'] = {'old': old_opt.get('value_name'), 'new': new_opt.get('value_name')}
        
        if old_opt.get('help') != new_opt.get('help'):
            changes['help'] = {'old': old_opt.get('help'), 'new': new_opt.get('help')}
        
        if old_opt.get('default') != new_opt.get('default'):
            changes['default'] = {'old': old_opt.get('default'), 'new': new_opt.get('default')}
        
        if old_opt.get('possible_values') != new_opt.get('possible_values'):
            changes['possible_values'] = {'old': old_opt.get('possible_values'), 'new': new_opt.get('possible_values')}
        
        if changes:
            modified.append({
                'option': key,
                'changes': changes
            })
    
    return {
        'added': added,
        'removed': removed,
        'modified': modified
    }


def compare_commands(old_cmds: Dict[str, Dict], new_cmds: Dict[str, Dict]) -> Dict:
    """
    Compare two command dictionaries and detect changes.
    
    Returns dict with: added, removed, modified
    """
    old_paths = set(old_cmds.keys())
    new_paths = set(new_cmds.keys())
    
    added = []
    removed = []
    modified = []
    
    # Find added commands
    for path in new_paths - old_paths:
        added.append({
            'command': path,
            'data': new_cmds[path]
        })
    
    # Find removed commands
    for path in old_paths - new_paths:
        removed.append({
            'command': path,
            'data': old_cmds[path]
        })
    
    # Find modified commands
    for path in old_paths & new_paths:
        old_cmd = old_cmds[path]
        new_cmd = new_cmds[path]
        
        changes = {}
        
        # Check about text
        if old_cmd.get('about') != new_cmd.get('about'):
            changes['about'] = {
                'old': old_cmd.get('about'),
                'new': new_cmd.get('about')
            }
        
        # Check aliases
        old_aliases = set(old_cmd.get('aliases', []))
        new_aliases = set(new_cmd.get('aliases', []))
        if old_aliases != new_aliases:
            changes['aliases'] = {
                'old': sorted(old_aliases),
                'new': sorted(new_aliases),
                'added': sorted(new_aliases - old_aliases),
                'removed': sorted(old_aliases - new_aliases)
            }
        
        # Check usage
        if old_cmd.get('usage') != new_cmd.get('usage'):
            changes['usage'] = {
                'old': old_cmd.get('usage'),
                'new': new_cmd.get('usage')
            }
        
        # Check options
        option_changes = compare_options(
            old_cmd.get('options', []),
            new_cmd.get('options', [])
        )
        if any(option_changes.values()):
            changes['options'] = option_changes
        
        if changes:
            modified.append({
                'command': path,
                'changes': changes
            })
    
    return {
        'added': added,
        'removed': removed,
        'modified': modified
    }


def categorize_breaking_changes(changes: Dict) -> List[Dict]:
    """
    Identify changes that are likely breaking changes.
    
    Returns list of breaking change descriptions.
    """
    breaking = []
    
    # Removed commands are breaking
    for item in changes['commands']['removed']:
        breaking.append({
            'type': 'command_removed',
            'command': item['command'],
            'severity': 'high',
            'description': f"Command '{item['command']}' was removed"
        })
    
    # Check modified commands for breaking changes
    for item in changes['commands']['modified']:
        cmd = item['command']
        cmd_changes = item['changes']
        
        # Removed options are breaking
        if 'options' in cmd_changes:
            for opt in cmd_changes['options']['removed']:
                opt_name = f"--{opt.get('long')}" if opt.get('long') else f"-{opt.get('short')}"
                breaking.append({
                    'type': 'option_removed',
                    'command': cmd,
                    'option': opt_name,
                    'severity': 'high',
                    'description': f"Option '{opt_name}' removed from '{cmd}'"
                })
            
            # Changed option flags are breaking
            for mod in cmd_changes['options']['modified']:
                if 'short' in mod['changes'] or 'long' in mod['changes']:
                    breaking.append({
                        'type': 'option_renamed',
                        'command': cmd,
                        'option': mod['option'],
                        'severity': 'high',
                        'description': f"Option flags changed in '{cmd}': {mod['option']}"
                    })
                
                # Changed default values might be breaking
                if 'default' in mod['changes']:
                    breaking.append({
                        'type': 'default_changed',
                        'command': cmd,
                        'option': mod['option'],
                        'severity': 'medium',
                        'description': f"Default value changed for '{cmd} --{mod['option']}'"
                    })
                
                # Removed possible values are breaking
                if 'possible_values' in mod['changes']:
                    old_vals = set(mod['changes']['possible_values']['old'] or [])
                    new_vals = set(mod['changes']['possible_values']['new'] or [])
                    removed_vals = old_vals - new_vals
                    if removed_vals:
                        breaking.append({
                            'type': 'enum_values_removed',
                            'command': cmd,
                            'option': mod['option'],
                            'severity': 'high',
                            'description': f"Possible values removed from '{cmd} --{mod['option']}': {', '.join(removed_vals)}"
                        })
        
        # Removed aliases might be breaking (users might rely on them)
        if 'aliases' in cmd_changes and cmd_changes['aliases']['removed']:
            for alias in cmd_changes['aliases']['removed']:
                breaking.append({
                    'type': 'alias_removed',
                    'command': cmd,
                    'alias': alias,
                    'severity': 'medium',
                    'description': f"Alias '{alias}' removed from '{cmd}'"
                })
    
    return breaking


def main():
    if len(sys.argv) != 3:
        print("Usage: diff-cli-structures.py <old-file> <new-file>", file=sys.stderr)
        print("Example: diff-cli-structures.py old.json new.json", file=sys.stderr)
        sys.exit(1)
    
    old_file = sys.argv[1]
    new_file = sys.argv[2]
    
    # Load JSON files
    try:
        with open(old_file, 'r') as f:
            old_data = json.load(f)
    except Exception as e:
        print(f"Error reading {old_file}: {e}", file=sys.stderr)
        sys.exit(1)
    
    try:
        with open(new_file, 'r') as f:
            new_data = json.load(f)
    except Exception as e:
        print(f"Error reading {new_file}: {e}", file=sys.stderr)
        sys.exit(1)
    
    print(f"Comparing {old_data['version']} → {new_data['version']}...", file=sys.stderr)
    
    # Flatten command structures
    old_commands = flatten_commands(old_data['commands'])
    new_commands = flatten_commands(new_data['commands'])
    
    print(f"Old version: {len(old_commands)} commands", file=sys.stderr)
    print(f"New version: {len(new_commands)} commands", file=sys.stderr)
    
    # Compare commands
    command_changes = compare_commands(old_commands, new_commands)
    
    # Categorize breaking changes
    breaking_changes = categorize_breaking_changes({'commands': command_changes})
    
    # Determine if there are any changes
    has_changes = (
        len(command_changes['added']) > 0 or
        len(command_changes['removed']) > 0 or
        len(command_changes['modified']) > 0
    )
    
    # Build output
    now = datetime.now(timezone.utc).isoformat().replace('+00:00', 'Z')
    
    output = {
        'old_version': old_data['version'],
        'new_version': new_data['version'],
        'compared_at': now,
        'has_changes': has_changes,
        'summary': {
            'commands_added': len(command_changes['added']),
            'commands_removed': len(command_changes['removed']),
            'commands_modified': len(command_changes['modified']),
            'breaking_changes': len([b for b in breaking_changes if b['severity'] == 'high'])
        },
        'changes': {
            'commands': command_changes
        },
        'breaking_changes': breaking_changes
    }
    
    # Output JSON
    print(json.dumps(output, indent=2))
    
    # Print summary to stderr
    print(f"\nSummary:", file=sys.stderr)
    print(f"  Commands added: {output['summary']['commands_added']}", file=sys.stderr)
    print(f"  Commands removed: {output['summary']['commands_removed']}", file=sys.stderr)
    print(f"  Commands modified: {output['summary']['commands_modified']}", file=sys.stderr)
    print(f"  Breaking changes: {output['summary']['breaking_changes']}", file=sys.stderr)


if __name__ == '__main__':
    main()
