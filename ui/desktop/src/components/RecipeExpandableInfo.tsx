import { useEffect, useRef, useState } from 'react';
import { ChevronDown } from 'lucide-react';

interface RecipeExpandableInfoProps {
  infoLabel: string;
  infoValue: string;
  required?: boolean;
  onClickEdit: () => void;
}

export default function RecipeExpandableInfo({
  infoValue,
  infoLabel,
  required = false,
  onClickEdit,
}: RecipeExpandableInfoProps) {
  const [isValueExpanded, setValueExpanded] = useState(false);
  const [isClamped, setIsClamped] = useState(false);
  // eslint-disable-next-line no-undef
  const contentRef = useRef<HTMLParagraphElement>(null);

  useEffect(() => {
    const el = contentRef.current;
    if (el) {
      if (!isValueExpanded) {
        setIsClamped(el.scrollHeight > el.clientHeight);
      } else {
        setIsClamped(true);
      }
    }
  }, [infoValue, isValueExpanded]);

  return (
    <>
      <div className="flex justify-between items-center mb-2">
        <label className="block text-md text-textProminent font-bold">
          {infoLabel} {required && <span className="text-red-500">*</span>}
        </label>
        <button
          type="button"
          onClick={(e) => {
            e.preventDefault();
            setValueExpanded(true);
            onClickEdit();
          }}
          className="w-36 px-3 py-1.5 bg-bgAppInverse text-sm text-textProminentInverse rounded-xl hover:bg-bgStandardInverse transition-colors"
        >
          {infoValue ? 'Edit' : 'Add'} {infoLabel.toLowerCase()}
        </button>
      </div>

      <div className="relative border rounded-lg bg-white p-3 text-textStandard">
        {!infoValue ? (
          <p className="text-gray-500">No {infoLabel.toLowerCase()} provided.</p>
        ) : (
          <>
            <p
              ref={contentRef}
              className={`whitespace-pre-wrap transition-all duration-300 ${
                !isValueExpanded ? 'line-clamp-3' : ''
              }`}
            >
              {infoValue}
            </p>

            {/* Toggle button */}
            {isClamped && (
              <div className="mt-2 flex justify-end">
                <button
                  type="button"
                  onClick={() => setValueExpanded(!isValueExpanded)}
                  aria-label={isValueExpanded ? 'Collapse content' : 'Expand content'}
                  title={isValueExpanded ? 'Collapse' : 'Expand'}
                  className="text-black hover:text-blue-800 transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-400 rounded"
                >
                  <ChevronDown
                    className={`w-6 h-6 transition-transform duration-300 ${
                      isValueExpanded ? 'rotate-180' : ''
                    }`}
                    strokeWidth={2.5}
                  />
                </button>
              </div>
            )}
          </>
        )}
      </div>
    </>
  );
}
