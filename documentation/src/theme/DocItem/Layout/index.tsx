import React, {type ReactNode, useState} from 'react';
import Layout from '@theme-original/DocItem/Layout';
import type LayoutType from '@theme/DocItem/Layout';
import type {WrapperProps} from '@docusaurus/types';
import {useDoc} from '@docusaurus/plugin-content-docs/client';
import clsx from 'clsx';
import {useWindowSize, ThemeClassNames} from '@docusaurus/theme-common';
import DocItemPaginator from '@theme/DocItem/Paginator';
import DocVersionBanner from '@theme/DocVersionBanner';
import DocVersionBadge from '@theme/DocVersionBadge';
import DocItemFooter from '@theme/DocItem/Footer';
import DocItemTOCMobile from '@theme/DocItem/TOC/Mobile';
import DocItemTOCDesktop from '@theme/DocItem/TOC/Desktop';
import DocBreadcrumbs from '@theme/DocBreadcrumbs';
import ContentVisibility from '@theme/ContentVisibility';
import Heading from '@theme/Heading';
import MDXContent from '@theme/MDXContent';
import styles from './CopyPageButton.module.css';

type Props = WrapperProps<typeof LayoutType>;

// Constants for better maintainability
const COPY_FEEDBACK_DURATION = 2000;

// Component for the Copy Page button
function CopyPageButton(): ReactNode {
  const [copied, setCopied] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  
  const handleCopy = async () => {
    // Check if clipboard API is available
    if (!navigator.clipboard) {
      setError('Clipboard not supported in this browser');
      setTimeout(() => setError(null), COPY_FEEDBACK_DURATION);
      return;
    }

    setIsLoading(true);
    setError(null);
    
    try {
      // For now, just copy a placeholder text
      // In Phase 2, we'll copy the actual markdown content
      const textToCopy = `# Copy Page Feature\n\nThis is a placeholder for the copy page functionality.\nIn Phase 2, this will copy the actual markdown source.`;
      
      await navigator.clipboard.writeText(textToCopy);
      setCopied(true);
      
      // Reset the "Copied" state after timeout
      setTimeout(() => {
        setCopied(false);
      }, COPY_FEEDBACK_DURATION);
    } catch (err) {
      setError('Failed to copy. Please try again.');
      setTimeout(() => setError(null), COPY_FEEDBACK_DURATION);
      console.error('Failed to copy text: ', err);
    } finally {
      setIsLoading(false);
    }
  };

  // Display error message if there's an error
  if (error) {
    return (
      <div className={styles.copyButton} style={{ backgroundColor: 'var(--ifm-color-danger-contrast-background)' }}>
        <span>{error}</span>
      </div>
    );
  }

  return (
    <button
      onClick={handleCopy}
      className={styles.copyButton}
      aria-label={copied ? 'Page copied to clipboard' : 'Copy page to clipboard'}
      type="button"
      disabled={isLoading}
    >
      {/* Copy icon - simple SVG */}
      <svg
        className={styles.copyIcon}
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
        aria-hidden="true"
      >
        <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
        <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2 2v1"></path>
      </svg>
      {isLoading ? 'Copying...' : copied ? 'Copied' : 'Copy page'}
    </button>
  );
}

// Hook to determine if we should show the copy button
function useShouldShowCopyButton(): boolean {
  const {metadata} = useDoc();

  // Show copy button only on actual content pages (not category/index pages)
  // A content page should have a source file (.md file)
  const hasSource = metadata?.source && metadata.source.includes('.md');
  
  // Don't show on generated index pages
  const isNotGeneratedIndex = !metadata?.isGeneratedIndex;
  
  // Don't show on category pages (they typically have /category/ in the permalink)
  const isNotCategoryPage = !metadata?.permalink?.includes('/category/');
  
  return hasSource && isNotGeneratedIndex && isNotCategoryPage;
}

/**
 * Decide if the toc should be rendered, on mobile or desktop viewports
 */
function useDocTOC() {
  const {frontMatter, toc} = useDoc();
  const windowSize = useWindowSize();

  const hidden = frontMatter.hide_table_of_contents;
  const canRender = !hidden && toc.length > 0;

  const mobile = canRender ? <DocItemTOCMobile /> : undefined;

  const desktop =
    canRender && (windowSize === 'desktop' || windowSize === 'ssr') ? (
      <DocItemTOCDesktop />
    ) : undefined;

  return {
    hidden,
    mobile,
    desktop,
  };
}

// Custom Content component that includes the copy button
function CustomDocItemContent({children}: {children: ReactNode}): ReactNode {
  const shouldShowCopyButton = useShouldShowCopyButton();
  const {metadata, frontMatter, contentTitle} = useDoc();
  
  // Check if we should render a synthetic title (same logic as original DocItem/Content)
  const shouldRenderTitle = !frontMatter.hide_title && typeof contentTitle === 'undefined';
  const syntheticTitle = shouldRenderTitle ? metadata.title : null;

  return (
    <div className={clsx(ThemeClassNames.docs.docMarkdown, 'markdown')}>
      {syntheticTitle && (
        <header className={styles.headerWithButton}>
          <Heading as="h1" className={styles.headerTitle}>{syntheticTitle}</Heading>
          {shouldShowCopyButton && <CopyPageButton />}
        </header>
      )}
      {!syntheticTitle && shouldShowCopyButton && (
        <div className={styles.buttonContainer}>
          <CopyPageButton />
        </div>
      )}
      <MDXContent>{children}</MDXContent>
    </div>
  );
}

// Custom Layout component that replicates the original but with our custom content
function CustomDocItemLayout({children}: {children: ReactNode}): ReactNode {
  const docTOC = useDocTOC();
  const {metadata} = useDoc();
  
  return (
    <div className="row">
      <div className={clsx('col', !docTOC.hidden && 'col--9')}>
        <ContentVisibility metadata={metadata} />
        <DocVersionBanner />
        <div className="docItemContainer_Djhp">
          <article>
            <DocBreadcrumbs />
            <DocVersionBadge />
            {docTOC.mobile}
            <CustomDocItemContent>{children}</CustomDocItemContent>
            <DocItemFooter />
          </article>
          <DocItemPaginator />
        </div>
      </div>
      {docTOC.desktop && <div className="col col--3">{docTOC.desktop}</div>}
    </div>
  );
}

export default function LayoutWrapper(props: Props): ReactNode {
  return <CustomDocItemLayout {...props} />;
}
