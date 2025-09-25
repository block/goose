import re

# Read the MessageContent file
with open('ui/desktop/src/components/MessageContent.tsx', 'r') as f:
    content = f.read()

print("Updating MessageContent to support mention pills in sent messages...")

# Add MentionPill import
old_imports = """import React, { useMemo } from 'react';
import MarkdownContent from './MarkdownContent';
import ActionPill from './ActionPill';
import { Zap, Code, FileText, Search, Play, Settings } from 'lucide-react';"""

new_imports = """import React, { useMemo } from 'react';
import MarkdownContent from './MarkdownContent';
import ActionPill from './ActionPill';
import MentionPill from './MentionPill';
import { Zap, Code, FileText, Search, Play, Settings } from 'lucide-react';"""

content = content.replace(old_imports, new_imports)

# Update the parsedContent logic to handle both actions and mentions
old_parsing_logic = '''  const parsedContent = useMemo(() => {
    // Find all [Action] patterns and replace them with pill components
    const actionRegex = /\[([^\]]+)\]/g;
    const parts: Array<{ type: 'text' | 'action'; content: string; actionId?: string }> = [];
    let currentIndex = 0;
    let match;

    while ((match = actionRegex.exec(content)) !== null) {
      // Add text before the action
      if (match.index > currentIndex) {
        parts.push({
          type: 'text',
          content: content.slice(currentIndex, match.index),
        });
      }

      // Add the action
      const actionLabel = match[1];
      const actionId = getActionIdFromLabel(actionLabel);
      
      parts.push({
        type: 'action',
        content: actionLabel,
        actionId: actionId,
      });

      currentIndex = match.index + match[0].length;
    }

    // Add remaining text
    if (currentIndex < content.length) {
      parts.push({
        type: 'text',
        content: content.slice(currentIndex),
      });
    }

    // If no actions found, return the original content as a single text part
    if (parts.length === 0) {
      parts.push({
        type: 'text',
        content: content,
      });
    }

    return parts;
  }, [content]);'''

new_parsing_logic = '''  const parsedContent = useMemo(() => {
    // Find all [Action] and @mention patterns and replace them with pill components
    const actionRegex = /\[([^\]]+)\]/g;
    const mentionRegex = /@([^\s]+)/g;
    const parts: Array<{ type: 'text' | 'action' | 'mention'; content: string; actionId?: string; fileName?: string }> = [];
    
    // Find all matches and sort by position
    const allMatches = [];
    
    // Find all action matches
    let actionMatch;
    actionRegex.lastIndex = 0;
    while ((actionMatch = actionRegex.exec(content)) !== null) {
      allMatches.push({
        type: 'action',
        index: actionMatch.index,
        length: actionMatch[0].length,
        content: actionMatch[1]
      });
    }
    
    // Find all mention matches
    let mentionMatch;
    mentionRegex.lastIndex = 0;
    while ((mentionMatch = mentionRegex.exec(content)) !== null) {
      allMatches.push({
        type: 'mention',
        index: mentionMatch.index,
        length: mentionMatch[0].length,
        content: mentionMatch[1] // filename without @
      });
    }
    
    // Sort matches by position
    allMatches.sort((a, b) => a.index - b.index);
    
    let currentIndex = 0;
    
    // Process all matches in order
    for (const match of allMatches) {
      // Add text before this match
      if (match.index > currentIndex) {
        parts.push({
          type: 'text',
          content: content.slice(currentIndex, match.index),
        });
      }
      
      if (match.type === 'action') {
        // Add the action
        const actionLabel = match.content;
        const actionId = getActionIdFromLabel(actionLabel);
        
        parts.push({
          type: 'action',
          content: actionLabel,
          actionId: actionId,
        });
      } else if (match.type === 'mention') {
        // Add the mention
        parts.push({
          type: 'mention',
          content: match.content, // filename without @
          fileName: match.content,
        });
      }
      
      currentIndex = match.index + match.length;
    }
    
    // Add remaining text
    if (currentIndex < content.length) {
      parts.push({
        type: 'text',
        content: content.slice(currentIndex),
      });
    }
    
    // If no matches found, return the original content as a single text part
    if (parts.length === 0) {
      parts.push({
        type: 'text',
        content: content,
      });
    }
    
    return parts;
  }, [content]);'''

content = content.replace(old_parsing_logic, new_parsing_logic)

# Update the rendering logic to handle mention pills
old_rendering = '''      {parsedContent.map((part, index) => {
        if (part.type === 'action' && part.actionId) {
          const actionInfo = getActionInfo(part.actionId);
          return (
            <ActionPill
              key={`action-${index}`}
              actionId={part.actionId}
              label={part.content}
              icon={actionInfo.icon}
              variant="message"
              size="sm"
              // No onRemove for message display - pills are read-only
            />
          );
        } else if (part.content.trim()) {
          return (
            <span
              key={`text-${index}`}
              className={`inline ${className || ''}`}
              dangerouslySetInnerHTML={{
                __html: part.content
                  .replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')
                  .replace(/\*(.*?)\*/g, '<em>$1</em>')
                  .replace(/`(.*?)`/g, '<code>$1</code>')
                  .replace(/\\n/g, '<br>')
              }}
            />
          );
        }
        return null;
      })}'''

new_rendering = '''      {parsedContent.map((part, index) => {
        if (part.type === 'action' && part.actionId) {
          const actionInfo = getActionInfo(part.actionId);
          return (
            <ActionPill
              key={`action-${index}`}
              actionId={part.actionId}
              label={part.content}
              icon={actionInfo.icon}
              variant="message"
              size="sm"
              // No onRemove for message display - pills are read-only
            />
          );
        } else if (part.type === 'mention' && part.fileName) {
          return (
            <MentionPill
              key={`mention-${index}`}
              fileName={part.fileName}
              filePath={`@${part.fileName}`}
              variant="message"
              size="sm"
              // No onRemove for message display - pills are read-only
            />
          );
        } else if (part.content.trim()) {
          return (
            <span
              key={`text-${index}`}
              className={`inline ${className || ''}`}
              dangerouslySetInnerHTML={{
                __html: part.content
                  .replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')
                  .replace(/\*(.*?)\*/g, '<em>$1</em>')
                  .replace(/`(.*?)`/g, '<code>$1</code>')
                  .replace(/\\n/g, '<br>')
              }}
            />
          );
        }
        return null;
      })}'''

content = content.replace(old_rendering, new_rendering)

# Write the modified content back
with open('ui/desktop/src/components/MessageContent.tsx', 'w') as f:
    f.write(content)

print("✅ Updated MessageContent for mention pills:")
print("   - Added MentionPill import")
print("   - Updated parsing logic to detect both [Action] and @mention patterns")
print("   - Added mention pill rendering in sent messages")
print("   - Mentions will appear as green pills in message history")
