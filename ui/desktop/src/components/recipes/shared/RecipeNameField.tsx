import { recipeNameSchema, RECIPE_NAME_PLACEHOLDER } from './recipeNameUtils';

interface RecipeNameFieldProps {
  id: string;
  value: string;
  onChange: (value: string) => void;
  onBlur: () => void;
  errors: string[];
  label?: string;
  required?: boolean;
  disabled?: boolean;
}

export function RecipeNameField({
  id,
  value,
  onChange,
  onBlur,
  errors,
  label = 'Recipe Name',
  required = true,
  disabled = false,
}: RecipeNameFieldProps) {
  return (
    <div>
      <label htmlFor={id} className="block text-sm font-medium text-text-standard mb-2">
        {label} {required && <span className="text-red-500">*</span>}
      </label>
      <input
        id={id}
        type="text"
        value={value}
        onChange={(e) => {
          // Allow typing normally, only filter out invalid characters but keep spaces
          const rawValue = e.target.value;
          const filtered = rawValue.replace(/[^a-zA-Z0-9\s-]/g, '');
          onChange(filtered);
        }}
        onBlur={(e) => {
          // Transform on blur: convert to lowercase and replace spaces with dashes
          const rawValue = e.target.value;
          const transformed = rawValue
            .toLowerCase()
            .replace(/\s+/g, '-')
            .replace(/[^a-z0-9-]/g, '')
            .replace(/-+/g, '-')
            .replace(/^-+|-+$/g, '');

          onChange(transformed);
          onBlur();
        }}
        disabled={disabled}
        className={`w-full p-3 border rounded-lg bg-background-default text-text-standard focus:outline-none focus:ring-2 focus:ring-blue-500 ${
          errors.length > 0 ? 'border-red-500' : 'border-border-subtle'
        } ${disabled ? 'opacity-50 cursor-not-allowed' : ''}`}
        placeholder={RECIPE_NAME_PLACEHOLDER}
        data-testid="recipe-name-input"
      />
      <p className="text-xs text-text-muted mt-1">
        Will be automatically formatted (lowercase, dashes for spaces)
      </p>
      {errors.length > 0 && <p className="text-red-500 text-sm mt-1">{errors[0]}</p>}
    </div>
  );
}

export { recipeNameSchema };
