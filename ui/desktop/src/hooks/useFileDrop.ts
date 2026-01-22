import { useCallback, useState, useRef, useEffect } from 'react';

export interface DroppedFile {
  id: string;
  path: string;
  name: string;
  type: string;
  isImage: boolean;
  dataUrl?: string;
  isLoading?: boolean;
  error?: string;
}

// Helper function to compress image data URLs
const compressImageDataUrl = async (dataUrl: string): Promise<string> => {
  return new Promise((resolve, reject) => {
    const img = new globalThis.Image();
    img.onload = () => {
      const canvas = document.createElement('canvas');
      const ctx = canvas.getContext('2d');
      if (!ctx) {
        reject(new Error('Failed to get canvas context'));
        return;
      }

      // Resize to max 1024px on longest side
      const maxSize = 1024;
      let width = img.width;
      let height = img.height;

      if (width > height && width > maxSize) {
        height = (height * maxSize) / width;
        width = maxSize;
      } else if (height > maxSize) {
        width = (width * maxSize) / height;
        height = maxSize;
      }

      canvas.width = width;
      canvas.height = height;
      ctx.drawImage(img, 0, 0, width, height);

      // Convert to JPEG with 0.85 quality
      const compressedDataUrl = canvas.toDataURL('image/jpeg', 0.85);
      resolve(compressedDataUrl);
    };
    img.onerror = () => reject(new Error('Failed to load image'));
    img.src = dataUrl;
  });
};

export const useFileDrop = () => {
  const [droppedFiles, setDroppedFiles] = useState<DroppedFile[]>([]);
  const activeReadersRef = useRef<Set<FileReader>>(new Set());

  // Cleanup effect to prevent memory leaks
  useEffect(() => {
    return () => {
      // Abort any active FileReaders on unmount
      // eslint-disable-next-line react-hooks/exhaustive-deps
      const readers = activeReadersRef.current;
      readers.forEach((reader) => {
        try {
          reader.abort();
        } catch {
          // Reader might already be done, ignore errors
        }
      });
      readers.clear();
    };
  }, []);

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

        if (droppedFile.isImage && !droppedFile.error) {
          const reader = new FileReader();
          activeReadersRef.current.add(reader);

          reader.onload = async (event) => {
            const dataUrl = event.target?.result as string;
            try {
              // Compress the image
              const compressedDataUrl = await compressImageDataUrl(dataUrl);
              setDroppedFiles((prev) =>
                prev.map((f) =>
                  f.id === droppedFile.id
                    ? { ...f, dataUrl: compressedDataUrl, isLoading: false }
                    : f
                )
              );
            } catch (compressionError) {
              console.error('Failed to compress image:', file.name, compressionError);
              setDroppedFiles((prev) =>
                prev.map((f) =>
                  f.id === droppedFile.id
                    ? { ...f, error: 'Failed to compress image', isLoading: false }
                    : f
                )
              );
            }
            activeReadersRef.current.delete(reader);
          };

          reader.onerror = () => {
            console.error('Failed to read image:', file.name);
            setDroppedFiles((prev) =>
              prev.map((f) =>
                f.id === droppedFile.id
                  ? { ...f, error: 'Failed to load image', isLoading: false }
                  : f
              )
            );
            activeReadersRef.current.delete(reader);
          };

          reader.onabort = () => {
            activeReadersRef.current.delete(reader);
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
