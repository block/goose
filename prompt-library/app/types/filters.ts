export interface FilterCategory {
  id: string;
  name: string;
}

export interface FilterCategories {
  functions: FilterCategory[];
  skillLevels: FilterCategory[];
  useCases: FilterCategory[];
}