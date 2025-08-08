import { describe, it, expect, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import MarkdownContent from './MarkdownContent';

// Mock the icons to avoid import issues
vi.mock('./icons', () => ({
  Check: () => <div data-testid="check-icon">✓</div>,
  Copy: () => <div data-testid="copy-icon">📋</div>,
}));

describe('MarkdownContent', () => {
  describe('HTML Security Integration', () => {
    it('renders safe markdown content normally', async () => {
      const content = `# Test Title

Visit <https://example.com> for more info.

Contact <admin@example.com> for support.

Use \`Array<T>\` for generics.`;

      render(<MarkdownContent content={content} />);

      await waitFor(() => {
        expect(screen.getByText('Test Title')).toBeInTheDocument();
        expect(screen.getByText(/Visit/)).toBeInTheDocument();
        expect(screen.getByText(/for more info/)).toBeInTheDocument();
        expect(screen.getByText(/Contact/)).toBeInTheDocument();
        expect(screen.getByText(/for support/)).toBeInTheDocument();
      });

      // Should not create extra code blocks for safe content
      const codeBlocks = screen.queryAllByText(/```html/);
      expect(codeBlocks).toHaveLength(0);
    });

    it('wraps dangerous HTML in code blocks', async () => {
      const content = `# Security Test

This is safe text.

<script>alert('xss')</script>

More safe text.`;

      render(<MarkdownContent content={content} />);

      await waitFor(() => {
        expect(screen.getByText('Security Test')).toBeInTheDocument();
        expect(screen.getByText('This is safe text.')).toBeInTheDocument();
        expect(screen.getByText('More safe text.')).toBeInTheDocument();
      });

      // The script tag should be in a code block, not executed
      const scriptElements = document.querySelectorAll('script');
      expect(scriptElements).toHaveLength(0); // No actual script tags should be created

      // Should find the script content in a code block (text may be split across spans)
      await waitFor(() => {
        expect(screen.getByText(/alert/)).toBeInTheDocument();
        expect(screen.getByText(/xss/)).toBeInTheDocument();
      });
    });

    it('handles HTML comments securely', async () => {
      const content = `# Comment Test

<!-- This is a malicious comment -->

Normal text continues.`;

      render(<MarkdownContent content={content} />);

      await waitFor(() => {
        expect(screen.getByText('Comment Test')).toBeInTheDocument();
        expect(screen.getByText('Normal text continues.')).toBeInTheDocument();
      });

      // Comment should be in a code block
      await waitFor(() => {
        expect(screen.getByText(/This is a malicious comment/)).toBeInTheDocument();
      });
    });

    it('preserves existing code blocks', async () => {
      const content = `# Code Block Test

\`\`\`javascript
const html = "<div>This is safe in a code block</div>";
console.log(html);
\`\`\`

<div>This should be wrapped</div>`;

      render(<MarkdownContent content={content} />);

      await waitFor(() => {
        expect(screen.getByText('Code Block Test')).toBeInTheDocument();
      });

      // Should preserve the original JavaScript code block (text may be split)
      await waitFor(() => {
        expect(screen.getByText(/const/)).toBeInTheDocument();
        expect(screen.getAllByText(/html/)).toHaveLength(2); // Variable name and function parameter
      });

      // The div outside the code block should be wrapped
      await waitFor(() => {
        expect(screen.getByText(/This should be wrapped/)).toBeInTheDocument();
      });
    });

    it('handles mixed safe and unsafe content', async () => {
      const content = `# Mixed Content Test

1. Auto-link: <https://block.dev>
2. Inline code: \`const x = Array<T>();\`
3. Real markup: <input type="text" disabled>
4. Placeholder path: <project-root>/src`;

      render(<MarkdownContent content={content} />);

      await waitFor(() => {
        expect(screen.getByText('Mixed Content Test')).toBeInTheDocument();
        expect(screen.getByText(/Auto-link/)).toBeInTheDocument();
        expect(screen.getByText(/Inline code/)).toBeInTheDocument();
        expect(screen.getByText(/Real markup/)).toBeInTheDocument();
        expect(screen.getByText(/Placeholder path/)).toBeInTheDocument();
      });

      // Only the input tag should be wrapped
      await waitFor(() => {
        expect(screen.getByText(/input/)).toBeInTheDocument();
        expect(screen.getByText(/type/)).toBeInTheDocument();
        expect(screen.getByText(/disabled/)).toBeInTheDocument();
      });

      // Should not have actual input elements in the DOM
      const inputElements = document.querySelectorAll('input');
      expect(inputElements).toHaveLength(0);
    });
  });

  describe('Code Block Functionality', () => {
    it('renders code blocks with syntax highlighting', async () => {
      const content = `\`\`\`javascript
console.log('Hello, World!');
\`\`\``;

      render(<MarkdownContent content={content} />);

      await waitFor(() => {
        expect(screen.getByText(/console/)).toBeInTheDocument();
        expect(screen.getByText(/log/)).toBeInTheDocument();
        expect(screen.getByText(/Hello, World!/)).toBeInTheDocument();
      });
    });

    it('renders inline code', async () => {
      const content = 'Use `console.log()` to debug.';

      render(<MarkdownContent content={content} />);

      await waitFor(() => {
        expect(screen.getByText(/Use/)).toBeInTheDocument();
        expect(screen.getByText(/to debug/)).toBeInTheDocument();
        expect(screen.getByText('console.log()')).toBeInTheDocument();
      });
    });
  });

  describe('Markdown Features', () => {
    it('renders headers correctly', async () => {
      const content = `# H1 Header
## H2 Header
### H3 Header`;

      render(<MarkdownContent content={content} />);

      await waitFor(() => {
        expect(screen.getByRole('heading', { level: 1, name: 'H1 Header' })).toBeInTheDocument();
        expect(screen.getByRole('heading', { level: 2, name: 'H2 Header' })).toBeInTheDocument();
        expect(screen.getByRole('heading', { level: 3, name: 'H3 Header' })).toBeInTheDocument();
      });
    });

    it('renders lists correctly', async () => {
      const content = `- Item 1
- Item 2
- Item 3

1. Numbered 1
2. Numbered 2`;

      render(<MarkdownContent content={content} />);

      await waitFor(() => {
        expect(screen.getByText('Item 1')).toBeInTheDocument();
        expect(screen.getByText('Item 2')).toBeInTheDocument();
        expect(screen.getByText('Item 3')).toBeInTheDocument();
        expect(screen.getByText('Numbered 1')).toBeInTheDocument();
        expect(screen.getByText('Numbered 2')).toBeInTheDocument();
      });
    });

    it('renders links with correct attributes', async () => {
      const content = '[Visit Block](https://block.dev)';

      render(<MarkdownContent content={content} />);

      await waitFor(() => {
        const link = screen.getByRole('link', { name: 'Visit Block' });
        expect(link).toBeInTheDocument();
        expect(link).toHaveAttribute('href', 'https://block.dev');
        expect(link).toHaveAttribute('target', '_blank');
        expect(link).toHaveAttribute('rel', 'noopener noreferrer');
      });
    });

    it('renders tables correctly', async () => {
      const content = `| Name | Value |
|------|-------|
| Test | 123   |
| Demo | 456   |`;

      render(<MarkdownContent content={content} />);

      await waitFor(() => {
        expect(screen.getByText('Name')).toBeInTheDocument();
        expect(screen.getByText('Value')).toBeInTheDocument();
        expect(screen.getByText('Test')).toBeInTheDocument();
        expect(screen.getByText('123')).toBeInTheDocument();
        expect(screen.getByText('Demo')).toBeInTheDocument();
        expect(screen.getByText('456')).toBeInTheDocument();
      });
    });
  });

  describe('Error Handling', () => {
    it('handles empty content gracefully', async () => {
      render(<MarkdownContent content="" />);

      // Should not throw and should render the component
      const container = document.querySelector('.w-full.overflow-x-hidden');
      expect(container).toBeInTheDocument();
    });

    it('handles malformed markdown gracefully', async () => {
      const content = `# Unclosed header
[Unclosed link(https://example.com
\`\`\`
Unclosed code block`;

      render(<MarkdownContent content={content} />);

      await waitFor(() => {
        // Should still render what it can
        expect(screen.getByText('Unclosed header')).toBeInTheDocument();
      });
    });
  });
});
