import React, { useState } from 'react';
import { ThumbsUp, ThumbsDown, MessageSquare } from 'lucide-react';
import { Tooltip, TooltipContent, TooltipTrigger } from './ui/Tooltip';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from './ui/dialog';

interface FeedbackToolbarProps {
  conversationId: string;
  sessionId?: string;
  messages: any[];
  providerUsed?: string;
  modelUsed?: string;
}

export default function FeedbackToolbar({
  conversationId,
  sessionId,
  messages,
  providerUsed,
  modelUsed,
}: FeedbackToolbarProps) {
  const [saving, setSaving] = useState(false);
  const [thanks, setThanks] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const [modalOpen, setModalOpen] = useState(false);
  const [correction, setCorrection] = useState('');
  const [comments, setComments] = useState('');

  const submitRating = async (rating: number) => {
    try {
      setSaving(true);
      setError(null);
      const payload = {
        conversation_id: conversationId,
        session_id: sessionId,
        messages,
        provider_used: providerUsed,
        model_used: modelUsed,
        rating,
      };
      const res = await window.electron.submitTrainingData(payload);
      if ((res as any)?.error) setError((res as any).error);
      else {
        setThanks('Thanks!');
        setTimeout(() => setThanks(null), 2000);
      }
    } catch (e: any) {
      setError(e?.message || 'Failed to submit');
    } finally {
      setSaving(false);
    }
  };

  const submitCorrection = async () => {
    try {
      setSaving(true);
      setError(null);
      const payload = {
        conversation_id: conversationId,
        session_id: sessionId,
        messages,
        provider_used: providerUsed,
        model_used: modelUsed,
        correction: correction.trim() || undefined,
        comments: comments.trim() || undefined,
      };
      const res = await window.electron.submitTrainingData(payload);
      if ((res as any)?.error) setError((res as any).error);
      else {
        setModalOpen(false);
        setCorrection('');
        setComments('');
        setThanks('Thanks!');
        setTimeout(() => setThanks(null), 2000);
      }
    } catch (e: any) {
      setError(e?.message || 'Failed to submit');
    } finally {
      setSaving(false);
    }
  };

  // Shared small icon button style similar to copy button footprint
  const btn =
    'flex font-mono items-center gap-1 text-xs text-textSubtle hover:cursor-pointer hover:text-textProminent transition-all duration-200 -translate-y-4 group-hover:translate-y-0';

  return (
    <div className="flex items-center gap-2">
      <Tooltip>
        <TooltipTrigger asChild>
          <button className={btn} disabled={saving} onClick={() => submitRating(5)} aria-label="Thumbs up">
            <ThumbsUp className="size-4" />
          </button>
        </TooltipTrigger>
        <TooltipContent>Thumbs up</TooltipContent>
      </Tooltip>

      <Tooltip>
        <TooltipTrigger asChild>
          <button className={btn} disabled={saving} onClick={() => submitRating(1)} aria-label="Thumbs down">
            <ThumbsDown className="size-4" />
          </button>
        </TooltipTrigger>
        <TooltipContent>Thumbs down</TooltipContent>
      </Tooltip>

      <Dialog open={modalOpen} onOpenChange={setModalOpen}>
        <Tooltip>
          <TooltipTrigger asChild>
            <DialogTrigger asChild>
              <button className={btn} disabled={saving} aria-label="Add correction or comment">
                <MessageSquare className="size-4" />
                <span className="hidden sm:inline">Feedback</span>
              </button>
            </DialogTrigger>
          </TooltipTrigger>
          <TooltipContent>Correction / comments</TooltipContent>
        </Tooltip>

        <DialogContent>
          <DialogHeader>
            <DialogTitle>Feedback</DialogTitle>
            <DialogDescription>
              Provide a correction and/or comments to help improve future responses.
            </DialogDescription>
          </DialogHeader>

          <div className="flex flex-col gap-3 py-1">
            <label className="text-sm text-text-muted">Correction (optional)</label>
            <textarea
              className="w-full min-h-[90px] rounded border border-borderSubtle bg-background-default p-2 text-sm"
              placeholder="Suggest a better or corrected response"
              value={correction}
              onChange={(e) => setCorrection(e.target.value)}
            />

            <label className="text-sm text-text-muted">Comments (optional)</label>
            <textarea
              className="w-full min-h-[70px] rounded border border-borderSubtle bg-background-default p-2 text-sm"
              placeholder="Additional context or notes"
              value={comments}
              onChange={(e) => setComments(e.target.value)}
            />

            {error && <div className="text-sm text-red-600">{error}</div>}
          </div>

          <DialogFooter>
            <button
              className="rounded bg-primary text-white px-3 h-8 text-sm disabled:opacity-50"
              disabled={saving}
              onClick={submitCorrection}
            >
              {saving ? 'Saving…' : 'Submit'}
            </button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {thanks && <span className="text-xs text-text-muted">{thanks}</span>}
    </div>
  );
}
