import React, { useRef, useEffect, useState, useCallback, forwardRef, useImperativeHandle } from 'react';
import SpellCheckTooltip from './SpellCheckTooltip';
import { checkSpelling, MisspelledWord } from '../utils/realSpellCheck';
import { ActionPill } from './ActionPill';
import MentionPill from './MentionPill';
import { Zap, Code, FileText, Search, Play, Settings } from 'lucide-react';

interface RichChatInputProps {
  value: string;
  onChange: (value: string, cursorPos?: number) => void;
  onKeyDown?: (e: React.KeyboardEvent<HTMLDivElement>) => void;
  onPaste?: (e: React.ClipboardEvent<HTMLDivElement>) => void;
  onFocus?: () => void;
  onBlur?: () => void;
  onCompositionStart?: () => void;
  onCompositionEnd?: () => void;
  placeholder?: string;
  disabled?: boolean;
  className?: string;
  style?: React.CSSProperties;
  autoFocus?: boolean;
  'data-testid'?: string;
  rows?: number;
}

// Action mapping for pill display
const ACTION_MAP = {
  'quick-task': { label: 'Quick Task', icon: <Zap size={12} /> },
  'generate-code': { label: 'Generate Code', icon: <Code size={12} /> },
  'create-document': { label: 'Create Document', icon: <FileText size={12} /> },
  'search-files': { label: 'Search Files', icon: <Search size={12} /> },
  'run-command': { label: 'Run Command', icon: <Play size={12} /> },
  'settings': { label: 'Settings', icon: <Settings size={12} /> },
};

export interface RichChatInputRef {
  focus: () => void;
  blur: () => void;
  setSelectionRange: (start: number, end: number) => void;
  getBoundingClientRect: () => DOMRect;
}

// Simple spell checking function using browser's built-in capabilities
const checkSpelling = async (text: string): Promise<{ word: string; start: number; end: number }[]> => {
  // This is a basic implementation - in a real app you might want to use a more sophisticated spell checker
  const misspelledWords: { word: string; start: number; end: number }[] = [];
  
  // Common misspelled words for demo purposes
  const commonMisspellings = [
    // Test words
    'sdd', 'asdf', 'qwerty', 'test', 'xyz',
    // Common misspellings
    'teh', 'recieve', 'seperate', 'occured', 'neccessary', 'definately', 
    'occassion', 'begining', 'tommorrow', 'accomodate', 'existance', 'maintainance',
    'alot', 'wierd', 'freind', 'thier', 'calender', 'enviroment', 'goverment',
    'independant', 'jewelery', 'liesure', 'mispell', 'noticable', 'occassionally',
    'perseverence', 'priviledge', 'recomend', 'rythm', 'sucessful', 'truely',
    'untill', 'vaccuum', 'wether', 'wich', 'writting', 'youre', 'its'
  ];
  
  // Split text into words while preserving positions
  const words = text.split(/(\s+|[^\w\s])/);
  let currentPos = 0;
  
  for (const word of words) {
    const cleanWord = word.toLowerCase().replace(/[^\w]/g, '');
    
    if (cleanWord && commonMisspellings.includes(cleanWord)) {
      const start = text.indexOf(word, currentPos);
      if (start !== -1) {
        misspelledWords.push({
          word: word,
          start: start,
          end: start + word.length
        });
      }
    }
    
    currentPos += word.length;
  }
  
  return misspelledWords;
};

export const RichChatInput = forwardRef<RichChatInputRef, RichChatInputProps>(({
  value,
  onChange,
  onKeyDown,
  onPaste,
  onFocus,
  onBlur,
  onCompositionStart,
  onCompositionEnd,
  placeholder,
  disabled,
  className,
  style,
  autoFocus,
  'data-testid': testId,
  rows = 1,
}, ref) => {
  const hiddenTextareaRef = useRef<HTMLTextAreaElement>(null);
  const displayRef = useRef<HTMLDivElement>(null);
  const [isFocused, setIsFocused] = useState(false);
  const [cursorPosition, setCursorPosition] = useState(0);
  const [misspelledWords, setMisspelledWords] = useState<MisspelledWord[]>([]);
  
  // Spell check tooltip state
  const [tooltip, setTooltip] = useState<{
    isVisible: boolean;
    position: { x: number; y: number };
    misspelledWord: string;
    suggestions: string[];
    wordStart: number;
    wordEnd: number;
  }>({
    isVisible: false,
    position: { x: 0, y: 0 },
    misspelledWord: '',
    suggestions: [],
    wordStart: 0,
    wordEnd: 0,
  });

  // Expose methods to parent component
  useImperativeHandle(ref, () => ({
    focus: () => hiddenTextareaRef.current?.focus(),
    blur: () => hiddenTextareaRef.current?.blur(),
    setSelectionRange: (start: number, end: number) => {
      hiddenTextareaRef.current?.setSelectionRange(start, end);
      setCursorPosition(start);
    },
    getBoundingClientRect: () => {
      return displayRef.current?.getBoundingClientRect() || new DOMRect();
    },
  }), []);

  // Update cursor position when selection changes
  const updateCursorPosition = useCallback(() => {
    if (hiddenTextareaRef.current) {
      setCursorPosition(hiddenTextareaRef.current.selectionStart);
    }
  }, []);

  // Spell check the content - INLINE VERSION
  const performSpellCheck = useCallback(async (text: string) => {
    console.log('🔍 INLINE SPELL CHECK: Starting check for text:', text);
    
    // Smart inline spell checker that catches obvious misspellings
    const misspelledWords: MisspelledWord[] = [];
    
    // Known misspellings
    const knownMisspellings = ['hellwo', 'gasdd2', 'recieve', 'seperate', 'definately', 'teh', 'wierd'];
    
    // Common correct words to skip
    const commonWords = [
      'the', 'and', 'for', 'are', 'but', 'not', 'you', 'all', 'can', 'had', 'her', 'was', 'one', 'our', 'out', 'day', 'get', 'has', 'him', 'his', 'how', 'man', 'new', 'now', 'old', 'see', 'two', 'way', 'who', 'boy', 'did', 'its', 'let', 'put', 'say', 'she', 'too', 'use',
      'hello', 'world', 'test', 'chat', 'message', 'text', 'word', 'good', 'great', 'nice', 'cool', 'awesome', 'amazing', 'perfect', 'excellent', 'wonderful', 'fantastic', 'brilliant', 'super', 'best', 'love', 'like', 'want', 'need', 'help', 'please', 'thank', 'thanks', 'welcome'
    ];
    
    console.log('🔍 SMART SPELL CHECK: Known misspellings:', knownMisspellings);
    
    // Simple word detection
    const words = text.split(/\s+/);
    for (let i = 0; i < words.length; i++) {
      const word = words[i];
      const cleanWord = word.toLowerCase().replace(/[^a-z]/g, '');
      console.log('🔍 SMART SPELL CHECK: Checking word:', word, 'cleaned:', cleanWord);
      
      // Skip very short words
      if (cleanWord.length < 3) {
        console.log('🔍 SMART SPELL CHECK: Skipping short word:', cleanWord);
        continue;
      }
      
      // Skip common correct words
      if (commonWords.includes(cleanWord)) {
        console.log('🔍 SMART SPELL CHECK: Skipping common word:', cleanWord);
        continue;
      }
      
      let isMisspelled = false;
      let suggestions: string[] = [];
      
      // Check known misspellings first
      if (knownMisspellings.includes(cleanWord)) {
        isMisspelled = true;
        // Generate specific suggestions
        if (cleanWord === 'hellwo') suggestions = ['hello', 'hollow', 'ellow'];
        else if (cleanWord === 'recieve') suggestions = ['receive', 'receiver', 'received'];
        else if (cleanWord === 'seperate') suggestions = ['separate', 'desperate', 'temperate'];
        else if (cleanWord === 'definately') suggestions = ['definitely', 'defiantly', 'definite'];
        else if (cleanWord === 'teh') suggestions = ['the', 'tea', 'ten'];
        else if (cleanWord === 'wierd') suggestions = ['weird', 'wired', 'wide'];
        else suggestions = ['suggestion1', 'suggestion2', 'suggestion3'];
      }
      // Check for obviously misspelled patterns
      else if (
        // Very long words with random letters (likely gibberish)
        (cleanWord.length > 8 && /[aeiou]{3,}/.test(cleanWord)) ||
        (cleanWord.length > 10 && /[bcdfghjklmnpqrstvwxyz]{4,}/.test(cleanWord)) ||
        // Words with repeated random patterns
        /(..)\1{2,}/.test(cleanWord) ||
        // Words ending in random consonant clusters
        /[bcdfghjklmnpqrstvwxyz]{3,}$/.test(cleanWord) ||
        // Words starting with random consonant clusters
        /^[bcdfghjklmnpqrstvwxyz]{4,}/.test(cleanWord) ||
        // Random letter combinations that look like keyboard mashing
        /[qwerty]{3,}/.test(cleanWord) ||
        /[asdf]{3,}/.test(cleanWord) ||
        /[zxcv]{3,}/.test(cleanWord) ||
        // Very long words (likely gibberish)
        cleanWord.length > 15
      ) {
        isMisspelled = true;
        suggestions = ['Did you mean to type something else?'];
        console.log('🔍 SMART SPELL CHECK: Detected gibberish pattern:', cleanWord);
      }
      
      if (isMisspelled) {
        const start = text.indexOf(word);
        misspelledWords.push({
          word: word,
          start: start,
          end: start + word.length,
          suggestions: suggestions
        });
        console.log('🔍 SMART SPELL CHECK: ✅ Found misspelling!', word);
      } else {
        console.log('🔍 SMART SPELL CHECK: ❌ Word looks correct:', cleanWord);
      }
    }
    
    console.log('🔍 INLINE SPELL CHECK: Final result:', misspelledWords);
    setMisspelledWords(misspelledWords);
  }, []);

  // Debounced spell check
  useEffect(() => {
    const timeoutId = setTimeout(() => {
      if (value.trim()) {
        performSpellCheck(value);
      } else {
        setMisspelledWords([]);
      }
    }, 500); // Debounce spell check by 500ms

    return () => clearTimeout(timeoutId);
  }, [value, performSpellCheck]);

  // Parse and render content with action pills, mention pills, spell checking, and cursor
  const renderContent = useCallback(() => {
    // Show placeholder when there's no text content (regardless of focus state)
    if (!value.trim()) {
      return (
        <div className="relative min-h-[1.5em] flex items-center">
          <span className="text-text-muted pointer-events-none select-none">
            {placeholder}
          </span>
          {isFocused && (
            <span 
              className="border-l-2 border-text-default inline-block align-baseline absolute left-0" 
              style={{ 
                animation: "blink 1s step-end infinite", 
                height: "1em", 
                verticalAlign: "baseline",
                top: "50%",
                transform: "translateY(-50%)"
              }} 
            />
          )}
        </div>
      );
    }

    const parts: React.ReactNode[] = [];
    const actionRegex = /\[([^\]]+)\]/g;
    const mentionRegex = /@([^\s]+)/g;
    let lastIndex = 0;
    let keyCounter = 0;
    let currentPos = 0;

    console.log('🎨 RichChatInput renderContent called with value:', value);
    console.log('🔍 Looking for action and mention patterns with regex:', { actionRegex, mentionRegex });
    console.log('📝 Misspelled words:', misspelledWords);
    
    // Find all actions, mentions, and misspelled words, then sort by position
    const allMatches = [];
    
    // Find all action matches
    let actionMatch;
    actionRegex.lastIndex = 0;
    while ((actionMatch = actionRegex.exec(value)) !== null) {
      allMatches.push({
        type: 'action',
        match: actionMatch,
        index: actionMatch.index,
        length: actionMatch[0].length,
        content: actionMatch[1]
      });
    }
    
    // Find all mention matches
    let mentionMatch;
    mentionRegex.lastIndex = 0;
    console.log('🔍 Searching for mentions in value:', value);
    while ((mentionMatch = mentionRegex.exec(value)) !== null) {
      console.log('📁 Found mention match:', mentionMatch);
      allMatches.push({
        type: 'mention',
        match: mentionMatch,
        index: mentionMatch.index,
        length: mentionMatch[0].length,
        content: mentionMatch[1]
      });
    }

    // Add misspelled words
    misspelledWords.forEach(misspelled => {
      allMatches.push({
        type: 'misspelled',
        match: null,
        index: misspelled.start,
        length: misspelled.end - misspelled.start,
        content: misspelled.word
      });
    });
    
    // Sort matches by position
    allMatches.sort((a, b) => a.index - b.index);
    
    console.log('🔍 All matches found:', allMatches);
    
    // Process all matches in order, handling overlaps
    const processedMatches = [];
    let lastProcessedEnd = 0;
    
    for (const matchData of allMatches) {
      // Skip overlapping matches (pills take priority over spell check)
      if (matchData.index < lastProcessedEnd) {
        continue;
      }
      
      processedMatches.push(matchData);
      lastProcessedEnd = matchData.index + matchData.length;
    }
    
    // Render content with processed matches
    currentPos = 0;
    lastIndex = 0;
    
    for (const matchData of processedMatches) {
      const { type, index, length, content } = matchData;
      
      // Add text before this match with potential cursor
      const beforeMatch = value.slice(lastIndex, index);
      if (beforeMatch) {
        let textWithCursor = [];
        for (let i = 0; i < beforeMatch.length; i++) {
          if (isFocused && cursorPosition === currentPos) {
            textWithCursor.push(
              <span key={`cursor-${keyCounter++}`} className="border-l-2 border-text-default inline-block align-baseline" style={{ animation: "blink 1s step-end infinite", height: "1em", marginLeft: "1px", verticalAlign: "baseline" }} />
            );
          }
          textWithCursor.push(beforeMatch[i]);
          currentPos++;
        }
        
        parts.push(
          <span key={`text-${keyCounter++}`} className="inline whitespace-pre-wrap">
            {textWithCursor}
          </span>
        );
      }
      
      // Add cursor before match if needed
      if (isFocused && cursorPosition === currentPos) {
        parts.push(
          <span key={`cursor-${keyCounter++}`} className="border-l-2 border-text-default inline-block align-baseline" style={{ animation: "blink 1s step-end infinite", height: "1em", marginLeft: "1px", verticalAlign: "baseline" }} />
        );
      }
      
      console.log('🎨 PROCESSING MATCH: type:', type, 'content:', content, 'index:', index);
      if (type === 'action') {
        // Handle action pills
        const actionLabel = content;
        const actionEntry = Object.entries(ACTION_MAP).find(
          ([_, config]) => config.label === actionLabel
        );
        
        console.log('🏷️ Creating action pill:', { actionLabel, actionEntry });
        
        if (actionEntry) {
          const [actionId, config] = actionEntry;
          parts.push(
            <ActionPill
              key={`action-${keyCounter++}`}
              actionId={actionId}
              label={config.label}
              icon={config.icon}
              variant="default"
              size="sm"
              onRemove={() => handleRemoveAction(actionLabel)}
            />
          );
        } else {
          // If no matching action, render as text
          parts.push(
            <span key={`text-${keyCounter++}`} className="inline whitespace-pre-wrap">
              {value.slice(index, index + length)}
            </span>
          );
        }
      } else if (type === 'mention') {
        // Handle mention pills
        const fileName = content;
        const filePath = `@${fileName}`;
        
        console.log('📁 Creating mention pill:', { fileName, filePath });
        
        parts.push(
          <MentionPill
            key={`mention-${keyCounter++}`}
            fileName={fileName}
            filePath={filePath}
            variant="default"
            size="sm"
            onRemove={() => handleRemoveMention(fileName)}
          />
        );
      } else if (type === 'misspelled') {
        // Handle misspelled words with red highlighting and hover tooltip
        const misspelledData = misspelledWords.find(m => m.word === content);
        console.log('🎨 RENDERING MISSPELLED: word:', content, 'data:', misspelledData);
        console.log('🎨 RENDERING MISSPELLED: all misspelled words:', misspelledWords);
        
        parts.push(
          <span 
            key={`misspelled-${keyCounter++}`} 
            className="inline whitespace-pre-wrap cursor-pointer"
            style={{
              // ULTRA visible styling - bright red background
              backgroundColor: '#fecaca', // Light red background
              color: '#dc2626', // Red text
              fontWeight: 'bold',
              padding: '4px 6px',
              borderRadius: '4px',
              border: '2px solid #dc2626',
              boxShadow: '0 2px 4px rgba(220, 38, 38, 0.5)',
              textDecoration: 'underline wavy #dc2626',
              pointerEvents: 'auto', // Override parent's pointer-events: none
              userSelect: 'none', // Prevent text selection interference
              position: 'relative', // Ensure it's above other elements
              zIndex: 10 // Higher z-index than parent
            }}
            title={`Hover for suggestions: ${content}`}
            onClick={(e) => {
              console.log('🖱️ CLICK: Clicked on misspelled word:', content);
              console.log('🖱️ CLICK: Event target:', e.target);
              console.log('🖱️ CLICK: Current target:', e.currentTarget);
            }}
            onMouseOver={(e) => {
              console.log('🖱️ MOUSEOVER: Mouse over misspelled word:', content);
            }}
            onMouseEnter={(e) => {
              console.log('🖱️ MOUSEENTER: Mouse entered misspelled word:', content);
              console.log('🖱️ MOUSEENTER: Event target:', e.target);
              console.log('🖱️ MOUSEENTER: Current target:', e.currentTarget);
              console.log('🖱️ MOUSEENTER: Misspelled data:', misspelledData);
              if (misspelledData) {
                const rect = e.currentTarget.getBoundingClientRect();
                console.log('🖱️ MOUSEENTER: Element rect:', rect);
                const tooltipData = {
                  isVisible: true,
                  position: { 
                    x: rect.left + rect.width / 2, 
                    y: rect.top 
                  },
                  misspelledWord: misspelledData.word,
                  suggestions: misspelledData.suggestions || [],
                  wordStart: misspelledData.start,
                  wordEnd: misspelledData.end,
                };
                console.log('🖱️ MOUSEENTER: Setting tooltip data:', tooltipData);
                setTooltip(tooltipData);
              } else {
                console.log('🖱️ MOUSEENTER: No misspelled data found for word:', content);
              }
            }}
            onMouseLeave={(e) => {
              console.log('🖱️ MOUSELEAVE: Mouse left misspelled word:', content);
              // Add a small delay before hiding to allow clicking on tooltip
              setTimeout(() => {
                setTooltip(prev => ({ ...prev, isVisible: false }));
              }, 150);
            }}
          >
            {content}
          </span>
        );
      }
      
      currentPos += length;
      lastIndex = index + length;
    }
    
    // Add remaining text with potential cursor
    const remainingText = value.slice(lastIndex);
    if (remainingText) {
      let textWithCursor = [];
      for (let i = 0; i < remainingText.length; i++) {
        if (isFocused && cursorPosition === currentPos) {
          textWithCursor.push(
            <span key={`cursor-${keyCounter++}`} className="border-l-2 border-text-default inline-block align-baseline" style={{ animation: "blink 1s step-end infinite", height: "1em", marginLeft: "1px", verticalAlign: "baseline" }} />
          );
        }
        textWithCursor.push(remainingText[i]);
        currentPos++;
      }
      
      parts.push(
        <span key={`text-${keyCounter++}`} className="inline whitespace-pre-wrap">
          {textWithCursor}
        </span>
      );
    }
    
    // Add cursor at the end if needed
    if (isFocused && cursorPosition === currentPos) {
      parts.push(
        <span key={`cursor-${keyCounter++}`} className="border-l-2 border-text-default inline-block align-baseline" style={{ animation: "blink 1s step-end infinite", height: "1em", marginLeft: "1px", verticalAlign: "baseline" }} />
      );
    }
    
    return (
      <div className="whitespace-pre-wrap min-h-[1.5em] leading-relaxed">
        {parts.length > 0 ? parts : (
          isFocused && (
            <span className="border-l-2 border-text-default inline-block align-baseline" style={{ animation: "blink 1s step-end infinite", height: "1em", marginLeft: "1px", verticalAlign: "baseline" }} />
          )
        )}
      </div>
    );
  }, [value, isFocused, placeholder, cursorPosition, misspelledWords]);

  const handleRemoveAction = useCallback((actionLabel: string) => {
    const actionText = `[${actionLabel}]`;
    const newValue = value.replace(actionText, '');
    onChange(newValue);
  }, [value, onChange]);

  const handleRemoveMention = useCallback((fileName: string) => {
    const mentionText = `@${fileName}`;
    const newValue = value.replace(mentionText, '');
    onChange(newValue);
  }, [value, onChange]);

  const handleTextareaChange = useCallback((e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const newValue = e.target.value;
    const newCursorPos = e.target.selectionStart;
    
    console.log('🔄 RichChatInput: onChange', { newValue, newCursorPos });
    onChange(newValue, newCursorPos);
    setCursorPosition(newCursorPos);
  }, [onChange]);

  const handleTextareaKeyDown = useCallback((e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    // Update cursor position on key events
    setTimeout(updateCursorPosition, 0);
    
    // Handle backspace on action and mention pills
    if (e.key === 'Backspace') {
      const cursorPos = e.currentTarget.selectionStart;
      const beforeCursor = value.slice(0, cursorPos);
      
      console.log('🔙 Backspace pressed, cursor at:', cursorPos);
      console.log('🔙 Text before cursor:', beforeCursor);
      
      // Check if cursor is right after an action pill [Action]
      const actionMatch = beforeCursor.match(/\[([^\]]+)\]$/);
      if (actionMatch) {
        console.log('🔙 Found action pill to remove:', actionMatch[1]);
        e.preventDefault();
        handleRemoveAction(actionMatch[1]);
        return;
      }
      
      // Check if cursor is right after a mention pill @filename
      const mentionMatch = beforeCursor.match(/@([^\s]+)$/);
      if (mentionMatch) {
        console.log('🔙 Found mention pill to remove:', mentionMatch[1]);
        e.preventDefault();
        handleRemoveMention(mentionMatch[1]);
        return;
      }
    }
    
    // Create a proper synthetic event that maintains all the original event properties
    const syntheticEvent = {
      ...e,
      key: e.key,
      shiftKey: e.shiftKey,
      altKey: e.altKey,
      ctrlKey: e.ctrlKey,
      metaKey: e.metaKey,
      preventDefault: () => e.preventDefault(),
      stopPropagation: () => e.stopPropagation(),
      currentTarget: {
        ...e.currentTarget,
        value: e.currentTarget.value,
        selectionStart: e.currentTarget.selectionStart,
        selectionEnd: e.currentTarget.selectionEnd,
        getBoundingClientRect: () => displayRef.current?.getBoundingClientRect() || new DOMRect(),
      },
      target: {
        ...e.currentTarget,
        value: e.currentTarget.value,
        selectionStart: e.currentTarget.selectionStart,
        selectionEnd: e.currentTarget.selectionEnd,
        getBoundingClientRect: () => displayRef.current?.getBoundingClientRect() || new DOMRect(),
      },
    } as any;
    
    onKeyDown?.(syntheticEvent);
  }, [value, handleRemoveAction, onKeyDown, updateCursorPosition]);

  const handleTextareaPaste = useCallback((e: React.ClipboardEvent<HTMLTextAreaElement>) => {
    // Update cursor position after paste
    setTimeout(updateCursorPosition, 0);
    
    // Create proper synthetic event
    const syntheticEvent = {
      ...e,
      preventDefault: () => e.preventDefault(),
      stopPropagation: () => e.stopPropagation(),
      clipboardData: e.clipboardData,
      currentTarget: displayRef.current,
      target: displayRef.current,
    } as any;
    
    onPaste?.(syntheticEvent);
  }, [onPaste, updateCursorPosition]);

  const handleTextareaFocus = useCallback(() => {
    setIsFocused(true);
    updateCursorPosition();
    onFocus?.();
  }, [onFocus, updateCursorPosition]);

  const handleTextareaBlur = useCallback(() => {
    setIsFocused(false);
    onBlur?.();
  }, [onBlur]);

  // Handle selection changes (cursor movement)
  const handleSelectionChange = useCallback(() => {
    if (document.activeElement === hiddenTextareaRef.current) {
      updateCursorPosition();
    }
  }, [updateCursorPosition]);

  // Auto-focus effect
  useEffect(() => {
    if (autoFocus && hiddenTextareaRef.current) {
      hiddenTextareaRef.current.focus();
    }
  }, [autoFocus]);

  // Listen for selection changes to update cursor position
  useEffect(() => {
    document.addEventListener('selectionchange', handleSelectionChange);
    return () => {
      document.removeEventListener('selectionchange', handleSelectionChange);
    };
  }, [handleSelectionChange]);

  // Tooltip handlers
  const handleSuggestionSelect = useCallback((suggestion: string) => {
    const newValue = value.slice(0, tooltip.wordStart) + 
                     suggestion + 
                     value.slice(tooltip.wordEnd);
    onChange(newValue);
    setTooltip(prev => ({ ...prev, isVisible: false }));
  }, [value, onChange, tooltip.wordStart, tooltip.wordEnd]);

  const handleAddToDictionary = useCallback(() => {
    // TODO: Implement add to dictionary functionality
    console.log('Add to dictionary:', tooltip.misspelledWord);
    setTooltip(prev => ({ ...prev, isVisible: false }));
  }, [tooltip.misspelledWord]);

  const handleIgnore = useCallback(() => {
    // TODO: Implement ignore functionality
    console.log('Ignore word:', tooltip.misspelledWord);
    setTooltip(prev => ({ ...prev, isVisible: false }));
  }, [tooltip.misspelledWord]);

  return (
    <div className="relative rich-text-input">
      {/* Hidden textarea for actual input handling with spell check enabled */}
      <textarea
        ref={hiddenTextareaRef}
        value={value}
        onChange={handleTextareaChange}
        onKeyDown={handleTextareaKeyDown}
        onPaste={handleTextareaPaste}
        onFocus={handleTextareaFocus}
        onBlur={handleTextareaBlur}
        onCompositionStart={onCompositionStart}
        onCompositionEnd={onCompositionEnd}
        disabled={disabled}
        data-testid={testId}
        spellCheck={false} // Disable browser spell check - we handle it ourselves
        className="absolute inset-0 w-full h-full resize-none selection:bg-blue-500 selection:text-white"
        onMouseMove={(e) => {
          // Detect if mouse is over a misspelled word
          const rect = e.currentTarget.getBoundingClientRect();
          const x = e.clientX - rect.left;
          const y = e.clientY - rect.top;
          
          // Get character position from mouse coordinates
          const textarea = e.currentTarget;
          const range = document.caretRangeFromPoint(e.clientX, e.clientY);
          
          if (range && range.startContainer.nodeType === Node.TEXT_NODE) {
            const textContent = textarea.value;
            const charIndex = range.startOffset;
            
            console.log('🖱️ TEXTAREA HOVER: Mouse at char index:', charIndex, 'char:', textContent[charIndex]);
            
            // Check if this character position is within a misspelled word
            const misspelledWord = misspelledWords.find(word => 
              charIndex >= word.start && charIndex < word.end
            );
            
            if (misspelledWord) {
              console.log('🖱️ TEXTAREA HOVER: Found misspelled word at mouse position:', misspelledWord);
              setTooltip({
                isVisible: true,
                position: { x: e.clientX, y: e.clientY - 50 },
                misspelledWord: misspelledWord.word,
                suggestions: misspelledWord.suggestions || [],
                wordStart: misspelledWord.start,
                wordEnd: misspelledWord.end,
              });
            } else {
              setTooltip(prev => ({ ...prev, isVisible: false }));
            }
          }
        }}
        style={{
          position: 'absolute',
          left: 0,
          top: 0,
          width: '100%',
          height: '100%',
          opacity: 0.15, // Slightly more visible for selection
          zIndex: 2, // Higher z-index to capture mouse events
          background: 'transparent',
          border: 'none',
          outline: 'none',
          resize: 'none',
          color: 'rgba(59, 130, 246, 0.2)', // Slightly more visible blue text
          caretColor: 'transparent', // Hide caret (we show our own)
          pointerEvents: 'auto', // Ensure it can receive mouse events
          fontFamily: 'Cash Sans, sans-serif', // Match exact font
          fontSize: '0.875rem', // Match text-sm (14px)
          lineHeight: '1.5', // Match leading-relaxed
          padding: '12px 80px 6px 12px', // Match original: px-3 pt-3 pb-1.5 pr-20
          margin: '0',
          boxSizing: 'border-box',
          whiteSpace: 'pre-wrap', // Match visual display
          wordWrap: 'break-word',
        }}
        rows={rows}
      />
      
      {/* Visual display with action pills, mention pills, spell check, and cursor */}
      <div
        ref={displayRef}
        className={`${className} cursor-text relative`}
        style={{
          ...style,
          minHeight: `${rows * 1.5}em`,
          zIndex: 1, // Lower z-index, behind textarea
          pointerEvents: 'none', // Don't interfere with text selection
          userSelect: 'none', // Prevent selection on visual layer
          WebkitUserSelect: 'none',
          fontFamily: 'Cash Sans, sans-serif', // Match textarea font
          fontSize: '0.875rem', // Match textarea size
          lineHeight: '1.5', // Match textarea line height
          padding: '12px 80px 6px 12px', // Match original: px-3 pt-3 pb-1.5 pr-20
          margin: '0',
          whiteSpace: 'pre-wrap', // Match textarea
          wordWrap: 'break-word',
        }}
        role="textbox"
        aria-multiline="true"
        aria-placeholder={placeholder}
      >
        {renderContent()}
      </div>
      
      {/* Spell Check Hover Tooltip */}
      {console.log('🖱️ TOOLTIP RENDER: tooltip state:', tooltip)}
      <SpellCheckTooltip
        isVisible={tooltip.isVisible}
        position={tooltip.position}
        suggestions={tooltip.suggestions}
        misspelledWord={tooltip.misspelledWord}
        onSuggestionSelect={handleSuggestionSelect}
        onAddToDictionary={handleAddToDictionary}
        onIgnore={handleIgnore}
      />
    </div>
  );
});

RichChatInput.displayName = 'RichChatInput';

export default RichChatInput;
