import { Lock } from 'lucide-react';

/**
 * SecureStorageNotice - A reusable component that displays a message about secure storage
 *
 * @param {Object} props - Component props
 * @param {string} [props.className] - Optional additional CSS classes
 * @param {string} [props.message] - Optional custom message (defaults to keys stored in .env)
 * @returns {JSX.Element} - The secure storage notice component
 */
export function SecureStorageNotice({
  className = '',
  message = 'Keys are stored securely in the keychain',
}) {
  return (
    <div className={`flex items-center mt-2 text-gray-600 dark:text-gray-300 ${className}`}>
      <Lock className="w-5 h-5" />
      <span className="text-sm font-light ml-2">{message}</span>
    </div>
  );
}
