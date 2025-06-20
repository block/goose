import { useCurrentModelInfo } from '../components/ChatView';

export function useCurrentModel() {
  const modelInfo = useCurrentModelInfo();

  return {
    currentModel: modelInfo?.model || null,
    isLoading: false,
  };
}
