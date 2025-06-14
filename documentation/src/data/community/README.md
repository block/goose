# Community All Stars - Monthly Update Guide

This directory contains the data files for the Community All Stars section on the community page.

## Monthly Update Process

### Step 1: Create New Month Data File

1. Copy `template.json` to create a new file named `MONTH-YEAR.json` (e.g., `june-2025.json`)
2. Update the data in the new file:

```json
{
  "month": "June 2025",
  "featuredContributors": [
    {
      "name": "John Doe",
      "handle": "johndoe"
    }
  ],
  "risingStars": [
    {
      "name": "Jane Smith", 
      "handle": "janesmith"
    }
  ],
  "leaderboard": [
    { "handle": "johndoe", "rank": 1, "medal": "🥇" },
    { "handle": "janesmith", "rank": 2, "medal": "🥈" },
    { "handle": "contributor3", "rank": 3, "medal": "🥉" },
    { "handle": "contributor4", "rank": 4 }
  ]
}
```

### Step 2: Update Configuration

1. Open `config.json`
2. Add the new month to the `availableMonths` array:

```json
{
  "availableMonths": [
    {
      "id": "june-2025",
      "display": "June 2025", 
      "file": "june-2025.json"
    }
  ],
  "defaultMonth": "june-2025"
}
```

3. Update `defaultMonth` to the new month's ID

### Step 3: Update Code Imports

1. Open `../pages/community.tsx`
2. Add import for the new data file:

```typescript
import june2025Data from "../data/community/june-2025.json";
```

3. Add the new data to the `communityDataMap`:

```typescript
const communityDataMap = {
  "june-2025": june2025Data,
  // ... other months
};
```

## Data Format

### Community Stars & Team Stars
- `name`: Full display name
- `handle`: GitHub username (without @)

### Monthly Leaderboard
- `handle`: GitHub username (without @)
- `rank`: Position number (1, 2, 3, etc.)
- `medal`: Only for top 3 ("🥇", "🥈", "🥉")

## Section Mapping

The JSON data maps to these page sections:
- `featuredContributors` → **Community Stars** section
- `risingStars` → **Team Stars** section  
- `leaderboard` → **Monthly Leaderboard** section

## Tips

- Avatar images are automatically generated from GitHub usernames
- GitHub links are automatically created using the handle
- The medal field is optional - only include for top 3 positions
- You can have any number of leaderboard entries
- Names and handles are case-sensitive

## File Structure

```
community/
├── config.json          # Main configuration
├── template.json        # Template for new months
├── april-2025.json     # April 2025 data
├── may-2025.json       # May 2025 data
└── README.md           # This file
```

## Quick Monthly Checklist

- [ ] Copy template.json to new month file
- [ ] Fill in contributor data for Community Stars
- [ ] Fill in contributor data for Team Stars
- [ ] Update Monthly Leaderboard rankings
- [ ] Update config.json with new month
- [ ] Add import to community.tsx
- [ ] Add to communityDataMap
- [ ] Test the page locally