import React, { useState, useEffect, useRef } from 'react';

// Import actual PNG images
import llamaSprite from '../../assets/battle-game/llama.png';
import gooseSprite from '../../assets/battle-game/goose.png';
import battleBackground from '../../assets/battle-game/background.png';
import battleMusic from '../../assets/battle-game/battle.mp3';

interface BattleState {
  currentStep: number;
  gooseHp: number;
  llamaHp: number;
  message: string;
  animation: string | null;
}

interface OllamaBattleGameProps {
  onComplete: (configValues: { [key: string]: string }) => void;
  requiredKeys: string[];
}

export function OllamaBattleGame({ onComplete, _requiredKeys }: OllamaBattleGameProps) {
  // Use type assertion for audioRef to avoid DOM lib dependency
  const audioRef = useRef<any>(null);
  const [isMuted, setIsMuted] = useState(false);

  const [battleState, setBattleState] = useState<BattleState>({
    currentStep: 0,
    gooseHp: 100,
    llamaHp: 100,
    message: 'A wild Ollama appeared!',
    animation: null,
  });

  const [configValues, setConfigValues] = useState<{ [key: string]: string }>({});

  // Initialize audio when component mounts
  useEffect(() => {
    if (typeof window !== 'undefined') {
      audioRef.current = new window.Audio(battleMusic);
      audioRef.current.loop = true;
      audioRef.current.volume = 0.5;
      audioRef.current.play().catch((e) => console.log('Audio autoplay prevented:', e));
    }

    return () => {
      if (audioRef.current) {
        audioRef.current.pause();
        audioRef.current = null;
      }
    };
  }, []);

  const toggleMute = () => {
    if (audioRef.current) {
      if (isMuted) {
        audioRef.current.volume = 0.5;
      } else {
        audioRef.current.volume = 0;
      }
      setIsMuted(!isMuted);
    }
  };

  const battleSteps = [
    {
      message: 'A wild Ollama appeared!',
      action: null,
      animation: 'appear',
    },
    {
      message: 'What will GOOSE do?',
      action: null,
    },
    {
      message: 'GOOSE used Configure Host!',
      action: 'host',
      prompt: 'Enter your Ollama host address:',
      configKey: 'OLLAMA_HOST',
      animation: 'attack',
      followUpMessages: ["It's super effective!", "OLLAMA's defense dropped sharply!"],
    },
    {
      message: 'OLLAMA is preparing a counter-attack!',
      action: null,
    },
    {
      message: 'OLLAMA used Model Selection!',
      action: 'model',
      prompt: 'Quick! Choose your model to counter:',
      configKey: 'OLLAMA_MODEL',
      animation: 'finish',
      followUpMessages: [
        "GOOSE's configuration was successful!",
        'OLLAMA has been configured!',
        'OLLAMA joined your team!',
      ],
    },
    {
      message: 'Configuration complete!\nOLLAMA will remember this friendship!',
      action: 'complete',
    },
  ];

  const animateHit = (isLlama: boolean) => {
    const element = document.querySelector(isLlama ? '.llama-sprite' : '.goose-sprite');
    if (element) {
      element.classList.add('hit-flash');
      setTimeout(() => {
        element.classList.remove('hit-flash');
      }, 500);
    }
  };

  useEffect(() => {
    // Add CSS for the hit animation
    const style = document.createElement('style');
    style.textContent = `
      @keyframes hitFlash {
        0%, 100% { opacity: 1; }
        50% { opacity: 0; }
      }
      .hit-flash {
        animation: hitFlash 0.5s;
      }
    `;
    document.head.appendChild(style);
    return () => {
      document.head.removeChild(style);
    };
  }, []);

  const handleAction = async (value: string) => {
    const currentStep =
      battleState.currentStep < battleSteps.length ? battleSteps[battleState.currentStep] : null;

    if (currentStep?.configKey && value) {
      setConfigValues((prev) => ({
        ...prev,
        [currentStep.configKey]: value,
      }));
      return; // Don't proceed with battle sequence during typing
    }

    // Only proceed with battle sequence on submit/continue
    if (currentStep?.animation === 'attack') {
      // Host configuration attack sequence
      setBattleState((prev) => ({
        ...prev,
        message: "It's super effective!",
        llamaHp: prev.llamaHp * 0.5, // Take away half HP
      }));
      animateHit(true);

      // Show follow-up messages with delays
      if (currentStep.followUpMessages) {
        for (const msg of currentStep.followUpMessages) {
          await new Promise((resolve) => setTimeout(resolve, 1000));
          setBattleState((prev) => ({ ...prev, message: msg }));
        }
      }

      await new Promise((resolve) => setTimeout(resolve, 1000));
    } else if (currentStep?.animation === 'finish') {
      // Final model selection sequence
      setBattleState((prev) => ({
        ...prev,
        llamaHp: 0,
      }));
      animateHit(true);

      // Show victory messages with delays
      if (currentStep.followUpMessages) {
        for (const msg of currentStep.followUpMessages) {
          await new Promise((resolve) => setTimeout(resolve, 1000));
          setBattleState((prev) => ({ ...prev, message: msg }));
        }
      }

      await new Promise((resolve) => setTimeout(resolve, 1000));
    }

    // Move to next step
    if (battleState.currentStep === battleSteps.length - 2) {
      // Last actionable step - complete configuration
      onComplete(configValues);
    }

    setBattleState((prev) => ({
      ...prev,
      currentStep: prev.currentStep + 1,
      message: battleSteps[prev.currentStep + 1]?.message || prev.message,
    }));
  };

  return (
    <div className="w-full h-full px-4 py-6">
      {/* Battle Scene */}
      <div
        className="relative w-full h-[300px] rounded-lg mb-4 bg-cover bg-center border-4 border-[#2C3E50] overflow-hidden"
        style={{
          backgroundImage: `url(${battleBackground})`,
          backgroundSize: 'cover',
          backgroundPosition: 'center bottom',
        }}
      >
        {/* Llama sprite */}
        <div className="absolute right-24 top-8">
          <div className="mb-2">
            <div className="bg-[#1F2937] rounded-lg px-3 py-1 text-white font-pokemon mb-1">
              OLLAMA
              <span className="text-xs ml-2">Lv.1</span>
            </div>
            <div className="flex items-center gap-2">
              <div className="h-2 bg-[#374151] rounded-full flex-grow">
                <div
                  className="h-full rounded-full transition-all duration-300"
                  style={{
                    width: `${battleState.llamaHp}%`,
                    backgroundColor:
                      battleState.llamaHp > 50
                        ? '#10B981'
                        : battleState.llamaHp > 20
                          ? '#F59E0B'
                          : '#EF4444',
                  }}
                />
              </div>
              <span className="text-sm font-pokemon text-[#1F2937]">
                {Math.floor(battleState.llamaHp)}/100
              </span>
            </div>
          </div>
          <img
            src={llamaSprite}
            alt="Llama"
            className="w-40 h-40 object-contain llama-sprite pixelated"
            style={{
              transform: `translateY(${battleState.currentStep % 2 === 1 ? '-4px' : '0'})`,
              transition: 'transform 0.3s ease-in-out',
              imageRendering: 'pixelated',
            }}
          />
        </div>

        {/* Goose sprite */}
        <div className="absolute left-24 bottom-4">
          <img
            src={gooseSprite}
            alt="Goose"
            className="w-40 h-40 object-contain mb-2 goose-sprite pixelated"
            style={{
              transform: `translateY(${battleState.currentStep % 2 === 0 ? '-4px' : '0'})`,
              transition: 'transform 0.3s ease-in-out',
              imageRendering: 'pixelated',
            }}
          />
          <div>
            <div className="bg-[#1F2937] rounded-lg px-3 py-1 text-white font-pokemon mb-1">
              GOOSE
              <span className="text-xs ml-2">Lv.99</span>
            </div>
            <div className="flex items-center gap-2">
              <div className="h-2 bg-[#374151] rounded-full flex-grow">
                <div
                  className="h-full rounded-full transition-all duration-300"
                  style={{
                    width: `${battleState.gooseHp}%`,
                    backgroundColor:
                      battleState.gooseHp > 50
                        ? '#10B981'
                        : battleState.gooseHp > 20
                          ? '#F59E0B'
                          : '#EF4444',
                  }}
                />
              </div>
              <span className="text-sm font-pokemon text-[#1F2937]">
                {Math.floor(battleState.gooseHp)}/100
              </span>
            </div>
          </div>
        </div>
      </div>

      {/* Dialog Box */}
      <div className="relative w-full">
        <div className="w-full bg-[#1F2937] rounded-lg p-6 border-4 border-[#4B5563] shadow-lg">
          <div className="absolute top-4 right-4">
            <button
              onClick={toggleMute}
              className="text-white hover:text-gray-300 transition-colors"
            >
              {isMuted ? '🔇' : '🔊'}
            </button>
          </div>
          <p className="text-lg mb-4 text-white font-pokemon leading-relaxed">
            {battleState.message}
          </p>

          {battleState.currentStep < battleSteps.length &&
            battleSteps[battleState.currentStep].action &&
            battleState.currentStep < battleSteps.length - 1 && (
              <div className="space-y-4">
                <p className="text-sm text-gray-300 font-pokemon">
                  {battleSteps[battleState.currentStep].prompt}
                </p>
                <div className="flex gap-2">
                  <input
                    type="text"
                    className="flex-grow px-4 py-2 bg-[#374151] border-2 border-[#4B5563] rounded-lg text-white font-pokemon placeholder-gray-400 focus:outline-none focus:border-[#60A5FA]"
                    placeholder={battleSteps[battleState.currentStep].prompt}
                    onChange={(e) => handleAction(e.target.value)}
                  />
                  <button
                    onClick={() => handleAction('')}
                    className="px-6 py-2 bg-[#2563EB] text-white font-pokemon rounded-lg hover:bg-[#1D4ED8] transition-colors focus:outline-none focus:ring-2 focus:ring-[#60A5FA] focus:ring-opacity-50"
                  >
                    Submit
                  </button>
                </div>
              </div>
            )}

          {(battleState.currentStep >= battleSteps.length ||
            !battleSteps[battleState.currentStep].action ||
            battleState.currentStep === battleSteps.length - 1) && (
            <button
              onClick={() => handleAction('')}
              className="mt-2 px-6 py-2 bg-[#2563EB] text-white font-pokemon rounded-lg hover:bg-[#1D4ED8] transition-colors focus:outline-none focus:ring-2 focus:ring-[#60A5FA] focus:ring-opacity-50"
            >
              ▶ Continue
            </button>
          )}
        </div>

        {/* Black corners for that classic Pokemon feel */}
        <div className="absolute top-0 left-0 w-4 h-4 bg-black rounded-tl-lg"></div>
        <div className="absolute top-0 right-0 w-4 h-4 bg-black rounded-tr-lg"></div>
        <div className="absolute bottom-0 left-0 w-4 h-4 bg-black rounded-bl-lg"></div>
        <div className="absolute bottom-0 right-0 w-4 h-4 bg-black rounded-br-lg"></div>
      </div>
    </div>
  );
}
