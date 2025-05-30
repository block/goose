/**
 * CustomRadio - A reusable radio button component with dark mode support
 * @param {Object} props - Component props
 * @param {string} props.id - Unique identifier for the radio input
 * @param {string} props.name - Name attribute for the radio input
 * @param {string} props.value - Value of the radio input
 * @param {boolean} props.checked - Whether the radio is checked
 * @param {function} props.onChange - Function to call when radio selection changes
 * @param {boolean} [props.disabled] - Whether the radio is disabled
 * @param {React.ReactNode} [props.label] - Primary label content
 * @param {React.ReactNode} [props.secondaryLabel] - Secondary/subtitle label content
 * @param {React.ReactNode} [props.rightContent] - Optional content to display on the right side
 * @param {string} [props.className] - Additional CSS classes for the main container
 * @returns {JSX.Element}
 */
const CustomRadio = ({
  id,
  name,
  value,
  checked,
  onChange,
  disabled = false,
  label = null,
  secondaryLabel = null,
  rightContent = null,
  className = '',
}: {
  id: string;
  name: string;
  value: string;
  checked: boolean;
  onChange: (event: React.ChangeEvent<HTMLInputElement>) => void;
  disabled?: boolean;
  label?: React.ReactNode;
  secondaryLabel?: React.ReactNode;
  rightContent?: React.ReactNode;
  className?: string;
}) => {
  return (
    <label
      htmlFor={id}
      className={`flex justify-between items-center py-2 cursor-pointer ${disabled ? 'opacity-50 cursor-not-allowed' : ''} ${className}`}
    >
      <div className="relative flex items-center">
        <input
          type="radio"
          id={id}
          name={name}
          value={value}
          checked={checked}
          onChange={onChange}
          disabled={disabled}
          className="peer sr-only"
        />
        <div
          className="h-4 w-4 rounded-full border border-gray-400 dark:border-gray-500 mr-4
                    peer-checked:border-[6px] peer-checked:border-black dark:peer-checked:border-white
                    peer-checked:bg-white dark:peer-checked:bg-black
                    transition-all duration-200 ease-in-out"
        ></div>

        {(label || secondaryLabel) && (
          <div>
            {label && <p className="text-sm text-gray-900 dark:text-gray-100">{label}</p>}
            {secondaryLabel && (
              <p className="text-xs text-gray-500 dark:text-gray-400">{secondaryLabel}</p>
            )}
          </div>
        )}
      </div>

      {rightContent && (
        <div className="flex items-center text-sm text-gray-500 dark:text-gray-400">
          {rightContent}
        </div>
      )}
    </label>
  );
};

export default CustomRadio;
