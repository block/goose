import { useState, useEffect } from 'react';
import { X, FolderOpen } from 'lucide-react';
import { Button } from '../../ui/button';
import { SubRecipeFormData } from './recipeFormSchema';
import { useEscapeKey } from '../../../hooks/useEscapeKey';
import KeyValueEditor from './KeyValueEditor';
import { toastError } from '../../../toasts';

interface SubRecipeModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSave: (subRecipe: SubRecipeFormData) => void;
  subRecipe?: SubRecipeFormData | null;
}

export default function SubRecipeModal({
  isOpen,
  onClose,
  onSave,
  subRecipe,
}: SubRecipeModalProps) {
  const [name, setName] = useState('');
  const [path, setPath] = useState('');
  const [description, setDescription] = useState('');
  const [sequentialWhenRepeated, setSequentialWhenRepeated] = useState(false);
  const [values, setValues] = useState<Record<string, string>>({});

  useEscapeKey(isOpen, onClose);

  useEffect(() => {
    if (isOpen) {
      if (subRecipe) {
        setName(subRecipe.name);
        setPath(subRecipe.path);
        setDescription(subRecipe.description || '');
        setSequentialWhenRepeated(subRecipe.sequential_when_repeated ?? false);
        setValues(subRecipe.values || {});
      } else {
        setName('');
        setPath('');
        setDescription('');
        setSequentialWhenRepeated(false);
        setValues({});
      }
    }
  }, [isOpen, subRecipe]);

  const handleSave = () => {
    if (!name.trim() || !path.trim()) {
      return;
    }

    const subRecipeData: SubRecipeFormData = {
      name: name.trim(),
      path: path.trim(),
      description: description.trim() || undefined,
      sequential_when_repeated: sequentialWhenRepeated,
      values: Object.keys(values).length > 0 ? values : undefined,
    };

    onSave(subRecipeData);
    onClose();
  };

  const handleBrowseFile = async () => {
    try {
      const selectedPath = await window.electron.selectFileOrDirectory();
      if (selectedPath) {
        if (!selectedPath.endsWith('.yaml') && !selectedPath.endsWith('.yml')) {
          toastError({
            title: 'Invalid File',
            msg: 'Please select a YAML file (.yaml or .yml).',
          });
          return;
        }
        setPath(selectedPath);
      }
    } catch (error) {
      console.error('Failed to browse for file:', error);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-[500] flex items-center justify-center bg-black/50">
      <div className="bg-background-primary border border-borderSubtle rounded-lg w-[90vw] max-w-2xl max-h-[90vh] flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between p-6 border-b border-borderSubtle">
          <div>
            <h2 className="text-xl font-medium text-textProminent">
              {subRecipe ? 'Configure Subrecipe' : 'Add Subrecipe'}
            </h2>
            <p className="text-textSubtle text-sm">
              Configure a subrecipe that can be called as a tool during recipe execution
            </p>
          </div>
          <Button
            onClick={onClose}
            variant="ghost"
            size="sm"
            className="p-2 hover:bg-bgSubtle rounded-lg transition-colors"
            aria-label="Close subrecipe modal"
          >
            <X className="w-5 h-5" />
          </Button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto px-6 py-4 space-y-4">
          {/* Name Field */}
          <div>
            <label
              htmlFor="subrecipe-name"
              className="block text-sm font-medium text-text-standard mb-2"
            >
              Name <span className="text-text-danger">*</span>
            </label>
            <input
              id="subrecipe-name"
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              className="w-full p-3 border border-border-subtle rounded-lg bg-background-primary text-text-standard focus:outline-none focus:ring-2 focus:ring-ring"
              placeholder="e.g., security_scan"
            />
            <p className="text-xs text-text-muted mt-1">
              Unique identifier used to generate the tool name
            </p>
          </div>

          {/* Path Field */}
          <div>
            <label
              htmlFor="subrecipe-path"
              className="block text-sm font-medium text-text-standard mb-2"
            >
              Path <span className="text-text-danger">*</span>
            </label>
            <div className="flex gap-2">
              <input
                id="subrecipe-path"
                type="text"
                value={path}
                onChange={(e) => setPath(e.target.value)}
                className="flex-1 p-3 border border-border-subtle rounded-lg bg-background-primary text-text-standard focus:outline-none focus:ring-2 focus:ring-ring"
                placeholder="e.g., ./subrecipes/security-analysis.yaml"
              />
              <Button
                type="button"
                onClick={handleBrowseFile}
                variant="outline"
                className="px-4 py-2 flex items-center gap-2"
              >
                <FolderOpen className="w-4 h-4" />
                Browse
              </Button>
            </div>
            <p className="text-xs text-text-muted mt-1">
              Browse for an existing recipe file or enter a path manually
            </p>
          </div>

          {/* Description Field */}
          <div>
            <label
              htmlFor="subrecipe-description"
              className="block text-sm font-medium text-text-standard mb-2"
            >
              Description
            </label>
            <textarea
              id="subrecipe-description"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              className="w-full p-3 border border-border-subtle rounded-lg bg-background-primary text-text-standard focus:outline-none focus:ring-2 focus:ring-ring resize-none"
              placeholder="Optional description of what this subrecipe does..."
              rows={3}
            />
          </div>

          {/* Sequential When Repeated */}
          <div className="flex items-center gap-2">
            <input
              id="subrecipe-sequential"
              type="checkbox"
              checked={sequentialWhenRepeated}
              onChange={(e) => setSequentialWhenRepeated(e.target.checked)}
              className="w-4 h-4 border-border-subtle rounded focus:ring-2 focus:ring-ring"
            />
            <label htmlFor="subrecipe-sequential" className="text-sm text-text-standard">
              Sequential when repeated
            </label>
            <span className="text-xs text-text-muted">
              (Forces sequential execution of multiple subrecipe instances)
            </span>
          </div>

          {/* Values Section */}
          <div>
            <label className="block text-sm font-medium text-text-standard mb-2">
              Pre-configured Values
            </label>
            <p className="text-xs text-text-muted mb-3">
              Optional parameter values that are always passed to the subrecipe
            </p>
            <KeyValueEditor values={values} onChange={setValues} />
          </div>
        </div>

        {/* Footer */}
        <div className="flex gap-2 p-6 border-t border-borderSubtle">
          <Button onClick={onClose} variant="outline" className="flex-1">
            Cancel
          </Button>
          <Button onClick={handleSave} disabled={!name.trim() || !path.trim()} className="flex-1">
            {subRecipe ? 'Apply' : 'Add Subrecipe'}
          </Button>
        </div>
      </div>
    </div>
  );
}
