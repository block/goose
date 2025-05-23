import React, { useState, useEffect } from 'react';
import { Recipe, RecipeParameter } from '../recipe';
import { Modal, ModalContent, ModalHeader, ModalTitle, ModalFooter } from './ui/modal';
import { Button } from './ui/button';
import { Label } from './ui/label';
import { Input } from './ui/input';
import ReactSelect from 'react-select';
import { Check } from './icons';

interface RecipeParametersModalProps {
  isOpen: boolean;
  recipeConfig: Recipe;
  onClose: () => void;
  onSubmit: (paramValues: Record<string, string>) => void;
}

export function RecipeParametersModal({ 
  isOpen, 
  recipeConfig, 
  onClose, 
  onSubmit 
}: RecipeParametersModalProps) {
  const [paramValues, setParamValues] = useState<Record<string, string>>({});

  // Initialize default values
  useEffect(() => {
    if (recipeConfig?.parameters) {
      const initialValues: Record<string, string> = {};
      recipeConfig.parameters.forEach(param => {
        if (param.default) {
          initialValues[param.key] = param.default;
        }
      });
      setParamValues(initialValues);
    }
  }, [recipeConfig]);

  const handleInputChange = (key: string, value: string) => {
    setParamValues(prev => ({ ...prev, [key]: value }));
  };

  const handleSubmit = () => {
    onSubmit(paramValues);
  };

  if (!recipeConfig || !recipeConfig.parameters || recipeConfig.parameters.length === 0) {
    return null;
  }

  return (
    <Modal open={isOpen} onOpenChange={(open: boolean) => !open && onClose()}>
      <ModalContent className="sm:max-w-[500px]">
        <ModalHeader>
          <ModalTitle>{recipeConfig.title || 'Recipe Parameters'}</ModalTitle>
        </ModalHeader>
        
        <div className="text-sm text-muted-foreground mb-4">
          {recipeConfig.description}
        </div>
        
        <div className="space-y-4 py-2 max-h-[400px] overflow-y-auto">
          {recipeConfig.parameters.map((param: RecipeParameter) => (
            <div key={param.key} className="space-y-2">
              <Label htmlFor={param.key}>
                {param.description || param.key}
                {param.requirement === 'required' && <span className="text-destructive ml-1">*</span>}
              </Label>
              
              {param.input_type === 'boolean' ? (
                <ReactSelect
                  id={param.key}
                  value={
                    paramValues[param.key] === 'true' 
                      ? { value: 'true', label: 'Yes' } 
                      : { value: 'false', label: 'No' }
                  }
                  onChange={(option: any) => handleInputChange(param.key, option.value)}
                  options={[
                    { value: 'true', label: 'Yes' },
                    { value: 'false', label: 'No' }
                  ]}
                  className="react-select-container"
                  classNamePrefix="react-select"
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
          <Button variant="outline" onClick={onClose}>
            Cancel
          </Button>
          <Button onClick={handleSubmit}>
            Start Session
          </Button>
        </ModalFooter>
      </ModalContent>
    </Modal>
  );
} 