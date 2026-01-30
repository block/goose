import React, { useState } from 'react';

interface RatingControlsProps {
  conversationId: string;
  sessionId?: string;
  messages: any[];
  providerUsed?: string;
  modelUsed?: string;
}

export default function RatingControls({
  conversationId,
  sessionId,
  messages,
  providerUsed,
  modelUsed,
}: RatingControlsProps) {
  const [rating, setRating] = useState<number | null>(null);
  const [correction, setCorrection] = useState<string>('');
  const [comments, setComments] = useState<string>('');
  const [saving, setSaving] = useState(false);
  const [thanks, setThanks] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const submit = async () => {
    try {
      setSaving(true);
      setError(null);
      const payload = {
        conversation_id: conversationId,
        session_id: sessionId,
        messages,
        provider_used: providerUsed,
        model_used: modelUsed,
        rating: rating ?? undefined,
        correction: correction.trim() || undefined,
        comments: comments.trim() || undefined,
      };
      const res = await window.electron.submitTrainingData(payload);
      if ((res as any)?.error) {
        setError((res as any).error);
      } else {
        setThanks('Thanks!');
        setTimeout(() => setThanks(null), 2500);
      }
    } catch (e: any) {
      setError(e?.message || 'Failed to submit');
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="flex items-start gap-2 mt-2 text-sm">
      <div className="flex items-center gap-1">
        <button
          className={`px-2 py-1 rounded border ${rating === 1 ? 'bg-red-600 text-white' : 'border-borderSubtle'}`}
          onClick={() => setRating(1)}
          title="Thumbs Down"
        >👎</button>
        <button
          className={`px-2 py-1 rounded border ${rating === 5 ? 'bg-green-600 text-white' : 'border-borderSubtle'}`}
          onClick={() => setRating(5)}
          title="Thumbs Up"
        >👍</button>
      </div>
      <input
        className="flex-1 px-2 py-1 border rounded text-sm"
        placeholder="Suggest a correction (optional)"
        value={correction}
        onChange={(e) => setCorrection(e.target.value)}
      />
      <input
        className="flex-1 px-2 py-1 border rounded text-sm"
        placeholder="Comments (optional)"
        value={comments}
        onChange={(e) => setComments(e.target.value)}
      />
      <button
        className="px-3 py-1 rounded bg-primary text-white disabled:opacity-50"
        disabled={saving}
        onClick={submit}
        title="Submit feedback"
      >{saving ? 'Saving…' : 'Submit'}</button>
      {thanks && <span className="ml-2 text-textSubtle">{thanks}</span>}
      {error && <span className="ml-2 text-red-600">{error}</span>}
    </div>
  );
}
