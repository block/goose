import React, {type ReactNode, useState, useEffect} from 'react';
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
import {Copy, Check} from 'lucide-react';
import styles from './CopyPageButton.module.css';
import TurndownService from 'turndown';

type Props = WrapperProps<typeof LayoutType>;

// Constants for better maintainability
const COPY_FEEDBACK_DURATION = 2000;

// Component for the Copy Page button
function CopyPageButton(): ReactNode {
  const [copied, setCopied] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [isClient, setIsClient] = useState(false);
  const {metadata} = useDoc();
  
  // Ensure we're on the client side to avoid hydration issues
  useEffect(() => {
    setIsClient(true);
  }, []);
  
  const handleCopy = async () => {
    // Ensure we're on client side and clipboard API is available
    if (!isClient || typeof window === 'undefined' || !navigator.clipboard) {
      setError('Clipboard not supported in this browser');
      setTimeout(() => setError(null), COPY_FEEDBACK_DURATION);
      return;
    }

    setIsLoading(true);
    setError(null);
    
    try {
      // Find the article element that contains the main content
      const articleElement = document.querySelector('article');
      
      if (!articleElement) {
        throw new Error('Could not find article content');
      }

      // Clone the article to avoid modifying the actual DOM
      const clonedArticle = articleElement.cloneNode(true) as HTMLElement;
      
      // Remove elements we don't want in the markdown
      const elementsToRemove = [
        '.breadcrumbs',           // Breadcrumb navigation
        '.theme-doc-version-badge', // Version badge
        '.theme-doc-version-banner', // Version banner
        '.pagination-nav',        // Previous/Next navigation
        '.theme-doc-footer',      // Footer
        '.theme-doc-toc-mobile',  // Mobile TOC
        'button',                 // All buttons (including copy buttons)
        '.hash-link',             // Hash links on headings
      ];
      
      elementsToRemove.forEach(selector => {
        clonedArticle.querySelectorAll(selector).forEach(el => el.remove());
      });

      // Initialize Turndown service
      const turndownService = new TurndownService({
        headingStyle: 'atx',      // Use # for headings
        codeBlockStyle: 'fenced',  // Use ``` for code blocks
        bulletListMarker: '-',     // Use - for bullet lists
      });

      // Add custom rule for tabs to convert them to sections
      turndownService.addRule('tabsToSections', {
        filter: function (node) {
          return (
            node.nodeName === 'DIV' &&
            (node as HTMLElement).classList.contains('tabs-container')
          );
        },
        replacement: function (content, node) {
          const tabsContainer = node as HTMLElement;
          let markdown = '\n\n';
          
          // Find all tab buttons to get labels
          const tabButtons = Array.from(tabsContainer.querySelectorAll('[role="tab"]'));
          
          // Find all tab panels
          const tabPanels = Array.from(tabsContainer.querySelectorAll('[role="tabpanel"]'));
          
          // Match panels with buttons by index
          tabPanels.forEach((panel, index) => {
            const panelElement = panel as HTMLElement;
            
            // Get the tab label from the corresponding button (same index)
            const tabLabel = tabButtons[index]?.textContent?.trim() || 'Section';
            
            // Add the tab label as a heading
            markdown += `## ${tabLabel}\n\n`;
            
            // Convert the panel content to markdown
            const panelContent = turndownService.turndown(panelElement.innerHTML);
            markdown += panelContent + '\n\n';
          });
          
          return markdown;
        }
      });

      // Add custom rule for code blocks to preserve language
      turndownService.addRule('fencedCodeBlock', {
        filter: function (node) {
          return (
            node.nodeName === 'PRE' &&
            node.firstChild &&
            node.firstChild.nodeName === 'CODE'
          );
        },
        replacement: function (content, node) {
          const codeElement = node.firstChild as HTMLElement;
          const className = codeElement.className || '';
          const language = className.match(/language-(\w+)/)?.[1] || '';
          
          // Get the actual code content
          const code = codeElement.textContent || '';
          
          return '\n\n```' + language + '\n' + code + '\n```\n\n';
        }
      });

      // Convert HTML to markdown
      let markdown = turndownService.turndown(clonedArticle);
      
      // Clean up the markdown
      markdown = markdown
        .replace(/\n{3,}/g, '\n\n')  // Remove excessive newlines
        .trim();                      // Remove leading/trailing whitespace
      
      // Copy to clipboard
      await navigator.clipboard.writeText(markdown);
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

  // Don't render anything until we're on the client side
  if (!isClient) {
    return null;
  }

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
      {/* Copy/Check icon using Lucide React */}
      {copied ? (
        <Check 
          className={styles.copyIcon}
          size={16}
          aria-hidden="true"
        />
      ) : (
        <Copy 
          className={styles.copyIcon}
          size={16}
          aria-hidden="true"
        />
      )}
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
