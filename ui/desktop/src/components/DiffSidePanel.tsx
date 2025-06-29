import { useState, useMemo } from 'react';
import { ScrollArea } from './ui/scroll-area';
import {
  SquareSplitHorizontal,
  BetweenHorizontalStart,
  FileDiff,
  PanelRightOpen,
  Check,
  X,
} from 'lucide-react';

interface DiffLine {
  type: 'context' | 'added' | 'removed' | 'header';
  content: string;
  oldLineNumber?: number;
  newLineNumber?: number;
}

interface DiffHunk {
  id: string;
  header: string;
  lines: DiffLine[];
  oldStart: number;
  oldCount: number;
  newStart: number;
  newCount: number;
}

interface DiffFile {
  fileName: string;
  hunks: DiffHunk[];
}

interface DiffSidePanelProps {
  diffContent: string;
  isOpen: boolean;
  onClose: () => void;
  onApplyHunk?: (fileIndex: number, hunkId: string) => void;
  onRejectHunk?: (fileIndex: number, hunkId: string) => void;
  onApplyFile?: (fileIndex: number) => void;
  onRejectFile?: (fileIndex: number) => void;
  enableActions?: boolean;
}

export default function DiffSidePanel({
  diffContent,
  isOpen,
  onClose,
  onApplyHunk,
  onRejectHunk,
  onApplyFile,
  onRejectFile,
  enableActions = true,
}: DiffSidePanelProps) {
  const [viewMode, setViewMode] = useState<'unified' | 'split'>('unified');
  const [appliedHunks, setAppliedHunks] = useState<Set<string>>(new Set());
  const [rejectedHunks, setRejectedHunks] = useState<Set<string>>(new Set());

  const parsedDiff = useMemo(() => parseDiff(diffContent), [diffContent]);

  const handleApplyHunk = (fileIndex: number, hunkId: string) => {
    setAppliedHunks((prev) => new Set([...prev, hunkId]));
    setRejectedHunks((prev) => {
      const newSet = new Set(prev);
      newSet.delete(hunkId);
      return newSet;
    });
    onApplyHunk?.(fileIndex, hunkId);
  };

  const handleRejectHunk = (fileIndex: number, hunkId: string) => {
    setRejectedHunks((prev) => new Set([...prev, hunkId]));
    setAppliedHunks((prev) => {
      const newSet = new Set(prev);
      newSet.delete(hunkId);
      return newSet;
    });
    onRejectHunk?.(fileIndex, hunkId);
  };

  const handleApplyFile = (fileIndex: number) => {
    const file = parsedDiff[fileIndex];
    const hunkIds = file.hunks.map((h) => h.id);
    setAppliedHunks((prev) => new Set([...prev, ...hunkIds]));
    setRejectedHunks((prev) => {
      const newSet = new Set(prev);
      hunkIds.forEach((id) => newSet.delete(id));
      return newSet;
    });
    onApplyFile?.(fileIndex);
  };

  const handleRejectFile = (fileIndex: number) => {
    const file = parsedDiff[fileIndex];
    const hunkIds = file.hunks.map((h) => h.id);
    setRejectedHunks((prev) => new Set([...prev, ...hunkIds]));
    setAppliedHunks((prev) => {
      const newSet = new Set(prev);
      hunkIds.forEach((id) => newSet.delete(id));
      return newSet;
    });
    onRejectFile?.(fileIndex);
  };

  if (!isOpen) return null;

  const toggleBaseStyles =
    'flex items-center gap-1 [&_svg]:size-4 h-8 px-4 text-xs hover:text-textStandard transition-all duration-200 ease-in-out';
  const toggleActiveStyles = `${toggleBaseStyles} bg-bgSubtle text-textStandard`;
  const toggleInactiveStyles = `${toggleBaseStyles} bg-background text-textSubtle`;

  return (
    <div className="flex-1 p-6 bg-bgSubtle animate-in slide-in-from-right duration-300 ease-out">
      <div className="flex flex-col bg-bgApp rounded-lg h-full overflow-hidden text-textStandard border border-borderSubtle shadow-lg">
        {/* Header */}
        <div className="flex items-center justify-between p-3 border-b border-borderSubtle">
          <h2 className="text-textSubtle font-medium text-sm inline-flex items-center gap-2">
            <FileDiff size={16} />
            Diff Viewer
          </h2>

          <div className="flex border hover:cursor-pointer border-borderSubtle hover:border-borderStandard rounded-lg overflow-hidden transition-all duration-200 ease-in-out">
            <button
              className={viewMode === 'unified' ? toggleActiveStyles : toggleInactiveStyles}
              onClick={() => setViewMode('unified')}
            >
              <BetweenHorizontalStart size={16} className="" />
              Unified
            </button>
            <button
              className={viewMode === 'split' ? toggleActiveStyles : toggleInactiveStyles}
              onClick={() => setViewMode('split')}
            >
              <SquareSplitHorizontal size={16} className="" />
              Split
            </button>
          </div>
          <button
            onClick={onClose}
            className="w-7 h-7 p-1 rounded-full border border-borderSubtle transition-all duration-200 ease-in-out cursor-pointer no-drag hover:text-textStandard hover:border-borderStandard hover:bg-bgSubtle flex items-center justify-center text-textSubtle transform hover:scale-105"
            title="Close diff viewer"
          >
            <PanelRightOpen size={16} />
          </button>
        </div>
        {/* Content */}
        <ScrollArea className="h-full">
          {parsedDiff.map((file, fileIndex) => (
            <div key={fileIndex} className="m-4  ">
              {/* File header */}
              <div className="bg-bgApp p-3 flex items-center justify-between rounded-t-lg bg-bgApp overflow-hidden border border-borderSubtle sticky top-2 z-10 shadow-[0_-15px_0px_var(--background-app)] transition-all duration-200 ease-in-out">
                <div className="font-mono text-sm truncate">{file.fileName}</div>
                {enableActions && (
                  <div className="flex gap-4 flex-shrink-0 text-xs">
                    <button
                      onClick={() => handleRejectFile(fileIndex)}
                      className="flex items-center text-red-500 hover:text-red-600 transition-colors duration-200 ease-in-out transform hover:scale-105"
                    >
                      <X strokeWidth="3" size={16} />
                      Reject All
                    </button>
                    <button
                      onClick={() => handleApplyFile(fileIndex)}
                      className="text-green-500 hover:text-green-600 flex items-center transition-colors duration-200 ease-in-out transform hover:scale-105"
                    >
                      <Check size={16} strokeWidth={2} />
                      Apply All
                    </button>
                  </div>
                )}
              </div>
              <div className="rounded-b-lg overflow-hidden border border-borderSubtle border-t-0">
                {/* Hunks */}
                {file.hunks.map((hunk) => (
                  <DiffHunkView
                    key={hunk.id}
                    hunk={hunk}
                    fileIndex={fileIndex}
                    viewMode={viewMode}
                    isApplied={appliedHunks.has(hunk.id)}
                    isRejected={rejectedHunks.has(hunk.id)}
                    onApply={() => handleApplyHunk(fileIndex, hunk.id)}
                    onReject={() => handleRejectHunk(fileIndex, hunk.id)}
                    enableActions={enableActions}
                  />
                ))}
              </div>
            </div>
          ))}
        </ScrollArea>
      </div>
    </div>
  );
}

interface DiffHunkViewProps {
  hunk: DiffHunk;
  fileIndex: number;
  viewMode: 'unified' | 'split';
  isApplied: boolean;
  isRejected: boolean;
  onApply: () => void;
  onReject: () => void;
  enableActions: boolean;
}

function DiffHunkView({
  hunk,
  viewMode,
  isApplied,
  isRejected,
  onApply,
  onReject,
  enableActions,
}: DiffHunkViewProps) {
  const getHunkStatus = () => {
    if (isApplied) return 'applied';
    if (isRejected) return 'rejected';
    return 'pending';
  };
  console.log('hunk', hunk);
  const status = getHunkStatus();

  return (
    <div
      className={`${
        status === 'applied'
          ? 'bg-green-50 dark:bg-green-900/20'
          : status === 'rejected'
            ? 'bg-red-50 dark:bg-red-900/20'
            : ''
      }`}
    >
      {/* Hunk header */}
      <div className="p-2 flex items-center justify-between bg-bgSubtle">
        <div className="font-mono text-xs text-gray-600 dark:text-gray-400 truncate flex-1 mr-2">
          {hunk.header}
        </div>

        {enableActions && (
          <div className="flex gap-2 flex-shrink-0">
            <button
              onClick={onApply}
              disabled={isApplied}
              className={`px-2 py-1 text-xs rounded transition-all duration-200 ease-in-out transform hover:scale-105 ${
                isApplied
                  ? 'bg-green-200 text-green-800 dark:bg-green-800 dark:text-green-200'
                  : 'bg-green-500 hover:bg-green-600 text-white'
              }`}
            >
              {isApplied ? 'Applied' : 'Apply'}
            </button>
            <button
              onClick={onReject}
              disabled={isRejected}
              className={`px-2 py-1 text-xs rounded transition-all duration-200 ease-in-out transform hover:scale-105 ${
                isRejected
                  ? 'bg-red-200 text-red-800 dark:bg-red-800 dark:text-red-200'
                  : 'bg-red-500 hover:bg-red-600 text-white'
              }`}
            >
              {isRejected ? 'Rejected' : 'Reject'}
            </button>
          </div>
        )}
      </div>

      {/* Hunk content */}
      {viewMode === 'unified' ? (
        <UnifiedDiffView lines={hunk.lines} />
      ) : (
        <SplitDiffView lines={hunk.lines} />
      )}
    </div>
  );
}

function UnifiedDiffView({ lines }: { lines: DiffLine[] }) {
  return (
    <div className="font-mono text-sm">
      {lines
        .filter((line) => line.type !== 'header')
        .map((line, index) => (
          <div
            key={index}
            className={`flex ${
              line.type === 'added'
                ? 'bg-green-100 dark:bg-green-900/30'
                : line.type === 'removed'
                  ? 'bg-red-100 dark:bg-red-900/30'
                  : line.type === 'header'
                    ? 'bg-blue-100 dark:bg-blue-900/30'
                    : ''
            }`}
          >
            <div className="w-12 px-1 text-gray-500 dark:text-gray-400 text-right border-r border-gray-200 dark:border-gray-700 text-xs">
              {line.oldLineNumber || ''}
            </div>
            <div className="w-12 px-1 text-gray-500 dark:text-gray-400 text-right border-r border-gray-200 dark:border-gray-700 text-xs">
              {line.newLineNumber || ''}
            </div>
            <div className="flex-1 px-2  overflow-x-auto">
              <span
                className={`inline-block w-4 ${
                  line.type === 'added'
                    ? 'text-green-600 dark:text-green-400'
                    : line.type === 'removed'
                      ? 'text-red-600 dark:text-red-400'
                      : ''
                }`}
              >
                {line.type === 'added' ? '+' : line.type === 'removed' ? '-' : ' '}
              </span>
              <span className="ml-1 text-xs">{line.content}</span>
            </div>
          </div>
        ))}
    </div>
  );
}

function SplitDiffView({ lines }: { lines: DiffLine[] }) {
  const leftLines: DiffLine[] = [];
  const rightLines: DiffLine[] = [];

  // Group lines for split view
  for (const line of lines) {
    console.log('line', line);
    if (line.type === 'removed') {
      leftLines.push(line);
      rightLines.push({
        type: 'context',
        content: '',
        oldLineNumber: undefined,
        newLineNumber: undefined,
      });
    } else if (line.type === 'added') {
      if (leftLines.length > rightLines.length) {
        rightLines[rightLines.length - 1] = line;
      } else {
        leftLines.push({
          type: 'context',
          content: '',
          oldLineNumber: undefined,
          newLineNumber: undefined,
        });
        rightLines.push(line);
      }
    } else {
      leftLines.push(line);
      rightLines.push(line);
    }
  }

  // Ensure both sides have the same number of lines
  while (leftLines.length < rightLines.length) {
    leftLines.push({
      type: 'context',
      content: '',
      oldLineNumber: undefined,
      newLineNumber: undefined,
    });
  }
  while (rightLines.length < leftLines.length) {
    rightLines.push({
      type: 'context',
      content: '',
      oldLineNumber: undefined,
      newLineNumber: undefined,
    });
  }

  return (
    <div className="flex font-mono text-sm break-all">
      {/* Left side (old) */}
      <div className="flex-1 border-r border-gray-200 dark:border-gray-700">
        {leftLines
          .filter((line) => line.type !== 'header')
          .map((line, index) => (
            <div
              key={index}
              className={`flex ${
                line.type === 'removed'
                  ? 'bg-red-100 dark:bg-red-900/30'
                  : line.type === 'header'
                    ? 'bg-blue-100 dark:bg-blue-900/30'
                    : ''
              }`}
            >
              <div className="w-12 px-1 text-gray-500 dark:text-gray-400 text-right border-r border-gray-200 dark:border-gray-700 text-xs">
                {line.oldLineNumber || ''}
              </div>
              <div className="flex-1 px-2 overflow-x-auto">
                <span
                  className={`inline-block w-4 ${
                    line.type === 'removed' ? 'text-red-600 dark:text-red-400' : ''
                  }`}
                >
                  {line.type === 'removed' ? '-' : ' '}
                </span>
                <span className="ml-1 text-xs">{line.content}</span>
              </div>
            </div>
          ))}
      </div>

      {/* Right side (new) */}
      <div className="flex-1">
        {rightLines
          .filter((line) => line.type !== 'header')
          .map((line, index) => (
            <div
              key={index}
              className={`flex ${
                line.type === 'added'
                  ? 'bg-green-100 dark:bg-green-900/30'
                  : line.type === 'header'
                    ? 'bg-blue-100 dark:bg-blue-900/30'
                    : ''
              }`}
            >
              <div className="w-12 px-1 text-gray-500 dark:text-gray-400 text-right border-r border-gray-200 dark:border-gray-700 text-xs">
                {line.newLineNumber || ''}
              </div>
              <div className="flex-1 px-2 overflow-x-auto">
                <span
                  className={`inline-block w-4 ${
                    line.type === 'added' ? 'text-green-600 dark:text-green-400' : ''
                  }`}
                >
                  {line.type === 'added' ? '+' : ' '}
                </span>
                <span className="ml-1 text-xs">{line.content}</span>
              </div>
            </div>
          ))}
      </div>
    </div>
  );
}

function parseDiff(diffContent: string): DiffFile[] {
  const lines = diffContent.split('\n');
  const files: DiffFile[] = [];
  let currentFile: DiffFile | null = null;
  let currentHunk: DiffHunk | null = null;
  let oldLineNumber = 0;
  let newLineNumber = 0;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];

    // File header (diff --git)
    if (line.startsWith('diff --git')) {
      // Extract filename from diff --git a/path/to/file b/path/to/file
      const match = line.match(/^diff --git a\/(.+) b\/(.+)$/);
      if (match) {
        const fileName = match[2]; // Use the 'b/' version (new file path)
        currentFile = {
          fileName,
          hunks: [],
        };
        files.push(currentFile);
      }
      continue;
    }

    // Handle --- and +++ lines as fallback if no diff --git line was found
    if (line.startsWith('--- ') || line.startsWith('+++ ')) {
      if (line.startsWith('--- ') && !currentFile) {
        // Fallback: extract filename from --- line if no diff --git was found
        const fileName = line.substring(4).replace(/^a\//, '');
        currentFile = {
          fileName,
          hunks: [],
        };
        files.push(currentFile);
      } else if (line.startsWith('+++ ') && currentFile && currentFile.fileName === 'a') {
        // Fix the filename if it was incorrectly set to 'a' from --- line
        const fileName = line.substring(4).replace(/^b\//, '');
        currentFile.fileName = fileName;
      }
      continue;
    }

    // Hunk header (@@ -x,y +a,b @@)
    if (line.startsWith('@@')) {
      const match = line.match(/@@ -(\d+),?(\d*) \+(\d+),?(\d*) @@(.*)/);
      if (match && currentFile) {
        const [, oldStart, oldCount, newStart, newCount, context] = match;
        currentHunk = {
          id: `${currentFile.fileName}-${files.length}-${currentFile.hunks.length}`,
          header: line,
          lines: [],
          oldStart: parseInt(oldStart),
          oldCount: parseInt(oldCount) || 1,
          newStart: parseInt(newStart),
          newCount: parseInt(newCount) || 1,
        };
        currentFile.hunks.push(currentHunk);
        oldLineNumber = parseInt(oldStart);
        newLineNumber = parseInt(newStart);

        // Add header line
        currentHunk.lines.push({
          type: 'header',
          content: context.trim(),
        });
      }
      continue;
    }

    // Diff content lines
    if (currentHunk) {
      if (line.startsWith('+')) {
        currentHunk.lines.push({
          type: 'added',
          content: line.substring(1),
          newLineNumber: newLineNumber++,
        });
      } else if (line.startsWith('-')) {
        currentHunk.lines.push({
          type: 'removed',
          content: line.substring(1),
          oldLineNumber: oldLineNumber++,
        });
      } else if (line.startsWith(' ') || line === '') {
        currentHunk.lines.push({
          type: 'context',
          content: line.substring(1),
          oldLineNumber: oldLineNumber++,
          newLineNumber: newLineNumber++,
        });
      }
    }
  }

  // Post-process to split large hunks into smaller, more manageable chunks
  return files.map((file) => ({
    ...file,
    hunks: splitLargeHunks(file.hunks),
  }));
}

function splitLargeHunks(hunks: DiffHunk[]): DiffHunk[] {
  const result: DiffHunk[] = [];
  const MAX_HUNK_SIZE = 50; // Maximum lines per hunk for better UX
  const CONTEXT_LINES = 3; // Lines of context to keep around changes

  for (const hunk of hunks) {
    if (hunk.lines.length <= MAX_HUNK_SIZE) {
      result.push(hunk);
      continue;
    }

    // Split large hunk into smaller chunks
    const chunks = splitHunkIntoChunks(hunk, MAX_HUNK_SIZE, CONTEXT_LINES);
    result.push(...chunks);
  }

  return result;
}

function splitHunkIntoChunks(hunk: DiffHunk, maxSize: number, contextLines: number): DiffHunk[] {
  const chunks: DiffHunk[] = [];
  const lines = hunk.lines.filter((line) => line.type !== 'header'); // Remove header line for processing

  if (lines.length <= maxSize) {
    return [hunk];
  }

  let currentChunk: DiffLine[] = [];
  let chunkIndex = 0;
  let oldLineStart = hunk.oldStart;
  let newLineStart = hunk.newStart;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    currentChunk.push(line);

    // Check if we should start a new chunk
    const shouldSplit =
      currentChunk.length >= maxSize && (line.type === 'context' || i === lines.length - 1);

    if (shouldSplit || i === lines.length - 1) {
      // Create chunk with context
      const chunkLines: DiffLine[] = [];

      // Add header
      chunkLines.push({
        type: 'header',
        content: `Chunk ${chunkIndex + 1}`,
      });

      // Add the actual lines
      chunkLines.push(...currentChunk);

      // Calculate line numbers for this chunk
      const oldCount = currentChunk.filter((l) => l.oldLineNumber !== undefined).length;
      const newCount = currentChunk.filter((l) => l.newLineNumber !== undefined).length;

      chunks.push({
        id: `${hunk.id}-chunk-${chunkIndex}`,
        header: `@@ -${oldLineStart},${oldCount} +${newLineStart},${newCount} @@ Chunk ${chunkIndex + 1}`,
        lines: chunkLines,
        oldStart: oldLineStart,
        oldCount: oldCount,
        newStart: newLineStart,
        newCount: newCount,
      });

      // Prepare for next chunk
      if (i < lines.length - 1) {
        // Update line starts for next chunk
        // Find the last line with old/new line numbers using reverse iteration
        let lastOldLine: number | undefined;
        let lastNewLine: number | undefined;

        for (let j = currentChunk.length - 1; j >= 0; j--) {
          if (lastOldLine === undefined && currentChunk[j].oldLineNumber !== undefined) {
            lastOldLine = currentChunk[j].oldLineNumber;
          }
          if (lastNewLine === undefined && currentChunk[j].newLineNumber !== undefined) {
            lastNewLine = currentChunk[j].newLineNumber;
          }
          if (lastOldLine !== undefined && lastNewLine !== undefined) {
            break;
          }
        }

        if (lastOldLine !== undefined) oldLineStart = lastOldLine + 1;
        if (lastNewLine !== undefined) newLineStart = lastNewLine + 1;

        // Keep some context lines for the next chunk
        const contextStart = Math.max(0, currentChunk.length - contextLines);
        currentChunk = currentChunk.slice(contextStart);
        chunkIndex++;
      }
    }
  }

  return chunks.length > 0 ? chunks : [hunk];
}
