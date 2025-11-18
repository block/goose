#!/usr/bin/env python3
"""
Community Stars Analysis Script for block/goose repository

This script analyzes GitHub contributor statistics and generates rankings for:
- Top 5 Community All-Stars (External contributors)
- Top 5 Team Stars (Block employees, non-goose team)
- Monthly Leaderboard (all eligible contributors)

Usage:
    python3 community_stars.py "November 2025"
    python3 community_stars.py "November 1, 2025 - November 17, 2025"
    python3 community_stars.py "2025-11-01 - 2025-11-17"

Requirements:
    - GitHub contributor data at /tmp/github_contributors.json
    - Team list file (local or from GitHub)
"""

import json
import re
import sys
import urllib.request
from datetime import datetime
import calendar
from pathlib import Path

# GitHub URL for team list file
TEAMS_FILE_URL = "https://raw.githubusercontent.com/block/goose/main/documentation/scripts/community_stars_teams.txt"
LOCAL_TEAMS_FILE = Path(__file__).parent / "community_stars_teams.txt"

def load_team_lists():
    """Load and parse team lists from file (local or GitHub)."""
    content = None
    
    # Try local file first
    if LOCAL_TEAMS_FILE.exists():
        with open(LOCAL_TEAMS_FILE, 'r') as f:
            content = f.read()
    else:
        # Fall back to GitHub
        try:
            with urllib.request.urlopen(TEAMS_FILE_URL) as response:
                content = response.read().decode('utf-8')
        except Exception as e:
            print(f"Error: Could not load team list file from {TEAMS_FILE_URL}")
            print(f"Details: {e}")
            sys.exit(1)
    
    # Parse the team lists
    goose_maintainers = set()
    block_non_goose = set()
    external_goose = set()
    external = set()
    bots = set()
    
    current_section = None
    for line in content.split('\n'):
        line = line.strip()
        
        # Skip comments and empty lines
        if not line or line.startswith('#'):
            # Check for section headers in comments
            if '# Goose Maintainers' in line:
                current_section = 'goose_maintainers'
            elif '# Block, non-goose' in line:
                current_section = 'block_non_goose'
            elif '# External, goose' in line:
                current_section = 'external_goose'
            elif line.startswith('# External') and 'goose' not in line.lower():
                current_section = 'external'
            elif '# Bots' in line:
                current_section = 'bots'
            continue
        
        # Add username to appropriate set (lowercase for case-insensitive matching)
        # Apply .lower() to entire username including brackets (e.g., "dependabot[bot]")
        # This matches the pattern used above: 'goose' not in line.lower()
        username = line.lower()
        if current_section == 'goose_maintainers':
            goose_maintainers.add(username)
        elif current_section == 'block_non_goose':
            block_non_goose.add(username)
        elif current_section == 'external_goose':
            external_goose.add(username)
        elif current_section == 'external':
            external.add(username)
        elif current_section == 'bots':
            bots.add(username)
    
    return goose_maintainers, block_non_goose, external_goose, external, bots

def parse_date_range(date_input):
    """Parse various date input formats and return start/end timestamps."""
    date_input = date_input.strip()
    
    # Format: "Month YYYY" (e.g., "November 2025")
    month_year_pattern = r'^(January|February|March|April|May|June|July|August|September|October|November|December)\s+(\d{4})$'
    match = re.match(month_year_pattern, date_input, re.IGNORECASE)
    if match:
        month_name = match.group(1).capitalize()
        year = int(match.group(2))
        start_date = datetime.strptime(f"{month_name} 1 {year}", "%B %d %Y")
        last_day = calendar.monthrange(year, start_date.month)[1]
        end_date = datetime(year, start_date.month, last_day, 23, 59, 59)
        return start_date.timestamp(), end_date.timestamp(), date_input
    
    # Format: "Date1 - Date2" (e.g., "November 1, 2025 - November 17, 2025" or "2025-11-01 - 2025-11-17")
    if ' - ' in date_input or ' to ' in date_input:
        separator = ' - ' if ' - ' in date_input else ' to '
        parts = date_input.split(separator)
        if len(parts) == 2:
            date_formats = ["%B %d, %Y", "%b %d, %Y", "%Y-%m-%d"]
            start_date = None
            end_date = None
            
            for fmt in date_formats:
                try:
                    start_date = datetime.strptime(parts[0].strip(), fmt)
                    end_date = datetime.strptime(parts[1].strip(), fmt)
                    break
                except ValueError:
                    continue
            
            if start_date and end_date:
                end_date = datetime(end_date.year, end_date.month, end_date.day, 23, 59, 59)
                return start_date.timestamp(), end_date.timestamp(), date_input
    
    raise ValueError(f"Could not parse date input: {date_input}\nSupported formats:\n  - 'Month YYYY' (e.g., 'November 2025')\n  - 'Month Day, YYYY - Month Day, YYYY' (e.g., 'November 1, 2025 - November 17, 2025')\n  - 'YYYY-MM-DD - YYYY-MM-DD' (e.g., '2025-11-01 - 2025-11-17')")

def main():
    # Parse command line arguments
    if len(sys.argv) < 2:
        print("Usage: python3 community_stars.py 'date_range'")
        print("Examples:")
        print("  python3 community_stars.py 'November 2025'")
        print("  python3 community_stars.py 'November 1, 2025 - November 17, 2025'")
        print("  python3 community_stars.py '2025-11-01 - 2025-11-17'")
        sys.exit(1)

    date_input = sys.argv[1]
    try:
        start_timestamp, end_timestamp, display_period = parse_date_range(date_input)
        start_date = datetime.fromtimestamp(start_timestamp)
        end_date = datetime.fromtimestamp(end_timestamp)
    except ValueError as e:
        print(f"Error: {e}")
        sys.exit(1)

    # Load team lists
    goose_maintainers, block_non_goose, external_goose, external, bots = load_team_lists()

    # Load GitHub data
    github_data_file = '/tmp/github_contributors.json'
    try:
        with open(github_data_file, 'r') as f:
            contributors_data = json.load(f)
    except FileNotFoundError:
        print(f"Error: GitHub contributor data not found at {github_data_file}")
        print("Please run: curl -s -H 'Accept: application/vnd.github.v3+json' 'https://api.github.com/repos/block/goose/stats/contributors' > /tmp/github_contributors.json")
        sys.exit(1)
    except json.JSONDecodeError as e:
        print(f"Error: Invalid JSON in {github_data_file}")
        print(f"Details: {e}")
        print("The GitHub API may have returned an error. Try fetching the data again.")
        sys.exit(1)

    # Process contributors
    contributor_stats = []

    for contributor in contributors_data:
        # Skip if author is None (deleted users)
        if contributor.get('author') is None:
            continue
            
        username = contributor['author']['login']
        username_lower = username.lower()
        
        # Skip excluded categories (case-insensitive matching)
        if username_lower in bots or username_lower in goose_maintainers or username_lower in external_goose:
            continue
        
        # Calculate stats for the specified period
        period_commits = 0
        period_additions = 0
        period_deletions = 0
        
        for week in contributor['weeks']:
            week_timestamp = week['w']
            if start_timestamp <= week_timestamp <= end_timestamp:
                period_commits += week['c']
                period_additions += week['a']
                period_deletions += week['d']
        
        # Only include contributors with activity in the period
        if period_commits > 0:
            total_lines = period_additions + period_deletions
            
            # Categorize (only Block non-goose and External now)
            if username_lower in block_non_goose:
                category = 'block_non_goose'
            elif username_lower in external:
                category = 'external'
            else:
                category = 'unknown'
            
            contributor_stats.append({
                'username': username,
                'category': category,
                'commits': period_commits,
                'additions': period_additions,
                'deletions': period_deletions,
                'total_lines': total_lines,
                'score': period_commits + total_lines
            })

    # Sort by score
    contributor_stats.sort(key=lambda x: x['score'], reverse=True)

    # Separate by category
    block_list = [c for c in contributor_stats if c['category'] == 'block_non_goose']
    external_list = [c for c in contributor_stats if c['category'] == 'external']
    unknown_list = [c for c in contributor_stats if c['category'] == 'unknown']

    # Get top 5 from each
    top_external = external_list[:5]
    top_internal = block_list[:5]

    # Print results
    print("=" * 70)
    print(f"COMMUNITY STARS - {display_period.upper()}")
    print(f"(Period: {start_date.strftime('%B %d, %Y')} - {end_date.strftime('%B %d, %Y')})")
    print("=" * 70)
    print()

    print("🏆 TOP 5 COMMUNITY ALL-STARS (External Contributors)")
    print("-" * 70)
    if top_external:
        for i, contrib in enumerate(top_external, 1):
            print(f"{i}. @{contrib['username']:20s} - {contrib['commits']:3d} commits, {contrib['total_lines']:6,d} lines")
    else:
        print("No external contributors found for this period.")

    print()
    print("⭐ TOP 5 TEAM STARS (Block, non-goose)")
    print("-" * 70)
    if top_internal:
        for i, contrib in enumerate(top_internal, 1):
            print(f"{i}. @{contrib['username']:20s} - {contrib['commits']:3d} commits, {contrib['total_lines']:6,d} lines")
    else:
        print("No internal contributors found for this period.")

    print()
    print("📊 MONTHLY LEADERBOARD (All Contributors)")
    print("-" * 70)
    if contributor_stats:
        for i, contrib in enumerate(contributor_stats, 1):
            cat_label = "External" if contrib['category'] == 'external' else "Block" if contrib['category'] == 'block_non_goose' else "Unknown"
            print(f"{i:2d}. @{contrib['username']:20s} - {contrib['commits']:3d} commits, {contrib['total_lines']:6,d} lines [{cat_label}]")
    else:
        print("No contributors found for this period.")

    if unknown_list:
        print()
        print("⚠️  UNKNOWN CONTRIBUTORS (not in team lists):")
        print("-" * 70)
        for contrib in unknown_list:
            print(f"   @{contrib['username']:20s} - {contrib['commits']:3d} commits, {contrib['total_lines']:6,d} lines")

    print()
    print("=" * 70)
    print(f"Total contributors (excluding bots, goose maintainers, external goose): {len(contributor_stats)}")
    print(f"  External: {len(external_list)}")
    print(f"  Block (non-goose): {len(block_list)}")
    if unknown_list:
        print(f"  Unknown: {len(unknown_list)}")
    print("=" * 70)

if __name__ == "__main__":
    main()
