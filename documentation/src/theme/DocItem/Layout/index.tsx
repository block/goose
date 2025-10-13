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

type Props = WrapperProps<typeof LayoutType>;

// Component for the Copy Page button
function CopyPageButton(): ReactNode {
  const [copied, setCopied] = useState(false);
  
  const handleCopy = async () => {
    try {
      // For now, just copy a placeholder text
      // In Phase 2, we'll copy the actual markdown content
      const textToCopy = `# Copy Page Feature\n\nThis is a placeholder for the copy page functionality.\nIn Phase 2, this will copy the actual markdown source.`;
      
      await navigator.clipboard.writeText(textToCopy);
      setCopied(true);
      
      // Reset the "Copied" state after 2 seconds
      setTimeout(() => {
        setCopied(false);
      }, 2000);
    } catch (err) {
      console.error('Failed to copy text: ', err);
    }
  };

  return (
    <button
      onClick={handleCopy}
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: '6px',
        padding: '6px 12px',
        backgroundColor: 'var(--ifm-color-emphasis-200)',
        border: '1px solid var(--ifm-color-emphasis-300)',
        borderRadius: '6px',
        color: 'var(--ifm-color-content)',
        fontSize: '14px',
        cursor: 'pointer',
        transition: 'all 0.2s ease',
        fontFamily: 'var(--ifm-font-family-base)',
      }}
      onMouseEnter={(e) => {
        e.currentTarget.style.backgroundColor = 'var(--ifm-color-emphasis-300)';
      }}
      onMouseLeave={(e) => {
        e.currentTarget.style.backgroundColor = 'var(--ifm-color-emphasis-200)';
      }}
    >
      {/* Copy icon - simple SVG */}
      <svg
        width="16"
        height="16"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
        <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
      </svg>
      {copied ? 'Copied' : 'Copy page'}
    </button>
  );
}

// Hook to determine if we should show the copy button
function useShouldShowCopyButton(): boolean {
  const {metadata} = useDoc();
  
  // Debug: Log metadata to understand the structure
  React.useEffect(() => {
    console.log('DocItem Metadata for Copy Button:', {
      source: metadata?.source,
      permalink: metadata?.permalink,
      isGeneratedIndex: metadata?.isGeneratedIndex,
      type: metadata?.type,
      frontMatter: metadata?.frontMatter,
    });
  }, [metadata]);

  // Show copy button only on actual content pages (not category/index pages)
  // A content page should have a source file (.md file)
  const hasSource = metadata?.source && metadata.source.includes('.md');
  
  // Don't show on generated index pages
  const isNotGeneratedIndex = !metadata?.isGeneratedIndex;
  
  // Don't show on category pages (they typically have /category/ in the permalink)
  const isNotCategoryPage = !metadata?.permalink?.includes('/category/');
  
  const shouldShow = hasSource && isNotGeneratedIndex && isNotCategoryPage;
  
  console.log('Copy button decision:', {
    hasSource,
    isNotGeneratedIndex,
    isNotCategoryPage,
    shouldShow,
    permalink: metadata?.permalink
  });
  
  return shouldShow;
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
        <header style={{ 
          display: 'flex', 
          justifyContent: 'space-between', 
          alignItems: 'flex-start',
          marginBottom: '1rem'
        }}>
          <Heading as="h1" style={{ margin: 0, flex: 1 }}>{syntheticTitle}</Heading>
          {shouldShowCopyButton && <CopyPageButton />}
        </header>
      )}
      {!syntheticTitle && shouldShowCopyButton && (
        <div style={{
          display: 'flex',
          justifyContent: 'flex-end',
          marginBottom: '1rem',
        }}>
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
