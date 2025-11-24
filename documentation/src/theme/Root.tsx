import React from 'react';
import type { ReactNode } from 'react';

interface Props {
  children: ReactNode;
}

const SHOW_BANNER = false;

export default function Root({ children }: Props): JSX.Element {
  return (
    <>
      {SHOW_BANNER && (
        <div
          style={{
            backgroundColor: '#25c2a0',
            color: '#000',
            padding: '8px 16px',
            textAlign: 'center',
            fontSize: '14px',
            fontWeight: '500',
            position: 'relative',
            zIndex: 1000,
            lineHeight: '1.3',
          }}
        >
          ✨ NO KEYBOARDS ALLOWED HACKATHON✨ : build next-gen interfaces with goose and win prizes.{' '}
          <a
            href="https://nokeyboardsallowed.dev"
            target="_blank"
            rel="noopener noreferrer"
            style={{
              color: '#000',
              textDecoration: 'underline',
              fontWeight: '700',
            }}
          >
            Deadline Nov 14
          </a>
          .
        </div>
      )}
      {children}
    </>
  );
}
