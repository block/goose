import React, { useRef, useState, useEffect, useMemo } from 'react';
import { Button } from './ui/button';
import type { View } from '../App';
import Stop from './ui/Stop';
import { Attach, Send, Close, Document } from './icons';
import { debounce } from 'lodash';
import BottomMenu from './bottom_menu/BottomMenu';
import { LocalMessageStorage } from '../utils/localMessageStorage';
import { Message, SessionFile } from '../types/message';
import { FolderOpen } from 'lucide-react';

// Constants for image handling
const MAX_IMAGES_PER_MESSAGE = 5;
const MAX_IMAGE_SIZE_MB = 5;

interface ChatInputProps {
  handleSubmit: (e: React.FormEvent) => void;
  isLoading?: boolean;
  onStop?: () => void;
  commandHistory?: string[]; // Current chat's message history
  initialValue?: string;
  setView: (view: View) => void;
  numTokens?: number;
  hasMessages?: boolean;
  messages?: Message[];
  setMessages: (messages: Message[]) => void;
  sessionFiles?: SessionFile[];
  setSessionFiles?: (files: SessionFile[]) => void;
}

export default function ChatInput({
  handleSubmit,
  isLoading = false,
  onStop,
  commandHistory = [],
  initialValue = '',
  setView,
  numTokens,
  messages = [],
  setMessages,
  sessionFiles = [],
  setSessionFiles,
}: ChatInputProps) {
  const [_value, setValue] = useState(initialValue);
  const [displayValue, setDisplayValue] = useState(initialValue); // For immediate visual feedback
  const [isFocused, setIsFocused] = useState(false);
  const [internalSessionFiles, setInternalSessionFiles] = useState<SessionFile[]>([]);

  // Use external sessionFiles if provided, otherwise use internal state
  const currentSessionFiles = sessionFiles || internalSessionFiles;
  const currentSetSessionFiles = setSessionFiles || setInternalSessionFiles;

  // Type assertion to fix the setState function type
  const setSessionFilesFn = currentSetSessionFiles as React.Dispatch<
    React.SetStateAction<SessionFile[]>
  >;

  // Update internal value when initialValue changes
  useEffect(() => {
    setValue(initialValue);
    setDisplayValue(initialValue);

    // Use a functional update to get the current sessionFiles
    // and perform cleanup. This avoids needing sessionFiles in the deps.
    setSessionFilesFn((currentSessionFiles: SessionFile[]) => {
      currentSessionFiles.forEach((file: SessionFile) => {
        if (file.filePath) {
          window.electron.deleteTempFile(file.filePath);
        }
      });
      return []; // Return a new empty array
    });

    // Reset history index when input is cleared
    setHistoryIndex(-1);
    setIsInGlobalHistory(false);
  }, [initialValue, setSessionFilesFn]); // Keep only initialValue as a dependency

  // State to track if the IME is composing (i.e., in the middle of Japanese IME input)
  const [isComposing, setIsComposing] = useState(false);
  const [historyIndex, setHistoryIndex] = useState(-1);
  const [savedInput, setSavedInput] = useState('');
  const [isInGlobalHistory, setIsInGlobalHistory] = useState(false);
  const textAreaRef = useRef<HTMLTextAreaElement>(null);

  const handleRemovePastedImage = (idToRemove: string) => {
    const imageToRemove = currentSessionFiles.find((img) => img.id === idToRemove);
    if (imageToRemove?.filePath) {
      window.electron.deleteTempFile(imageToRemove.filePath);
    }
    setSessionFilesFn((prev) => prev.filter((img) => img.id !== idToRemove));
  };

  const handleRetryImageSave = async (imageId: string) => {
    const imageToRetry = currentSessionFiles.find((img) => img.id === imageId);
    if (!imageToRetry || !imageToRetry.dataUrl) return;

    // Set the image to loading state
    setSessionFilesFn((prev) =>
      prev.map((img) => (img.id === imageId ? { ...img, isLoading: true, error: undefined } : img))
    );

    try {
      const result = await window.electron.saveDataUrlToTemp(imageToRetry.dataUrl, imageId);
      setSessionFilesFn((prev) =>
        prev.map((img) =>
          img.id === result.id
            ? { ...img, filePath: result.filePath, error: result.error, isLoading: false }
            : img
        )
      );
    } catch (err) {
      console.error('Error retrying image save:', err);
      setSessionFilesFn((prev) =>
        prev.map((img) =>
          img.id === imageId
            ? { ...img, error: 'Failed to save image via Electron.', isLoading: false }
            : img
        )
      );
    }
  };

  useEffect(() => {
    if (textAreaRef.current) {
      textAreaRef.current.focus();
    }
  }, []);

  const minHeight = '1rem';
  const maxHeight = 10 * 24;

  // Debounced function to update actual value
  const debouncedSetValue = useMemo(
    () =>
      debounce((value: string) => {
        setValue(value);
      }, 150),
    [setValue]
  );

  // Debounced autosize function
  const debouncedAutosize = useMemo(
    () =>
      debounce((element: HTMLTextAreaElement) => {
        element.style.height = '0px'; // Reset height
        const scrollHeight = element.scrollHeight;
        element.style.height = Math.min(scrollHeight, maxHeight) + 'px';
      }, 150),
    [maxHeight]
  );

  useEffect(() => {
    if (textAreaRef.current) {
      debouncedAutosize(textAreaRef.current);
    }
  }, [debouncedAutosize, displayValue]);

  const handleChange = (evt: React.ChangeEvent<HTMLTextAreaElement>) => {
    const val = evt.target.value;
    setDisplayValue(val); // Update display immediately
    debouncedSetValue(val); // Debounce the actual state update
  };

  const handlePaste = async (evt: React.ClipboardEvent<HTMLTextAreaElement>) => {
    const files = Array.from(evt.clipboardData.files || []);
    const imageFiles = files.filter((file) => file.type.startsWith('image/'));
    const nonImageFiles = files.filter((file) => !file.type.startsWith('image/'));

    // If there are any files (image or non-image), prevent default paste behavior
    if (files.length > 0) {
      evt.preventDefault();
    }

    // Handle non-image files first - add them to sessionFiles
    if (nonImageFiles.length > 0) {
      const processNonImageFiles = async () => {
        // Collect all new context paths first
        const newSessionFiles: SessionFile[] = [];

        for (const file of nonImageFiles) {
          try {
            // Get the file path using the electron API
            const filePath = window.electron.getPathForFile(file);
            if (filePath) {
              // Get the path type
              const pathType = await window.electron.getPathType(filePath);

              // Check if this path is already in sessionFiles
              const isAlreadyAdded = currentSessionFiles.some((item) => item.path === filePath);

              if (!isAlreadyAdded) {
                const newSessionFile: SessionFile = {
                  id: `file-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`,
                  path: filePath,
                  type: pathType === 'directory' ? 'directory' : 'file',
                };
                newSessionFiles.push(newSessionFile);
              }
            }
          } catch (error) {
            console.error('Error processing dropped file:', error);
          }
        }

        // Update sessionFiles with all new items at once
        if (newSessionFiles.length > 0) {
          setSessionFilesFn((prev) => [...prev, ...newSessionFiles]);
        }
      };
      processNonImageFiles();
    }

    // Handle image files with existing functionality
    if (imageFiles.length === 0) return;

    // Check if adding these images would exceed the limit
    const currentImageCount = currentSessionFiles.filter((file) => file.type === 'image').length;
    if (currentImageCount + imageFiles.length > MAX_IMAGES_PER_MESSAGE) {
      // Show error message to user
      setSessionFilesFn((prev) => [
        ...prev,
        {
          id: `error-${Date.now()}`,
          path: '',
          type: 'image',
          dataUrl: '',
          isLoading: false,
          error: `Cannot paste ${imageFiles.length} image(s). Maximum ${MAX_IMAGES_PER_MESSAGE} images per message allowed.`,
        },
      ]);

      // Remove the error message after 3 seconds
      setTimeout(() => {
        setSessionFilesFn((prev) => prev.filter((img) => !img.id.startsWith('error-')));
      }, 3000);

      return;
    }

    for (const file of imageFiles) {
      // Check individual file size before processing
      if (file.size > MAX_IMAGE_SIZE_MB * 1024 * 1024) {
        const errorId = `error-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`;
        setSessionFilesFn((prev) => [
          ...prev,
          {
            id: errorId,
            path: '',
            type: 'image',
            dataUrl: '',
            isLoading: false,
            error: `Image too large (${Math.round(file.size / (1024 * 1024))}MB). Maximum ${MAX_IMAGE_SIZE_MB}MB allowed.`,
          },
        ]);

        // Remove the error message after 3 seconds
        setTimeout(() => {
          setSessionFilesFn((prev) => prev.filter((img) => img.id !== errorId));
        }, 3000);

        continue;
      }

      const reader = new FileReader();
      reader.onload = async (e) => {
        const dataUrl = e.target?.result as string;
        if (dataUrl) {
          const imageId = `img-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`;
          setSessionFilesFn((prev) => [
            ...prev,
            {
              id: imageId,
              path: '',
              type: 'image',
              dataUrl,
              isLoading: true,
            },
          ]);

          try {
            const result = await window.electron.saveDataUrlToTemp(dataUrl, imageId);
            setSessionFilesFn((prev) =>
              prev.map((img) =>
                img.id === result.id
                  ? {
                      ...img,
                      filePath: result.filePath || '',
                      error: result.error,
                      isLoading: false,
                    }
                  : img
              )
            );
          } catch (err) {
            console.error('Error saving pasted image:', err);
            setSessionFilesFn((prev) =>
              prev.map((img) =>
                img.id === imageId
                  ? { ...img, error: 'Failed to save image via Electron.', isLoading: false }
                  : img
              )
            );
          }
        }
      };
      reader.readAsDataURL(file);
    }
  };

  // Cleanup debounced functions on unmount
  useEffect(() => {
    return () => {
      debouncedSetValue.cancel?.();
      debouncedAutosize.cancel?.();
    };
  }, [debouncedSetValue, debouncedAutosize]);

  // Handlers for composition events, which are crucial for proper IME behavior
  const handleCompositionStart = () => {
    setIsComposing(true);
  };

  const handleCompositionEnd = () => {
    setIsComposing(false);
  };

  const handleHistoryNavigation = (evt: React.KeyboardEvent<HTMLTextAreaElement>) => {
    const isUp = evt.key === 'ArrowUp';
    const isDown = evt.key === 'ArrowDown';

    // Only handle up/down keys with Cmd/Ctrl modifier
    if ((!isUp && !isDown) || !(evt.metaKey || evt.ctrlKey) || evt.altKey || evt.shiftKey) {
      return;
    }

    evt.preventDefault();

    // Get global history once to avoid multiple calls
    const globalHistory = LocalMessageStorage.getRecentMessages() || [];

    // Save current input if we're just starting to navigate history
    if (historyIndex === -1) {
      setSavedInput(displayValue || '');
      setIsInGlobalHistory(commandHistory.length === 0);
    }

    // Determine which history we're using
    const currentHistory = isInGlobalHistory ? globalHistory : commandHistory;
    let newIndex = historyIndex;
    let newValue = '';

    // Handle navigation
    if (isUp) {
      // Moving up through history
      if (newIndex < currentHistory.length - 1) {
        // Still have items in current history
        newIndex = historyIndex + 1;
        newValue = currentHistory[newIndex];
      } else if (!isInGlobalHistory && globalHistory.length > 0) {
        // Switch to global history
        setIsInGlobalHistory(true);
        newIndex = 0;
        newValue = globalHistory[newIndex];
      }
    } else {
      // Moving down through history
      if (newIndex > 0) {
        // Still have items in current history
        newIndex = historyIndex - 1;
        newValue = currentHistory[newIndex];
      } else if (isInGlobalHistory && commandHistory.length > 0) {
        // Switch to chat history
        setIsInGlobalHistory(false);
        newIndex = commandHistory.length - 1;
        newValue = commandHistory[newIndex];
      } else {
        // Return to original input
        newIndex = -1;
        newValue = savedInput;
      }
    }

    // Update display if we have a new value
    if (newIndex !== historyIndex) {
      setHistoryIndex(newIndex);
      if (newIndex === -1) {
        setDisplayValue(savedInput || '');
        setValue(savedInput || '');
      } else {
        setDisplayValue(newValue || '');
        setValue(newValue || '');
      }
    }
  };

  const performSubmit = () => {
    // Create a custom event with the current value and session files
    const customEvent = new CustomEvent('submit', {
      detail: {
        value: _value,
        sessionFiles: currentSessionFiles, // Pass session files directly
      },
    });

    // Call the handleSubmit function with the custom event
    handleSubmit(customEvent as unknown as React.FormEvent);

    // Reset the input
    setDisplayValue('');
    setValue('');
    setSessionFilesFn([]);
    setHistoryIndex(-1);
    setSavedInput('');
    setIsInGlobalHistory(false);
  };

  const handleKeyDown = (evt: React.KeyboardEvent<HTMLTextAreaElement>) => {
    // Don't handle key events if IME is composing
    if (isComposing) return;

    const canSubmit =
      !isLoading &&
      (displayValue.trim() ||
        currentSessionFiles.some(
          (file) => file.type === 'image' && file.filePath && !file.error && !file.isLoading
        ) ||
        currentSessionFiles.some((file) => file.type !== 'image' && file.path));
    if (canSubmit) {
      if (evt.key === 'Enter' && !evt.shiftKey) {
        evt.preventDefault();
        performSubmit();
        return;
      }
    }

    // Handle history navigation
    handleHistoryNavigation(evt);
  };

  const onFormSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const canSubmit =
      !isLoading &&
      (displayValue.trim() ||
        currentSessionFiles.some(
          (file) => file.type === 'image' && file.filePath && !file.error && !file.isLoading
        ) ||
        currentSessionFiles.some((file) => file.type !== 'image' && file.path));
    if (canSubmit) {
      performSubmit();
    }
  };

  const hasSubmittableContent =
    displayValue.trim() ||
    currentSessionFiles.some(
      (file) => file.type === 'image' && file.filePath && !file.error && !file.isLoading
    ) ||
    currentSessionFiles.some((file) => file.type !== 'image' && file.path);
  const isAnyImageLoading = currentSessionFiles.some(
    (file) => file.type === 'image' && file.isLoading
  );

  // Additional paths menu state and handlers
  const [isAdditionalPathsMenuOpen, setIsAdditionalPathsMenuOpen] = useState(false);
  const additionalPathsMenuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (
        additionalPathsMenuRef.current &&
        !additionalPathsMenuRef.current.contains(event.target as Node)
      ) {
        setIsAdditionalPathsMenuOpen(false);
      }
    };

    if (isAdditionalPathsMenuOpen) {
      document.addEventListener('mousedown', handleClickOutside);
    }

    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [isAdditionalPathsMenuOpen]);

  const handleSelectFilesAndFolders = async () => {
    try {
      const filePaths = await window.electron.selectMultipleFiles();
      if (filePaths.length > 0) {
        // Collect all new context paths first
        const newSessionFiles: SessionFile[] = [];

        // Process each file path
        for (const filePath of filePaths) {
          try {
            // Get the path type
            const pathType = await window.electron.getPathType(filePath);

            // Check if this is an image file by examining the file extension
            const isImageFile = /\.(jpg|jpeg|png|gif|bmp|webp|svg|ico|tiff|tif)$/i.test(filePath);

            if (isImageFile) {
              // Handle image files like pasted images
              // Check if adding this image would exceed the limit
              const currentImageCount = currentSessionFiles.filter(
                (file) => file.type === 'image'
              ).length;
              if (currentImageCount + 1 > MAX_IMAGES_PER_MESSAGE) {
                // Show error message to user
                setSessionFilesFn((prev) => [
                  ...prev,
                  {
                    id: `error-${Date.now()}`,
                    path: '',
                    type: 'image',
                    dataUrl: '',
                    isLoading: false,
                    error: `Cannot add image. Maximum ${MAX_IMAGES_PER_MESSAGE} images per message allowed.`,
                  },
                ]);

                // Remove the error message after 3 seconds
                setTimeout(() => {
                  setSessionFilesFn((prev) => prev.filter((img) => !img.id.startsWith('error-')));
                }, 3000);

                continue;
              }

              // Read the image file and convert to data URL using the Electron API
              try {
                const dataUrl = await window.electron.readImageFile(filePath);
                if (dataUrl) {
                  const imageId = `img-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`;
                  setSessionFilesFn((prev) => [
                    ...prev,
                    {
                      id: imageId,
                      path: '',
                      type: 'image',
                      dataUrl,
                      isLoading: true,
                    },
                  ]);

                  try {
                    const result = await window.electron.saveDataUrlToTemp(dataUrl, imageId);
                    setSessionFilesFn((prev) =>
                      prev.map((img) =>
                        img.id === result.id
                          ? {
                              ...img,
                              filePath: result.filePath,
                              error: result.error,
                              isLoading: false,
                            }
                          : img
                      )
                    );
                  } catch (err) {
                    console.error('Error saving selected image:', err);
                    setSessionFilesFn((prev) =>
                      prev.map((img) =>
                        img.id === imageId
                          ? {
                              ...img,
                              error: 'Failed to save image via Electron.',
                              isLoading: false,
                            }
                          : img
                      )
                    );
                  }
                } else {
                  // Show error message for unsupported or invalid image
                  const errorId = `error-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`;
                  setSessionFilesFn((prev) => [
                    ...prev,
                    {
                      id: errorId,
                      path: '',
                      type: 'image',
                      dataUrl: '',
                      isLoading: false,
                      error:
                        'Unable to read image file. File may be unsupported, too large, or corrupted.',
                    },
                  ]);

                  // Remove the error message after 3 seconds
                  setTimeout(() => {
                    setSessionFilesFn((prev) => prev.filter((img) => img.id !== errorId));
                  }, 3000);
                }
              } catch (err) {
                console.error('Error reading image file:', filePath, err);
                const errorId = `error-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`;
                setSessionFilesFn((prev) => [
                  ...prev,
                  {
                    id: errorId,
                    path: '',
                    type: 'image',
                    dataUrl: '',
                    isLoading: false,
                    error: 'Failed to read image file.',
                  },
                ]);

                // Remove the error message after 3 seconds
                setTimeout(() => {
                  setSessionFilesFn((prev) => prev.filter((img) => img.id !== errorId));
                }, 3000);
              }
            } else {
              // Handle non-image files - collect them for batch update
              // Check if this path is already in sessionFiles
              const isAlreadyAdded = currentSessionFiles.some((item) => item.path === filePath);

              if (!isAlreadyAdded) {
                const newSessionFile: SessionFile = {
                  id: `file-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`,
                  path: filePath,
                  type: pathType === 'directory' ? 'directory' : 'file',
                };
                newSessionFiles.push(newSessionFile);
              }
            }
          } catch (error) {
            console.error('Error processing selected file:', filePath, error);
          }
        }

        // Update sessionFiles with all new items at once
        if (newSessionFiles.length > 0) {
          setSessionFilesFn((prev) => [...prev, ...newSessionFiles]);
        }
      }
    } catch (error) {
      console.error('Error selecting files:', error);
    }
  };

  const handleDrop = (evt: React.DragEvent<HTMLDivElement>) => {
    evt.preventDefault();
    evt.stopPropagation(); // Prevent parent drop handler from firing
    const files = Array.from(evt.dataTransfer.files);
    const imageFiles = files.filter((file) => file.type.startsWith('image/'));
    const nonImageFiles = files.filter((file) => !file.type.startsWith('image/'));

    // Handle non-image files first - add them to sessionFiles
    if (nonImageFiles.length > 0) {
      const processNonImageFiles = async () => {
        // Collect all new context paths first
        const newSessionFiles: SessionFile[] = [];

        for (const file of nonImageFiles) {
          try {
            // Get the file path using the electron API
            const filePath = window.electron.getPathForFile(file);
            if (filePath) {
              // Get the path type
              const pathType = await window.electron.getPathType(filePath);

              // Check if this path is already in sessionFiles
              const isAlreadyAdded = currentSessionFiles.some((item) => item.path === filePath);

              if (!isAlreadyAdded) {
                const newSessionFile: SessionFile = {
                  id: `file-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`,
                  path: filePath,
                  type: pathType === 'directory' ? 'directory' : 'file',
                };
                newSessionFiles.push(newSessionFile);
              }
            }
          } catch (error) {
            console.error('Error processing dropped file:', error);
          }
        }

        // Update sessionFiles with all new items at once
        if (newSessionFiles.length > 0) {
          setSessionFilesFn((prev) => [...prev, ...newSessionFiles]);
        }
      };
      processNonImageFiles();
    }

    // Handle image files with the same logic as paste functionality
    if (imageFiles.length === 0) return;

    // Check if adding these images would exceed the limit
    const currentImageCount = currentSessionFiles.filter((file) => file.type === 'image').length;
    if (currentImageCount + imageFiles.length > MAX_IMAGES_PER_MESSAGE) {
      // Show error message to user
      setSessionFilesFn((prev) => [
        ...prev,
        {
          id: `error-${Date.now()}`,
          path: '',
          type: 'image',
          dataUrl: '',
          isLoading: false,
          error: `Cannot drop ${imageFiles.length} image(s). Maximum ${MAX_IMAGES_PER_MESSAGE} images per message allowed.`,
        },
      ]);

      // Remove the error message after 3 seconds
      setTimeout(() => {
        setSessionFilesFn((prev) => prev.filter((img) => !img.id.startsWith('error-')));
      }, 3000);

      return;
    }

    for (const file of imageFiles) {
      // Check individual file size before processing
      if (file.size > MAX_IMAGE_SIZE_MB * 1024 * 1024) {
        const errorId = `error-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`;
        setSessionFilesFn((prev) => [
          ...prev,
          {
            id: errorId,
            path: '',
            type: 'image',
            dataUrl: '',
            isLoading: false,
            error: `Image too large (${Math.round(file.size / (1024 * 1024))}MB). Maximum ${MAX_IMAGE_SIZE_MB}MB allowed.`,
          },
        ]);

        // Remove the error message after 3 seconds
        setTimeout(() => {
          setSessionFilesFn((prev) => prev.filter((img) => img.id !== errorId));
        }, 3000);

        continue;
      }

      const reader = new FileReader();
      reader.onload = async (e) => {
        const dataUrl = e.target?.result as string;
        if (dataUrl) {
          const imageId = `img-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`;
          setSessionFilesFn((prev) => [
            ...prev,
            {
              id: imageId,
              path: '',
              type: 'image',
              dataUrl,
              isLoading: true,
            },
          ]);

          try {
            const result = await window.electron.saveDataUrlToTemp(dataUrl, imageId);
            setSessionFilesFn((prev) =>
              prev.map((img) =>
                img.id === result.id
                  ? { ...img, filePath: result.filePath, error: result.error, isLoading: false }
                  : img
              )
            );
          } catch (err) {
            console.error('Error saving dropped image:', err);
            setSessionFilesFn((prev) =>
              prev.map((img) =>
                img.id === imageId
                  ? { ...img, error: 'Failed to save image via Electron.', isLoading: false }
                  : img
              )
            );
          }
        }
      };
      reader.readAsDataURL(file);
    }
  };

  const handleDragOver = (evt: React.DragEvent<HTMLDivElement>) => {
    evt.preventDefault();
  };

  // Filter session files to show only non-image files in the context display
  const contextFiles = currentSessionFiles.filter((file) => file.type !== 'image');

  return (
    <div
      className={`flex flex-col relative h-auto border rounded-lg transition-colors ${
        isFocused
          ? 'border-borderProminent hover:border-borderProminent'
          : 'border-borderSubtle hover:border-borderStandard'
      } bg-bgApp z-10`}
      onDrop={handleDrop}
      onDragOver={handleDragOver}
    >
      <form onSubmit={onFormSubmit}>
        {/* Context files (lozenges) above the textarea */}
        {contextFiles.length > 0 && (
          <div className="flex flex-wrap gap-2 p-2 pb-0">
            <div className="flex items-center gap-2 w-full">
              <div className="flex flex-wrap gap-2 flex-1">
                {contextFiles.slice(0, 10).map((filePath) => (
                  <div
                    key={filePath.path}
                    className="flex items-center gap-1 px-2 py-1 bg-bgSubtle border border-borderSubtle rounded-full text-xs text-textStandard"
                  >
                    {filePath.type === 'directory' ? (
                      <FolderOpen className="w-3 h-3 text-textSubtle" />
                    ) : (
                      <Document className="w-3 h-3 text-textSubtle" />
                    )}
                    <span className="max-w-[200px] truncate" title={filePath.path}>
                      {filePath.path.split('/').pop() || filePath.path}
                    </span>
                    <button
                      type="button"
                      onClick={() =>
                        setSessionFilesFn(
                          currentSessionFiles.filter((fp) => fp.path !== filePath.path)
                        )
                      }
                      className="text-textSubtle hover:text-textStandard transition-colors"
                      title="Remove from context"
                    >
                      <Close className="w-3 h-3" />
                    </button>
                  </div>
                ))}
                {contextFiles.length > 10 && (
                  <div className="relative" ref={additionalPathsMenuRef}>
                    <button
                      type="button"
                      onClick={() => setIsAdditionalPathsMenuOpen(!isAdditionalPathsMenuOpen)}
                      className="flex items-center gap-1 px-2 py-1 bg-bgSubtle border border-borderSubtle rounded-full text-xs text-textStandard hover:bg-bgStandard transition-colors"
                      title={`Show ${contextFiles.length - 10} more files`}
                    >
                      <span className="text-textSubtle">+{contextFiles.length - 10}</span>
                    </button>

                    {isAdditionalPathsMenuOpen && (
                      <div className="absolute bottom-full left-0 mb-2 w-80 max-h-60 overflow-y-auto bg-bgApp rounded-lg border border-borderSubtle shadow-lg z-20">
                        <div className="p-2">
                          <div className="text-xs text-textSubtle mb-2 px-2 py-1">
                            Additional files ({contextFiles.length - 10}):
                          </div>
                          {contextFiles.slice(10).map((filePath) => (
                            <div
                              key={filePath.path}
                              className="flex items-center gap-2 px-2 py-1 hover:bg-bgSubtle rounded text-xs text-textStandard"
                            >
                              {filePath.type === 'directory' ? (
                                <FolderOpen className="w-3 h-3 text-textSubtle flex-shrink-0" />
                              ) : (
                                <Document className="w-3 h-3 text-textSubtle flex-shrink-0" />
                              )}
                              <span className="truncate flex-1" title={filePath.path}>
                                {filePath.path}
                              </span>
                              <button
                                type="button"
                                onClick={() => {
                                  setSessionFilesFn(
                                    currentSessionFiles.filter((fp) => fp.path !== filePath.path)
                                  );
                                }}
                                className="text-textSubtle hover:text-textStandard transition-colors flex-shrink-0"
                                title="Remove from context"
                              >
                                <Close className="w-3 h-3" />
                              </button>
                            </div>
                          ))}
                        </div>
                      </div>
                    )}
                  </div>
                )}
              </div>
            </div>
          </div>
        )}

        {/* Images above the textarea */}
        {currentSessionFiles.filter((file) => file.type === 'image').length > 0 && (
          <div className="flex flex-wrap gap-2 p-2 pb-0">
            {currentSessionFiles
              .filter((file) => file.type === 'image')
              .map((img) => (
                <div key={img.id} className="relative group w-20 h-20">
                  {img.dataUrl && (
                    <img
                      src={img.dataUrl} // Use dataUrl for instant preview
                      alt={`Pasted image ${img.id}`}
                      className={`w-full h-full object-cover rounded border ${img.error ? 'border-red-500' : 'border-borderStandard'}`}
                    />
                  )}
                  {img.isLoading && (
                    <div className="absolute inset-0 flex items-center justify-center bg-black bg-opacity-50 rounded">
                      <div className="animate-spin rounded-full h-6 w-6 border-t-2 border-b-2 border-white"></div>
                    </div>
                  )}
                  {img.error && !img.isLoading && (
                    <div className="absolute inset-0 flex flex-col items-center justify-center bg-black bg-opacity-75 rounded p-1 text-center">
                      <p className="text-red-400 text-[10px] leading-tight break-all mb-1">
                        {img.error.substring(0, 50)}
                      </p>
                      {img.dataUrl && (
                        <button
                          type="button"
                          onClick={() => handleRetryImageSave(img.id)}
                          className="bg-blue-600 hover:bg-blue-700 text-white rounded px-1 py-0.5 text-[8px] leading-none"
                          title="Retry saving image"
                        >
                          Retry
                        </button>
                      )}
                    </div>
                  )}
                  {!img.isLoading && (
                    <button
                      type="button"
                      onClick={() => handleRemovePastedImage(img.id)}
                      className="absolute -top-1 -right-1 bg-gray-700 hover:bg-red-600 text-white rounded-full w-5 h-5 flex items-center justify-center text-xs leading-none opacity-0 group-hover:opacity-100 focus:opacity-100 transition-opacity z-10"
                      aria-label="Remove image"
                    >
                      <Close className="w-3.5 h-3.5" />
                    </button>
                  )}
                </div>
              ))}
          </div>
        )}

        <textarea
          data-testid="chat-input"
          autoFocus
          id="dynamic-textarea"
          placeholder="What can goose help with?   ⌘↑/⌘↓"
          value={displayValue}
          onChange={handleChange}
          onCompositionStart={handleCompositionStart}
          onCompositionEnd={handleCompositionEnd}
          onKeyDown={handleKeyDown}
          onPaste={handlePaste}
          onFocus={() => setIsFocused(true)}
          onBlur={() => setIsFocused(false)}
          ref={textAreaRef}
          rows={1}
          style={{
            minHeight: `${minHeight}px`,
            maxHeight: `${maxHeight}px`,
            overflowY: 'auto',
          }}
          className="w-full pl-4 pr-[68px] outline-none border-none focus:ring-0 bg-transparent pt-3 pb-1.5 text-sm resize-none text-textStandard placeholder:text-textPlaceholder"
        />

        {isLoading ? (
          <Button
            type="button"
            size="icon"
            variant="ghost"
            onClick={(e) => {
              e.preventDefault();
              e.stopPropagation();
              onStop?.();
            }}
            className="absolute right-3 top-2 text-textSubtle rounded-full border border-borderSubtle hover:border-borderStandard hover:text-textStandard w-7 h-7 [&_svg]:size-4"
          >
            <Stop size={24} />
          </Button>
        ) : (
          <Button
            type="submit"
            size="icon"
            variant="ghost"
            disabled={!hasSubmittableContent || isAnyImageLoading} // Disable if no content or if images are still loading/saving
            className={`absolute right-3 top-2 transition-colors rounded-full w-7 h-7 [&_svg]:size-4 ${
              !hasSubmittableContent || isAnyImageLoading
                ? 'text-textSubtle cursor-not-allowed'
                : 'bg-bgAppInverse text-textProminentInverse hover:cursor-pointer'
            }`}
            title={isAnyImageLoading ? 'Waiting for images to save...' : 'Send'}
          >
            <Send />
          </Button>
        )}
      </form>

      <div className="flex items-center transition-colors text-textSubtle relative text-xs p-2 pr-3 border-t border-borderStandard gap-2">
        <div className="gap-1 flex items-center justify-between w-full">
          <div className="flex items-center gap-1">
            {/* File Attachment Button */}
            <Button
              type="button"
              size="icon"
              variant="ghost"
              onClick={handleSelectFilesAndFolders}
              className="text-textSubtle hover:text-textStandard w-7 h-7 [&_svg]:size-4"
              title="Attach files & folders"
            >
              <Attach />
            </Button>
          </div>

          <BottomMenu
            setView={setView}
            numTokens={numTokens}
            messages={messages}
            isLoading={isLoading}
            setMessages={setMessages}
          />
        </div>
      </div>
    </div>
  );
}
