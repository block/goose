import { useState } from 'react';
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from './ui/collapsible';
import { Button } from './ui/button';
import { ChevronDown, ChevronUp, Image as ImageIcon, File } from 'lucide-react';

interface AttachmentItem {
  id: string;
  name?: string;
  path: string;
  type: 'image' | 'file';
  dataUrl?: string;
  isLoading?: boolean;
  error?: string;
}

interface AttachmentSummaryProps {
  images: Array<{
    id: string;
    filePath?: string;
    dataUrl?: string;
    isLoading?: boolean;
    error?: string;
  }>;
  files: Array<{
    id: string;
    name: string;
    path: string;
    isImage?: boolean;
    dataUrl?: string;
    isLoading?: boolean;
    error?: string;
  }>;
  onRemoveImage?: (id: string) => void;
  onRemoveFile?: (id: string) => void;
  onRetryImage?: (id: string) => void;
}

export default function AttachmentSummary({
  images,
  files,
  onRemoveImage,
  onRemoveFile,
  onRetryImage,
}: AttachmentSummaryProps) {
  const [isExpanded, setIsExpanded] = useState(false);

  // Count all images and files (including loading/error states)
  const imageCount = images.length;
  const fileCount = files.length;
  const totalAttachments = imageCount + fileCount;

  if (totalAttachments === 0) {
    return null;
  }

  // only show valid ones
  const validImages = images.filter((img) => img.filePath && !img.error && !img.isLoading);
  const validFiles = files.filter((file) => !file.error && !file.isLoading);

  // Build attachment items for the expanded view
  const attachmentItems: AttachmentItem[] = [
    ...validImages.map((img) => ({
      id: img.id,
      path: img.filePath!,
      type: 'image' as const,
      dataUrl: img.dataUrl,
    })),
    ...validFiles.map((file) => ({
      id: file.id,
      name: file.name,
      path: file.path,
      type: file.isImage ? ('image' as const) : ('file' as const),
      dataUrl: file.dataUrl,
    })),
  ];

  return (
    <div className="border-t border-borderSubtle bg-bgSubtle">
      <Collapsible open={isExpanded} onOpenChange={setIsExpanded}>
        <CollapsibleTrigger asChild>
          <Button
            type="button"
            variant="ghost"
            className="w-full justify-between px-4 py-3 h-auto hover:bg-bgSecondary rounded-none border-b border-borderSubtle"
          >
            <div className="flex items-center gap-3 text-sm font-medium text-textStandard">
              {imageCount > 0 && (
                <span className="flex items-center gap-1.5">
                  <ImageIcon className="w-4 h-4 text-blue-500" />
                  <span>
                    {imageCount} {imageCount === 1 ? 'image' : 'images'}
                  </span>
                </span>
              )}
              {fileCount > 0 && (
                <span className="flex items-center gap-1.5">
                  <File className="w-4 h-4 text-purple-500" />
                  <span>
                    {fileCount} {fileCount === 1 ? 'file' : 'files'}
                  </span>
                </span>
              )}
              <span className="text-xs text-textSubtle font-normal ml-2">
                (Click to view files)
              </span>
            </div>
            {isExpanded ? (
              <ChevronUp className="w-4 h-4 text-textSubtle" />
            ) : (
              <ChevronDown className="w-4 h-4 text-textSubtle" />
            )}
          </Button>
        </CollapsibleTrigger>
        <CollapsibleContent>
          <div className="px-4 pb-4 space-y-3">
            {/* image and file previews */}
            <div className="flex flex-wrap gap-2 mt-4">
              {images.map((img) => {
                if (img.isLoading) {
                  return (
                    <div
                      key={img.id}
                      className="relative w-20 h-20 flex items-center justify-center bg-bgSecondary rounded border border-borderStandard"
                    >
                      <div className="animate-spin rounded-full h-6 w-6 border-t-2 border-b-2 border-textSubtle"></div>
                    </div>
                  );
                }
                if (img.error) {
                  return (
                    <div
                      key={img.id}
                      className="relative w-20 h-20 flex flex-col items-center justify-center bg-bgSecondary rounded border border-red-500 p-1"
                    >
                      <p className="text-red-400 text-[10px] leading-tight break-all text-center mb-1">
                        {img.error.substring(0, 30)}
                      </p>
                      {img.dataUrl && onRetryImage && (
                        <Button
                          type="button"
                          variant="outline"
                          size="xs"
                          onClick={() => onRetryImage(img.id)}
                          className="text-[10px] px-1 py-0.5"
                        >
                          Retry
                        </Button>
                      )}
                      {onRemoveImage && (
                        <Button
                          type="button"
                          variant="outline"
                          size="xs"
                          onClick={() => onRemoveImage(img.id)}
                          className="absolute -top-1 -right-1 opacity-0 group-hover:opacity-100 transition-opacity z-10"
                        >
                          ×
                        </Button>
                      )}
                    </div>
                  );
                }
                if (!img.filePath) return null;
                return (
                  <div key={img.id} className="relative group">
                    {img.dataUrl && (
                      <img
                        src={img.dataUrl}
                        alt="Attachment preview"
                        className="w-20 h-20 object-cover rounded border border-borderStandard"
                      />
                    )}
                    {onRemoveImage && (
                      <Button
                        type="button"
                        variant="outline"
                        size="xs"
                        onClick={() => onRemoveImage(img.id)}
                        className="absolute -top-1 -right-1 opacity-0 group-hover:opacity-100 transition-opacity z-10"
                      >
                        ×
                      </Button>
                    )}
                  </div>
                );
              })}
              {files.map((file) => {
                if (file.isLoading) {
                  return (
                    <div
                      key={file.id}
                      className="relative w-20 h-20 flex items-center justify-center bg-bgSecondary rounded border border-borderStandard"
                    >
                      <div className="animate-spin rounded-full h-6 w-6 border-t-2 border-b-2 border-textSubtle"></div>
                    </div>
                  );
                }
                if (file.error) {
                  return (
                    <div
                      key={file.id}
                      className="relative flex items-center gap-2 px-3 py-2 bg-bgSecondary border border-red-500 rounded-lg"
                    >
                      <File className="w-4 h-4 text-red-400" />
                      <div className="flex-1 min-w-0">
                        <p className="text-sm text-red-400 truncate max-w-[150px]">{file.name}</p>
                        <p className="text-xs text-red-400">{file.error.substring(0, 30)}</p>
                      </div>
                      {onRemoveFile && (
                        <Button
                          type="button"
                          variant="outline"
                          size="xs"
                          onClick={() => onRemoveFile(file.id)}
                          className="opacity-0 group-hover:opacity-100 transition-opacity"
                        >
                          ×
                        </Button>
                      )}
                    </div>
                  );
                }
                if (file.isImage && file.dataUrl) {
                  return (
                    <div key={file.id} className="relative group">
                      <img
                        src={file.dataUrl}
                        alt={file.name}
                        className="w-20 h-20 object-cover rounded border border-borderStandard"
                      />
                      {onRemoveFile && (
                        <Button
                          type="button"
                          variant="outline"
                          size="xs"
                          onClick={() => onRemoveFile(file.id)}
                          className="absolute -top-1 -right-1 opacity-0 group-hover:opacity-100 transition-opacity z-10"
                        >
                          ×
                        </Button>
                      )}
                    </div>
                  );
                }
                return (
                  <div
                    key={file.id}
                    className="relative group flex items-center gap-2 px-3 py-2 bg-bgSecondary border border-borderStandard rounded-lg"
                  >
                    <File className="w-4 h-4 text-textSubtle" />
                    <span className="text-sm text-textStandard truncate max-w-[150px]">
                      {file.name}
                    </span>
                    {onRemoveFile && (
                      <Button
                        type="button"
                        variant="outline"
                        size="xs"
                        onClick={() => onRemoveFile(file.id)}
                        className="opacity-0 group-hover:opacity-100 transition-opacity"
                      >
                        ×
                      </Button>
                    )}
                  </div>
                );
              })}
            </div>

            {/* Expandable path details */}
            <div className="space-y-2 pt-2 border-t border-borderSubtle">
              <div className="text-xs font-medium text-textSubtle uppercase tracking-wide">
                Local Paths
              </div>
              <div className="space-y-1.5">
                {attachmentItems.map((item) => (
                  <div
                    key={item.id}
                    className="flex items-start gap-2 p-2 bg-bgSecondary rounded border border-borderSubtle hover:border-borderStandard transition-colors"
                  >
                    <div className="shrink-0 mt-0.5">
                      {item.type === 'image' ? (
                        <ImageIcon className="w-4 h-4 text-textSubtle" />
                      ) : (
                        <File className="w-4 h-4 text-textSubtle" />
                      )}
                    </div>
                    <div className="flex-1 min-w-0">
                      {item.name && (
                        <div className="text-sm font-medium text-textStandard mb-0.5">
                          {item.name}
                        </div>
                      )}
                      <div className="text-xs font-mono text-textSubtle break-all">{item.path}</div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </CollapsibleContent>
      </Collapsible>
    </div>
  );
}
