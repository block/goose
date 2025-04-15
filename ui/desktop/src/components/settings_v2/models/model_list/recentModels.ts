import { useEffect, useState } from 'react';
import Model from '../modelInterface';

const MAX_RECENT_MODELS = 3;

export function useRecentModels() {
  const [recentModels, setRecentModels] = useState<Model[]>([]);

  useEffect(() => {
    const storedModels = localStorage.getItem('recentModels');
    if (storedModels) {
      setRecentModels(JSON.parse(storedModels));
    }
  }, []);

  const addRecentModel = (model: Model) => {
    const modelWithTimestamp = { ...model, lastUsed: new Date().toISOString() }; // Add lastUsed field
    setRecentModels((prevModels) => {
      const updatedModels = [
        modelWithTimestamp,
        ...prevModels.filter((m) => m.name !== model.name),
      ].slice(0, MAX_RECENT_MODELS);

      localStorage.setItem('recentModels', JSON.stringify(updatedModels));
      return updatedModels;
    });
  };

  return { recentModels, addRecentModel };
}
