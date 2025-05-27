import React, { useState, useEffect } from 'react';
import { Recipe, RecipeParameter } from '../recipe';
import { Modal, ModalContent, ModalHeader, ModalTitle, ModalFooter } from './ui/modal';
import { Button } from './ui/button';
import { Label } from './ui/label';
import { Input } from './ui/input';

interface RecipeParametersModalProps {
  isOpen: boolean;
  recipeConfig: Recipe;
  onSubmit: (paramValues: Record<string, string>) => void;
  onCancel: () => void;
}

export function RecipeParametersModal({
  isOpen,
  recipeConfig,
  onSubmit,
  onCancel,
}: RecipeParametersModalProps) {
  const [paramValues, setParamValues] = useState<Record<string, string>>({});
  const [showCancelConfirm, setShowCancelConfirm] = useState(false);

  // Initialize default values
  useEffect(() => {
    if (recipeConfig?.parameters) {
      const initialValues: Record<string, string> = {};
      recipeConfig.parameters.forEach((param) => {
        if (param.input_type === 'boolean') {
          // For boolean parameters, always set a default value
          initialValues[param.key] = param.default || 'false';
        } else if (param.default) {
          initialValues[param.key] = param.default;
        }
      });
      setParamValues(initialValues);
    }
  }, [recipeConfig]);

  const getBooleanValue = (param: RecipeParameter) => {
    const currentValue = paramValues[param.key] ?? param.default ?? 'false';
    return currentValue === 'true' ? 'true' : 'false';
  };

  const handleInputChange = (key: string, value: string) => {
    setParamValues((prev) => ({ ...prev, [key]: value }));
  };

  const handleSubmit = () => {
    onSubmit(paramValues);
  };

  const handleCancelClick = () => {
    setShowCancelConfirm(true);
  };

  const handleConfirmCancel = () => {
    setShowCancelConfirm(false);
    onCancel();
  };

  const handleCancelConfirmClose = () => {
    setShowCancelConfirm(false);
  };

  if (!recipeConfig || !recipeConfig.parameters || recipeConfig.parameters.length === 0) {
    return null;
  }

  return (
    <>
      <Modal open={isOpen} onOpenChange={(open: boolean) => !open && handleCancelClick()}>
        <ModalContent className="sm:max-w-[500px]">
          <ModalHeader>
            <ModalTitle>{recipeConfig.title || 'Recipe Parameters'}</ModalTitle>
          </ModalHeader>

          <div className="text-sm text-muted-foreground mb-4">{recipeConfig.description}</div>

          <div className="space-y-4 py-2 max-h-[400px] overflow-y-auto">
            {recipeConfig.parameters.map((param: RecipeParameter) => (
              <div key={param.key} className="space-y-2">
                <Label htmlFor={param.key}>
                  {param.description || param.key}
                  {param.requirement === 'required' && (
                    <span className="text-destructive ml-1">*</span>
                  )}
                </Label>

                {param.input_type === 'boolean' ? (
                  <input
                    id={param.key}
                    type="checkbox"
                    checked={getBooleanValue(param) === 'true'}
                    onChange={(e) =>
                      handleInputChange(param.key, e.target.checked ? 'true' : 'false')
                    }
                    className="h-4 w-4 rounded border-gray-300 text-blue-600 focus:ring-blue-500"
                  />
                ) : param.input_type === 'number' ? (
                  <Input
                    id={param.key}
                    type="number"
                    value={paramValues[param.key] || ''}
                    onChange={(e) => handleInputChange(param.key, e.target.value)}
                    required={param.requirement === 'required'}
                  />
                ) : param.input_type === 'date' ? (
                  <Input
                    id={param.key}
                    type="date"
                    value={paramValues[param.key] || ''}
                    onChange={(e) => handleInputChange(param.key, e.target.value)}
                    required={param.requirement === 'required'}
                  />
                ) : (
                  <Input
                    id={param.key}
                    type="text"
                    value={paramValues[param.key] || ''}
                    onChange={(e) => handleInputChange(param.key, e.target.value)}
                    required={param.requirement === 'required'}
                    placeholder={param.input_type === 'file' ? 'Enter file path' : ''}
                  />
                )}
              </div>
            ))}
          </div>

          <ModalFooter>
            <Button variant="outline" onClick={handleCancelClick}>
              Cancel
            </Button>
            <Button onClick={handleSubmit}>Start Session</Button>
          </ModalFooter>
        </ModalContent>
      </Modal>

      {/* Cancel Confirmation Modal */}
      <Modal open={showCancelConfirm} onOpenChange={handleCancelConfirmClose}>
        <ModalContent className="sm:max-w-[400px]">
          <ModalHeader>
            <ModalTitle>Skip Parameters?</ModalTitle>
          </ModalHeader>

          <div className="text-sm text-muted-foreground mb-4">
            Do you want to start the chat without filling in the parameters? The recipe will use the
            raw prompt with placeholder values like {`{{parameter_name}}`}.
          </div>

          <ModalFooter>
            <Button variant="outline" onClick={handleCancelConfirmClose}>
              Go Back
            </Button>
            <Button onClick={handleConfirmCancel}>Start Without Parameters</Button>
          </ModalFooter>
        </ModalContent>
      </Modal>
    </>
  );
}
