import { Button } from "./ui/button";

interface FilterPillsProps {
  categories: string[];
  selectedCategory: string | null;
  onSelectCategory: (category: string | null) => void;
}

export function FilterPills({ categories, selectedCategory, onSelectCategory }: FilterPillsProps) {
  return (
    <div className="flex gap-3 flex-wrap">
      <Button
        variant={selectedCategory === null ? "default" : "outline"}
        size="sm"
        onClick={() => onSelectCategory(null)}
        className={`
          rounded-full px-6 py-2 font-medium transition-all
          ${selectedCategory === null 
            ? "bg-primary text-primary-foreground hover:bg-primary/90 shadow-sm"
            : "bg-background hover:bg-muted border-border text-foreground hover:text-foreground dark:border-gray-600 dark:bg-gray-800 dark:text-gray-200 dark:hover:bg-gray-700"
          }
        `}
      >
        All
      </Button>
      {categories.map((category) => (
        <Button
          key={category}
          variant={selectedCategory === category ? "default" : "outline"}
          size="sm"
          onClick={() => onSelectCategory(category)}
          className={`
            rounded-full px-6 py-2 font-medium transition-all
            ${selectedCategory === category 
              ? "bg-primary text-primary-foreground hover:bg-primary/90 shadow-sm"
              : "bg-background hover:bg-muted border-border text-foreground hover:text-foreground dark:border-gray-600 dark:bg-gray-800 dark:text-gray-200 dark:hover:bg-gray-700"
            }
          `}
        >
          {category}
        </Button>
      ))}
    </div>
  );
}