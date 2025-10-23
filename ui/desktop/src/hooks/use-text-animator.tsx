import SplitType from 'split-type';
import { useEffect, useRef } from 'react';

interface TextSplitterOptions {
  resizeCallback?: () => void;
  splitTypeTypes?: ('lines' | 'words' | 'chars')[];
}

// Class to split text into lines, words, and characters for animation
export class TextSplitter {
  textElement: HTMLElement;
  onResize: (() => void) | null;
  splitText: SplitType;
  previousContainerWidth: number | null = null;

  constructor(textElement: HTMLElement, options: TextSplitterOptions = {}) {
    if (!textElement || !(textElement instanceof HTMLElement)) {
      throw new Error('Invalid text element provided.');
    }

    const { resizeCallback, splitTypeTypes } = options;
    this.textElement = textElement;
    this.onResize = typeof resizeCallback === 'function' ? resizeCallback : null;

    const splitOptions = splitTypeTypes ? { types: splitTypeTypes } : {};
    this.splitText = new SplitType(this.textElement, splitOptions);

    if (this.onResize) {
      this.initResizeObserver();
    }
  }

  initResizeObserver() {
    // Use a simpler approach to avoid type issues
    const resizeObserver = new ResizeObserver(() => {
      // Just check the current width directly from the element
      if (this.textElement) {
        const currentWidth = Math.floor(this.textElement.getBoundingClientRect().width);

        if (this.previousContainerWidth && this.previousContainerWidth !== currentWidth) {
          this.splitText.split({ types: ['chars'] });
          this.onResize?.();
        }

        this.previousContainerWidth = currentWidth;
      }
    });

    resizeObserver.observe(this.textElement);
  }

  getLines(): HTMLElement[] {
    return this.splitText.lines ?? [];
  }

  getChars(): HTMLElement[] {
    return this.splitText.chars ?? [];
  }
}

// Text animation class for hover effects
const lettersAndSymbols = [
  'a',
  'b',
  'c',
  'd',
  'e',
  'f',
  'g',
  'h',
  'i',
  'j',
  'k',
  'l',
  'm',
  'n',
  'o',
  'p',
  'q',
  'r',
  's',
  't',
  'u',
  'v',
  'w',
  'x',
  'y',
  'z',
  '!',
  '@',
  '#',
  '$',
  '%',
  '^',
  '&',
  '*',
  '-',
  '_',
  '+',
  '=',
  ';',
  ':',
  '<',
  '>',
  ',',
];

export class TextAnimator {
  textElement: HTMLElement;
  splitter!: TextSplitter;
  originalChars!: string[];
  activeAnimations: globalThis.Animation[] = [];
  activeTimeouts: ReturnType<typeof setTimeout>[] = [];

  constructor(textElement: HTMLElement) {
    if (!textElement || !(textElement instanceof HTMLElement)) {
      throw new Error('Invalid text element provided.');
    }

    this.textElement = textElement;
    this.splitText();
  }

  private splitText() {
    this.splitter = new TextSplitter(this.textElement, {
      splitTypeTypes: ['words', 'chars'],
    });
    this.originalChars = this.splitter.getChars().map((char) => char.textContent || '');
  }

  animate() {
    this.reset();

    const chars = this.splitter.getChars();

    chars.forEach((char, position) => {
      const initialText = char.textContent || '';

      char.style.opacity = '1';
      char.style.display = 'inline-block';
      char.style.position = 'relative';

      const animation = char.animate(
        [
          {
            opacity: 1,
            color: '#666',
            fontFamily: 'Cash Sans Mono',
            fontWeight: '300',
          },
          {
            opacity: 0.5,
            color: '#999',
          },
          {
            opacity: 1,
            color: 'inherit',
            fontFamily: 'inherit',
            fontWeight: 'inherit',
          },
        ],
        {
          duration: 300, // Total duration for all iterations
          easing: 'ease-in-out',
          delay: position * 30, // Stagger the start of each animation
          iterations: 1,
        }
      );

      this.activeAnimations.push(animation);

      let iteration = 0;
      const maxIterations = 2;

      const animateCharacterChange = () => {
        if (iteration < maxIterations) {
          char.textContent =
            lettersAndSymbols[Math.floor(Math.random() * lettersAndSymbols.length)];
          const timeoutId = setTimeout(animateCharacterChange, 100);
          this.activeTimeouts.push(timeoutId);
          iteration++;
        } else {
          char.textContent = initialText;
        }
      };

      const timeoutId = setTimeout(animateCharacterChange, position * 30);
      this.activeTimeouts.push(timeoutId);

      animation.onfinish = () => {
        char.textContent = initialText;
        char.style.color = '';
        char.style.fontFamily = '';
        char.style.opacity = '1';
      };
    });
  }

  reset() {
    // Clear all timeouts
    this.activeTimeouts.forEach((timeoutId) => clearTimeout(timeoutId));
    this.activeTimeouts = [];

    // Cancel all animations
    this.activeAnimations.forEach((animation) => animation.cancel());
    this.activeAnimations = [];

    // Reset text content
    const chars = this.splitter.getChars();
    chars.forEach((char, index) => {
      if (this.originalChars[index]) {
        char.textContent = this.originalChars[index];
      }
    });
  }
}

interface UseTextAnimatorProps {
  text: string;
}

export function useTextAnimator({ text }: UseTextAnimatorProps) {
  const elementRef = useRef<HTMLDivElement>(null);
  const animator = useRef<TextAnimator | null>(null);

  useEffect(() => {
    if (!elementRef.current) return;

    // Create animator
    animator.current = new TextAnimator(elementRef.current);

    // Small delay to ensure content is ready
    const timeoutId = setTimeout(() => {
      animator.current?.animate();
    }, 100);

    // Cleanup
    return () => {
      window.clearTimeout(timeoutId);
      if (animator.current) {
        animator.current.reset();
      }
    };
  }, [text]); // Re-run when text changes

  return elementRef;
}
