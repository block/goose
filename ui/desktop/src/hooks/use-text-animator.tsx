import { useEffect, useRef } from 'react';
import SplitType from 'split-type';

interface TextSplitterOptions {
  resizeCallback?: () => void;
  splitTypeTypes?: ('lines' | 'words' | 'chars')[];
}

export class TextSplitter {
  textElement: HTMLElement;
  onResize: (() => void) | null;
  splitText: SplitType;
  previousContainerWidth: number | null = null;
  resizeObserver: ResizeObserver | null = null;

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
    this.resizeObserver = new ResizeObserver(() => {
      if (this.textElement) {
        const currentWidth = Math.floor(this.textElement.getBoundingClientRect().width);

        if (this.previousContainerWidth !== null && this.previousContainerWidth !== currentWidth) {
          this.splitText.split({ types: ['chars'] });
          this.onResize?.();
        }

        this.previousContainerWidth = currentWidth;
      }
    });

    this.resizeObserver.observe(this.textElement);
  }

  destroy() {
    this.resizeObserver?.disconnect();
    this.resizeObserver = null;
    this.splitText.revert();
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
    const updateOriginalChars = () => {
      this.originalChars = this.splitter.getChars().map((char) => char.textContent || '');
    };

    this.splitter = new TextSplitter(this.textElement, {
      resizeCallback: updateOriginalChars,
      splitTypeTypes: ['words', 'chars'],
    });
    updateOriginalChars();
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
    this.activeTimeouts.forEach((timeoutId) => {
      clearTimeout(timeoutId);
    });
    this.activeTimeouts = [];
    this.activeAnimations.forEach((animation) => {
      animation.cancel();
    });
    this.activeAnimations = [];

    const chars = this.splitter.getChars();
    chars.forEach((char, index) => {
      const original = this.originalChars[index];
      if (original !== undefined) {
        char.textContent = original;
      }
    });
  }

  destroy() {
    this.reset();
    this.splitter.destroy();
  }
}

interface UseTextAnimatorProps {
  text: string;
}

function prefersReducedMotion(): boolean {
  return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}

export function useTextAnimator({ text: _text }: UseTextAnimatorProps) {
  const elementRef = useRef<HTMLDivElement>(null);
  const animator = useRef<TextAnimator | null>(null);

  useEffect(() => {
    if (!elementRef.current) return;

    if (prefersReducedMotion()) {
      return;
    }

    animator.current = new TextAnimator(elementRef.current);

    const timeoutId = setTimeout(() => {
      animator.current?.animate();
    }, 100);

    return () => {
      window.clearTimeout(timeoutId);
      animator.current?.destroy();
      animator.current = null;
    };
  }, []);

  return elementRef;
}
