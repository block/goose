import React, { useState, useImperativeHandle, forwardRef } from 'react';

export interface ProgressBarHandle {
  resetProgressBar: () => void;
}

const ProgressBar = forwardRef<ProgressBarHandle>((props, ref) => {
  const [progress, setProgress] = useState(0); // percentage 0-100
  const [tokensUsed, setTokensUsed] = useState(0);

  // Expose resetProgressBar method to parent components via ref
  useImperativeHandle(ref, () => ({
    resetProgressBar() {
      setProgress(0);
      setTokensUsed(0);
    },
  }));

  // Simulate progress text display and progress bar
  return (
    <div>
      <div style={{ marginBottom: '8px' }}>
        Context Usage: {progress}% ({tokensUsed} tokens)
      </div>
      <progress value={progress} max={100} style={{ width: '100%' }} />
    </div>
  );
});

export default ProgressBar;
