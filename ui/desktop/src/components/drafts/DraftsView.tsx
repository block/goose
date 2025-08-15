import React, { useEffect } from 'react';
import { FileText, Trash2, Clock, MessageSquare } from 'lucide-react';
import { Button } from '../ui/button';
import { formatDistanceToNow } from 'date-fns';
import { useNavigate } from 'react-router-dom';
import {
  useDraftContext,
  groupDraftsByDate,
  truncateText,
  type DraftItem,
} from '../../contexts/DraftContext';

interface DraftsViewProps {
  onClose?: () => void;
  showHeader?: boolean;
}

const DraftsView: React.FC<DraftsViewProps> = ({ onClose, showHeader = true }) => {
  const navigate = useNavigate();
  const { getAllDraftItems, refreshDrafts, deleteDraft, deleteAllDrafts } = useDraftContext();
  const drafts = getAllDraftItems();

  useEffect(() => {
    // Refresh drafts when component mounts
    refreshDrafts();

    // Refresh drafts when the window gains focus
    const handleFocus = () => {
      refreshDrafts();
    };

    // Refresh drafts when the tab becomes visible
    const handleVisibilityChange = () => {
      if (!document.hidden) {
        refreshDrafts();
      }
    };

    window.addEventListener('focus', handleFocus);
    document.addEventListener('visibilitychange', handleVisibilityChange);

    return () => {
      window.removeEventListener('focus', handleFocus);
      document.removeEventListener('visibilitychange', handleVisibilityChange);
    };
  }, [refreshDrafts]);

  const handleDeleteDraft = (draftId: string) => {
    deleteDraft(draftId);
  };

  const handleDeleteAllDrafts = () => {
    deleteAllDrafts();
  };

  const handleOpenDraft = (draft: DraftItem) => {
    // Navigate to the appropriate chat view with the draft content
    if (draft.contextType === 'hub') {
      navigate('/', { state: { draftContent: draft.content } });
    } else {
      navigate('/pair', { state: { draftContent: draft.content } });
    }

    // Optionally close the drafts view
    if (onClose) {
      onClose();
    }
  };

  const dateGroups = groupDraftsByDate(drafts);

  const renderContent = () => {
    if (drafts.length === 0) {
      return (
        <div className="flex flex-col items-center justify-center h-full text-center p-8">
          <FileText className="w-16 h-16 text-textSubtle mb-4 opacity-50" />
          <h2 className="text-lg font-medium text-textStandard mb-2">No drafts saved</h2>
          <p className="text-sm text-textSubtle max-w-md">
            Your unsent messages are automatically saved as drafts. They'll appear here when you
            type something in a chat window.
          </p>
        </div>
      );
    }

    if (showHeader) {
      // List view for standalone DraftsView (not used currently but kept for compatibility)
      return (
        <div className="p-4 space-y-3">
          {drafts.map((draft) => (
            <div
              key={draft.id}
              className="group bg-background-muted rounded-lg border border-borderSubtle hover:border-borderStandard transition-all duration-200 overflow-hidden"
            >
              <div className="p-4 cursor-pointer" onClick={() => handleOpenDraft(draft)}>
                <div className="flex items-start justify-between mb-2">
                  <div className="flex items-center gap-2">
                    <MessageSquare className="w-4 h-4 text-textSubtle" />
                    <span className="text-sm font-medium text-textStandard">
                      {draft.title || (draft.contextType === 'hub' ? 'Home Chat' : 'Pair Chat')}
                    </span>
                    <span className="text-xs px-2 py-0.5 bg-background-default rounded-full text-textSubtle">
                      {draft.contextType}
                    </span>
                  </div>
                  <div className="flex items-center gap-1 text-xs text-textSubtle">
                    <Clock className="w-3 h-3" />
                    <span>{formatDistanceToNow(draft.timestamp, { addSuffix: true })}</span>
                  </div>
                </div>
                <div className="text-sm text-textSubtle">{truncateText(draft.content, 150)}</div>
              </div>
              <div className="flex items-center justify-end gap-2 px-4 py-2 bg-background-default border-t border-borderSubtle opacity-0 group-hover:opacity-100 transition-opacity">
                <Button
                  onClick={(e) => {
                    e.stopPropagation();
                    handleDeleteDraft(draft.id);
                  }}
                  variant="ghost"
                  size="xs"
                  className="text-red-500 hover:text-red-600"
                >
                  <Trash2 className="w-3 h-3 mr-1" />
                  Delete
                </Button>
              </div>
            </div>
          ))}
        </div>
      );
    }

    // Grid view for tabbed content
    return (
      <div className="space-y-8">
        <div className="flex items-center justify-between mb-4">
          <div className="text-sm text-textSubtle">
            {drafts.length} draft{drafts.length !== 1 ? 's' : ''}
          </div>
          {drafts.length > 0 && (
            <Button
              onClick={handleDeleteAllDrafts}
              variant="ghost"
              size="sm"
              className="text-red-500 hover:text-red-600 hover:bg-red-50"
            >
              <Trash2 className="w-4 h-4 mr-2" />
              Delete All
            </Button>
          )}
        </div>

        {dateGroups.map((group) => (
          <div key={group.label} className="space-y-4">
            <div className="sticky top-0 z-10 bg-background-default/95 backdrop-blur-sm">
              <h2 className="text-text-muted">{group.label}</h2>
            </div>
            <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 2xl:grid-cols-5 gap-4">
              {group.drafts.map((draft) => (
                <div
                  key={draft.id}
                  className="h-full py-3 px-4 hover:shadow-default cursor-pointer transition-all duration-150 flex flex-col justify-between relative group bg-background-card text-text-default rounded-xl border shadow-sm"
                  onClick={() => handleOpenDraft(draft)}
                >
                  <Button
                    onClick={(e) => {
                      e.stopPropagation();
                      handleDeleteDraft(draft.id);
                    }}
                    className="absolute top-3 right-4 p-2 rounded opacity-0 group-hover:opacity-100 transition-opacity hover:bg-gray-100 dark:hover:bg-gray-700 cursor-pointer"
                    variant="ghost"
                    size="xs"
                    title="Delete draft"
                  >
                    <Trash2 className="w-3 h-3 text-textSubtle hover:text-textStandard" />
                  </Button>

                  <div className="flex-1">
                    <h3 className="text-base mb-1 pr-6 break-words">
                      {draft.title || (draft.contextType === 'hub' ? 'Home Chat' : 'Pair Chat')}
                    </h3>

                    <div className="flex items-center text-text-muted text-xs mb-1">
                      <Clock className="w-3 h-3 mr-1 flex-shrink-0" />
                      <span>{formatDistanceToNow(draft.timestamp, { addSuffix: true })}</span>
                    </div>

                    <div className="text-sm text-textSubtle mt-2">
                      {truncateText(draft.content, 120)}
                    </div>
                  </div>

                  <div className="flex items-center justify-between mt-1 pt-2">
                    <div className="flex items-center space-x-3 text-xs text-text-muted">
                      <div className="flex items-center">
                        <MessageSquare className="w-3 h-3 mr-1" />
                        <span className="font-mono">{draft.content.length} chars</span>
                      </div>
                      <span className="text-xs px-2 py-0.5 bg-background-default rounded-full text-textSubtle">
                        {draft.contextType}
                      </span>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </div>
        ))}
      </div>
    );
  };

  if (!showHeader) {
    // Content-only version for tabbed view
    return (
      <div className="flex-1 min-h-0 relative px-8">
        <div className="h-full overflow-y-auto">{renderContent()}</div>
      </div>
    );
  }

  // Full version with header (not currently used but kept for compatibility)
  return (
    <div className="flex flex-col h-full bg-background-default">
      <div className="flex items-center justify-between p-6 border-b border-borderSubtle">
        <div className="flex items-center gap-3">
          <FileText className="w-6 h-6 text-textStandard" />
          <h1 className="text-xl font-semibold text-textStandard">Drafts</h1>
          <span className="text-sm text-textSubtle">({drafts.length})</span>
        </div>

        {drafts.length > 0 && (
          <Button
            onClick={handleDeleteAllDrafts}
            variant="ghost"
            size="sm"
            className="text-red-500 hover:text-red-600 hover:bg-red-50"
          >
            <Trash2 className="w-4 h-4 mr-2" />
            Delete All
          </Button>
        )}
      </div>

      <div className="flex-1 overflow-y-auto">{renderContent()}</div>
    </div>
  );
};

// Export content-only version for tabbed view
export const DraftsViewContent: React.FC<DraftsViewProps> = (props) => (
  <DraftsView {...props} showHeader={false} />
);

export default DraftsView;
