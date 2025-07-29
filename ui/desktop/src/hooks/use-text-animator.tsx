import { gsap } from 'gsap';
import SplitType from 'split-type';
import { useEffect, useRef } from 'react';

// Utility debounce function
export const debounce = <T extends (...args: unknown[]) => void>(func: T, delay: number): T => {
  let timerId: ReturnType<typeof setTimeout>;
  return ((...args: unknown[]) => {
    window.clearTimeout(timerId);
    timerId = setTimeout(() => {
      func(...args);
    }, delay);
  }) as T;
};

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

  revert() {
    return this.splitText.revert();
  }

  getLines(): HTMLElement[] {
    return this.splitText.lines ?? [];
  }

  getWords(): HTMLElement[] {
    return this.splitText.words ?? [];
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
    this.originalChars = this.splitter.getChars().map((char) => char.innerHTML);
  }

  animate() {
    this.reset();

    const chars = this.splitter.getChars();

    chars.forEach((char, position) => {
      const initialHTML = char.innerHTML;
      let repeatCount = 0;

      // Set initial state
      gsap.set(char, {
        opacity: 1,
        display: 'inline-block',
        position: 'relative',
      });

      gsap.fromTo(
        char,
        {
          opacity: 1,
        },
        {
          duration: 0.1, // Increased duration
          ease: 'power2.out',
          onStart: () => {
            gsap.set(char, {
              fontFamily: 'Cash Sans Mono',
              fontWeight: 300,
              color: '#666', // Add color change
            });
          },
          onComplete: () => {
            gsap.set(char, {
              innerHTML: initialHTML,
              color: '',
              fontFamily: '',
              opacity: 1,
            });
          },
          repeat: 2, // Reduced repeats
          onRepeat: () => {
            repeatCount++;
            if (repeatCount === 1) {
              gsap.set(char, {
                opacity: 0.5,
                color: '#999',
              });
            }
          },
          repeatRefresh: true,
          repeatDelay: 0.05, // Increased delay
          delay: position * 0.03, // Reduced delay between chars
          innerHTML: () => lettersAndSymbols[Math.floor(Math.random() * lettersAndSymbols.length)],
          opacity: 1,
        }
      );
    });
  }

  reset() {
    const chars = this.splitter.getChars();
    chars.forEach((char, index) => {
      gsap.killTweensOf(char);
      char.innerHTML = this.originalChars[index];
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
