interface RecipeTitleFieldProps {
  id: string;
  value: string;
  onChange: (value: string) => void;
  onBlur: () => void;
  errors: string[];
  label?: string;
  required?: boolean;
  disabled?: boolean;
}

export function RecipeTitleField({
  id,
  value,
  onChange,
  onBlur,
  errors,
  label = 'Recipe Title',
  required = true,
  disabled = false,
}: RecipeTitleFieldProps) {
  return (
    <div>
      <label htmlFor={id} className="block text-sm font-medium text-text-standard mb-2">
        {label} {required && <span className="text-red-500">*</span>}
      </label>
      <input
        id={id}
        type="text"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        onBlur={onBlur}
        disabled={disabled}
        className={`w-full p-3 border rounded-lg bg-background-default text-text-standard focus:outline-none focus:ring-2 focus:ring-blue-500 ${
          errors.length > 0 ? 'border-red-500' : 'border-border-subtle'
        } ${disabled ? 'opacity-50 cursor-not-allowed' : ''}`}
        placeholder="My Recipe Title"
      />
      <p className="text-xs text-text-muted mt-1">
        This will be the display name shown in your recipe library
      </p>
      {errors.length > 0 && <p className="text-red-500 text-sm mt-1">{errors[0]}</p>}
    </div>
  );
}
