#!/usr/bin/env python3
"""
Test the command parsing logic for the proxy.
"""

def parse_command(command: str):
    """Parse a command string and return the parsed components."""
    # Remove all whitespace
    command_no_space = command.replace(" ", "")
    if not command_no_space:
        return None, None, None
    
    # Get the first character (error type letter)
    error_letter = command_no_space[0].lower()
    
    # Map letter to error type
    mode_map = {
        'n': 'NO_ERROR',
        'c': 'CONTEXT_LENGTH',
        'r': 'RATE_LIMIT',
        'u': 'SERVER_ERROR',
        'q': 'QUIT'
    }
    
    if error_letter not in mode_map:
        return None, None, None
    
    mode = mode_map[error_letter]
    
    # Parse the rest as count or percentage
    count = 1
    percentage = 0.0
    
    if len(command_no_space) > 1:
        value_str = command_no_space[1:]
        
        try:
            # Check for * (100%)
            if value_str == '*':
                percentage = 1.0
                count = 0  # Percentage mode
            # Check for percentage with % sign (e.g., "30%")
            elif value_str.endswith('%'):
                percentage = float(value_str[:-1]) / 100.0
                count = 0  # Percentage mode
            # Check if it's a decimal (percentage as 0.0-1.0)
            elif '.' in value_str:
                percentage = float(value_str)
                count = 0  # Percentage mode
            else:
                # It's an integer count
                count = int(value_str)
        except ValueError:
            return None, None, None
    
    return mode, count, percentage


# Test cases
test_cases = [
    ("n", "NO_ERROR", 1, 0.0),
    ("c", "CONTEXT_LENGTH", 1, 0.0),
    ("c4", "CONTEXT_LENGTH", 4, 0.0),
    ("c 4", "CONTEXT_LENGTH", 4, 0.0),
    ("c  4", "CONTEXT_LENGTH", 4, 0.0),
    ("c0.3", "CONTEXT_LENGTH", 0, 0.3),
    ("c 0.3", "CONTEXT_LENGTH", 0, 0.3),
    ("c30%", "CONTEXT_LENGTH", 0, 0.3),
    ("c 30%", "CONTEXT_LENGTH", 0, 0.3),
    ("c 100%", "CONTEXT_LENGTH", 0, 1.0),
    ("c*", "CONTEXT_LENGTH", 0, 1.0),
    ("c *", "CONTEXT_LENGTH", 0, 1.0),
    ("r", "RATE_LIMIT", 1, 0.0),
    ("r 2", "RATE_LIMIT", 2, 0.0),
    ("r0.5", "RATE_LIMIT", 0, 0.5),
    ("u", "SERVER_ERROR", 1, 0.0),
    ("u 10", "SERVER_ERROR", 10, 0.0),
    ("u50%", "SERVER_ERROR", 0, 0.5),
]

print("Testing command parsing logic:")
print("=" * 70)

all_passed = True
for cmd, expected_mode, expected_count, expected_pct in test_cases:
    mode, count, pct = parse_command(cmd)
    passed = (mode == expected_mode and count == expected_count and abs(pct - expected_pct) < 0.001)
    status = "✅" if passed else "❌"
    
    if not passed:
        all_passed = False
        print(f"{status} '{cmd:10}' -> mode={mode}, count={count}, pct={pct:.2f}")
        print(f"   Expected: mode={expected_mode}, count={expected_count}, pct={expected_pct:.2f}")
    else:
        print(f"{status} '{cmd:10}' -> mode={mode}, count={count}, pct={pct:.2f}")

print("=" * 70)
if all_passed:
    print("✅ All tests passed!")
else:
    print("❌ Some tests failed!")
