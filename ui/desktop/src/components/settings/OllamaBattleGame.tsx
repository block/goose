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
  lastChoice?: string;
  showHostInput?: boolean;
  processingAction?: boolean;
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
    processingAction: false,
  });

  const [configValues, setConfigValues] = useState<{ [key: string]: string }>({});

  // Initialize audio when component mounts
  useEffect(() => {
    if (typeof window !== 'undefined') {
      audioRef.current = new window.Audio(battleMusic);
      audioRef.current.loop = true;
      audioRef.current.volume = 0.2;
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
        audioRef.current.volume = 0.2;
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
      action: 'choice',
      choices: ['Pacify', 'HONK!'],
      animation: 'attack',
      followUpMessages: ["It's not very effective...", 'But OLLAMA is confused!'],
    },
    {
      message: 'OLLAMA used YAML Confusion!',
      action: null,
      animation: 'counter',
      followUpMessages: ['OLLAMA hurt itself in confusion!', 'GOOSE maintained composure!'],
    },
    {
      message: 'What will GOOSE do?',
      action: 'final_choice',
      choices: (previousChoice: string) => [
        previousChoice === 'Pacify' ? 'HONK!' : 'Pacify',
        'Configure Host',
      ],
      animation: 'attack',
    },
    {
      message: 'OLLAMA used Docker Dependency!',
      action: null,
      animation: 'counter',
      followUpMessages: ["It's not very effective...", 'GOOSE knows containerization!'],
    },
    {
      message: 'What will GOOSE do?',
      action: 'host_choice',
      choices: ['Configure Host'],
      animation: 'finish',
    },
    {
      message: '', // Will be set dynamically based on choice
      action: 'host_input',
      prompt: 'Enter your Ollama host address:',
      configKey: 'OLLAMA_HOST',
      animation: 'finish',
      followUpMessages: [
        "It's super effective!",
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
    // Add CSS for the hit animation and defeat animation
    const style = document.createElement('style');
    style.textContent = `
      @keyframes hitFlash {
        0%, 100% { opacity: 1; }
        50% { opacity: 0; }
      }
      .hit-flash {
        animation: hitFlash 0.5s;
      }
      @keyframes defeat {
        0% { transform: translateY(0); opacity: 1; }
        20% { transform: translateY(-30px); opacity: 1; }
        100% { transform: translateY(500px); opacity: 0; }
      }
      .defeated {
        animation: defeat 1.3s cubic-bezier(.36,.07,.19,.97) both;
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

    if (!currentStep) return;

    // Handle host input
    if (currentStep.action === 'host_input' && value) {
      setConfigValues((prev) => ({
        ...prev,
        [currentStep.configKey]: value,
      }));
      return;
    }

    // Handle host submit
    if (currentStep.action === 'host_input' && !value) {
      setBattleState((prev) => ({
        ...prev,
        processingAction: true,
        llamaHp: 0,
        message: "It's super effective!",
      }));
      animateHit(true);

      // Add defeat class to llama sprite and health bar
      const llamaContainer = document.querySelector('.llama-container');
      if (llamaContainer) {
        await new Promise((resolve) => setTimeout(resolve, 500));
        llamaContainer.classList.add('defeated');
      }

      // Show victory messages with delays
      if (currentStep.followUpMessages) {
        for (const msg of currentStep.followUpMessages) {
          await new Promise((resolve) => setTimeout(resolve, 1000));
          setBattleState((prev) => ({ ...prev, message: msg }));
        }
      }

      await new Promise((resolve) => setTimeout(resolve, 1000));
      onComplete(configValues);
      return;
    }

    // Handle continue button for messages
    if (!currentStep.action) {
      setBattleState((prev) => ({
        ...prev,
        currentStep: prev.currentStep + 1,
        message: battleSteps[prev.currentStep + 1]?.message || prev.message,
        processingAction: false,
      }));
      return;
    }

    // Handle choices (Pacify/HONK/Configure Host)
    if (
      (currentStep.action === 'choice' ||
        currentStep.action === 'final_choice' ||
        currentStep.action === 'host_choice') &&
      value
    ) {
      // Set processing flag to hide buttons
      setBattleState((prev) => ({
        ...prev,
        processingAction: true,
      }));

      if (value === 'Configure Host') {
        setBattleState((prev) => ({
          ...prev,
          message: 'GOOSE used Configure Host!',
          showHostInput: true,
          currentStep: battleSteps.findIndex((step) => step.action === 'host_input'),
          processingAction: false,
        }));
        return;
      }

      // Handle Pacify or HONK attacks
      setBattleState((prev) => ({
        ...prev,
        lastChoice: value,
        llamaHp: Math.max(0, prev.llamaHp - 25),
        message: `GOOSE used ${value}!`,
      }));
      animateHit(true);

      // Show follow-up messages
      if (currentStep.followUpMessages) {
        for (const msg of currentStep.followUpMessages) {
          await new Promise((resolve) => setTimeout(resolve, 1000));
          setBattleState((prev) => ({ ...prev, message: msg }));
        }
      }

      // Proceed to counter-attack
      await new Promise((resolve) => setTimeout(resolve, 1000));
      const isFirstCycle = currentStep.action === 'choice';
      const nextStep = battleSteps[battleState.currentStep + 1];
      setBattleState((prev) => ({
        ...prev,
        gooseHp: Math.max(0, prev.gooseHp - 25),
        message: isFirstCycle ? 'OLLAMA used YAML Confusion!' : 'OLLAMA used Docker Dependency!',
        currentStep: prev.currentStep + 1,
        processingAction: false,
      }));
      animateHit(false);

      // Show counter-attack messages
      if (nextStep?.followUpMessages) {
        await new Promise((resolve) => setTimeout(resolve, 1000));
        for (const msg of nextStep.followUpMessages) {
          await new Promise((resolve) => setTimeout(resolve, 1000));
          setBattleState((prev) => ({ ...prev, message: msg }));
        }
      }

      return;
    }

    // Check for battle completion
    if (battleState.currentStep === battleSteps.length - 2) {
      onComplete(configValues);
    }
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
        <div className="absolute right-24 top-8 llama-container">
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

          {battleState.currentStep < battleSteps.length && (
            <div className="space-y-4">
              {/* Show battle choices */}
              {(battleSteps[battleState.currentStep].action === 'choice' ||
                battleSteps[battleState.currentStep].action === 'final_choice' ||
                battleSteps[battleState.currentStep].action === 'host_choice') &&
                !battleState.showHostInput &&
                !battleState.processingAction && (
                  <div className="space-y-2">
                    {(typeof battleSteps[battleState.currentStep].choices === 'function'
                      ? battleSteps[battleState.currentStep].choices(battleState.lastChoice || '')
                      : battleSteps[battleState.currentStep].choices
                    )?.map((choice: string) => (
                      <button
                        key={choice}
                        onClick={() => handleAction(choice)}
                        className="w-full text-left px-4 py-2 text-white font-pokemon hover:bg-[#2563EB] transition-colors rounded-lg group flex items-center"
                      >
                        <span className="opacity-0 group-hover:opacity-100 transition-opacity mr-2">
                          ▶
                        </span>
                        {choice}
                      </button>
                    ))}
                  </div>
                )}

              {/* Show host input when needed */}
              {battleState.showHostInput && !battleState.processingAction && (
                <div className="space-y-4">
                  <p className="text-sm text-gray-300 font-pokemon">
                    Enter your Ollama host address:
                  </p>
                  <div className="flex gap-2">
                    <input
                      type="text"
                      className="flex-grow px-4 py-2 bg-[#374151] border-2 border-[#4B5563] rounded-lg text-white font-pokemon placeholder-gray-400 focus:outline-none focus:border-[#60A5FA]"
                      placeholder="http://localhost:11434"
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

              {/* Continue button for messages */}
              {!battleSteps[battleState.currentStep].action && !battleState.processingAction && (
                <button
                  onClick={() => handleAction('')}
                  className="mt-2 px-6 py-2 bg-[#2563EB] text-white font-pokemon rounded-lg hover:bg-[#1D4ED8] transition-colors focus:outline-none focus:ring-2 focus:ring-[#60A5FA] focus:ring-opacity-50"
                >
                  ▶ Continue
                </button>
              )}
            </div>
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
