import type { FilterCategories } from '../types/filters';

export async function fetchFilterCategories(): Promise<FilterCategories> {
  // In a real app, this would be an API call
  // For now, we'll import the metadata directly
  const metadata = {
    "filterCategories": {
      "functions": [
        {
          "id": "coding",
          "name": "Coding"
        },
        {
          "id": "design",
          "name": "Design"
        },
        {
          "id": "data",
          "name": "Data"
        },
        {
          "id": "sales",
          "name": "Sales"
        },
        {
          "id": "marketing",
          "name": "Marketing"
        },
        {
          "id": "legal",
          "name": "Legal"
        },
        {
          "id": "operations",
          "name": "Operations"
        },
        {
          "id": "content",
          "name": "Content"
        },
        {
          "id": "miscellaneous",
          "name": "Miscellaneous"
        }
      ],
      "skillLevels": [
        {
          "id": "beginner",
          "name": "Beginner"
        },
        {
          "id": "intermediate",
          "name": "Intermediate"
        },
        {
          "id": "advanced",
          "name": "Advanced"
        },
        {
          "id": "expert",
          "name": "Expert"
        }
      ],
      "useCases": [
        {
          "id": "code-review",
          "name": "Code Review"
        },
        {
          "id": "debugging",
          "name": "Debugging"
        },
        {
          "id": "documentation",
          "name": "Documentation"
        },
        {
          "id": "learning",
          "name": "Learning"
        },
        {
          "id": "optimization",
          "name": "Optimization"
        },
        {
          "id": "automation",
          "name": "Automation"
        },
        {
          "id": "data-analysis",
          "name": "Data Analysis"
        },
        {
          "id": "visualization",
          "name": "Data Visualization"
        }
      ]
    }
  };
  
  return metadata.filterCategories;
}