import { useCallback, useState } from 'react';

export interface DroppedFile {
  id: string;
  path: string;
  name: string;
  type: string;
  isImage: boolean;
  dataUrl?: string; // For image previews
  isLoading?: boolean;
  error?: string;
}

export const useFileDrop = () => {
  const [droppedFiles, setDroppedFiles] = useState<DroppedFile[]>([]);

  const handleDrop = useCallback(async (e: React.DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    const files = e.dataTransfer.files;
    if (files.length > 0) {
      const droppedFileObjects: DroppedFile[] = [];

      for (let i = 0; i < files.length; i++) {
        const file = files[i];

        let droppedFile: DroppedFile;

        try {
          const path = window.electron.getPathForFile(file);
          const isImage = file.type.startsWith('image/');

          droppedFile = {
            id: `dropped-${Date.now()}-${i}`,
            path,
            name: file.name,
            type: file.type,
            isImage,
            isLoading: isImage, // Only images need loading state for preview generation
          };
        } catch (error) {
          console.error('Error processing file:', file.name, error);
          // Create an error file object
          droppedFile = {
            id: `dropped-error-${Date.now()}-${i}`,
            path: '',
            name: file.name,
            type: file.type,
            isImage: false,
            isLoading: false,
            error: `Failed to get file path: ${error instanceof Error ? error.message : 'Unknown error'}`,
          };
        }

        droppedFileObjects.push(droppedFile);

        // For images, generate a preview (only if successfully processed)
        if (droppedFile.isImage && !droppedFile.error) {
          const reader = new FileReader();
          reader.onload = (event) => {
            const dataUrl = event.target?.result as string;
            setDroppedFiles((prev) =>
              prev.map((f) => (f.id === droppedFile.id ? { ...f, dataUrl, isLoading: false } : f))
            );
          };
          reader.onerror = () => {
            console.error('Failed to generate preview for:', file.name);
            setDroppedFiles((prev) =>
              prev.map((f) =>
                f.id === droppedFile.id
                  ? { ...f, error: 'Failed to load image preview', isLoading: false }
                  : f
              )
            );
          };
          reader.readAsDataURL(file);
        }
      }

      setDroppedFiles((prev) => [...prev, ...droppedFileObjects]);
    }
  }, []);

  const handleDragOver = useCallback((e: React.DragEvent<HTMLDivElement>) => {
    e.preventDefault();
  }, []);

  return {
    droppedFiles,
    setDroppedFiles,
    handleDrop,
    handleDragOver,
  };
};
