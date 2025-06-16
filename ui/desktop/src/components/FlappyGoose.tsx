import React, { useEffect, useRef, useState, useCallback } from 'react';

import svg1 from '../images/loading-goose/1.svg';
import svg7 from '../images/loading-goose/7.svg';

interface Obstacle {
  x: number;
  gapY: number;
  passed: boolean;
}

interface FlappyGooseProps {
  onClose: () => void;
}

const FlappyGoose: React.FC<FlappyGooseProps> = ({ onClose }) => {
  // eslint-disable-next-line no-undef
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const [gameOver, setGameOver] = useState(false);
  const [displayScore, setDisplayScore] = useState(0);
  // eslint-disable-next-line no-undef
  const gooseImages = useRef<HTMLImageElement[]>([]);
  const framesLoaded = useRef(0);
  const [imagesReady, setImagesReady] = useState(false);

  // Game state
  const gameState = useRef({
    gooseY: 200,
    velocity: 0,
    obstacles: [] as Obstacle[],
    gameLoop: 0,
    running: false,
    score: 0,
    isFlapping: false,
    flapEndTime: 0,
  });

  // Game settings
  const CANVAS_WIDTH = 600;
  const CANVAS_HEIGHT = 400;
  const GRAVITY = 0.35;
  const FLAP_FORCE = -7;
  const OBSTACLE_SPEED = 2.5;
  const OBSTACLE_GAP = 180;
  const GOOSE_SIZE = 35;
  const GOOSE_X = 50;
  const OBSTACLE_WIDTH = 40;
  const FLAP_DURATION = 150;

  const safeRequestAnimationFrame = useCallback((callback: (time: number) => void) => {
    if (typeof window !== 'undefined' && typeof requestAnimationFrame !== 'undefined') {
      requestAnimationFrame(callback);
    }
  }, []);

  const handleGameOver = useCallback(() => {
    gameState.current.running = false;
    setGameOver(true);
  }, []);

  const gameLoop = useCallback(() => {
    if (!gameState.current.running || !imagesReady) return;
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    // Check if flap animation should end
    if (gameState.current.isFlapping && Date.now() >= gameState.current.flapEndTime) {
      gameState.current.isFlapping = false;
    }

    // Update goose position
    gameState.current.velocity += GRAVITY;
    gameState.current.gooseY += gameState.current.velocity;

    // Generate obstacles
    if (gameState.current.gameLoop % 120 === 0) {
      gameState.current.obstacles.push({
        x: CANVAS_WIDTH,
        gapY: Math.random() * (CANVAS_HEIGHT - OBSTACLE_GAP - 100) + 50,
        passed: false,
      });
    }

    // Update obstacles and check for score
    gameState.current.obstacles = gameState.current.obstacles.filter((obstacle) => {
      obstacle.x -= OBSTACLE_SPEED;

      // Check for score when the goose passes the middle of the obstacle
      const obstacleMiddle = obstacle.x + OBSTACLE_WIDTH / 2;
      const gooseMiddle = GOOSE_X + GOOSE_SIZE / 2;

      if (!obstacle.passed && obstacleMiddle < gooseMiddle) {
        obstacle.passed = true;
        gameState.current.score += 1;
        setDisplayScore(gameState.current.score);
      }

      return obstacle.x > -OBSTACLE_WIDTH;
    });

    // Check collisions
    const gooseBox = {
      x: GOOSE_X,
      y: gameState.current.gooseY,
      width: GOOSE_SIZE,
      height: GOOSE_SIZE,
    };

    // Collision with ground or ceiling
    if (gameState.current.gooseY < 0 || gameState.current.gooseY > CANVAS_HEIGHT - GOOSE_SIZE) {
      handleGameOver();
      return;
    }

    // Collision with obstacles
    for (const obstacle of gameState.current.obstacles) {
      if (gooseBox.x < obstacle.x + OBSTACLE_WIDTH && gooseBox.x + gooseBox.width > obstacle.x) {
        if (
          gooseBox.y < obstacle.gapY - OBSTACLE_GAP / 2 ||
          gooseBox.y + gooseBox.height > obstacle.gapY + OBSTACLE_GAP / 2
        ) {
          handleGameOver();
          return;
        }
      }
    }

    // Draw game
    ctx.clearRect(0, 0, CANVAS_WIDTH, CANVAS_HEIGHT);

    // Draw rotated goose
    ctx.save();
    ctx.translate(GOOSE_X + GOOSE_SIZE / 2, gameState.current.gooseY + GOOSE_SIZE / 2);
    const rotation = Math.min(Math.max(gameState.current.velocity * 0.05, -0.5), 0.5);
    ctx.rotate(rotation);
    ctx.drawImage(
      gooseImages.current[gameState.current.isFlapping ? 1 : 0],
      -GOOSE_SIZE / 2,
      -GOOSE_SIZE / 2,
      GOOSE_SIZE,
      GOOSE_SIZE
    );
    ctx.restore();

    // Draw obstacles
    ctx.fillStyle = '#4CAF50';
    gameState.current.obstacles.forEach((obstacle) => {
      // Top obstacle
      ctx.fillRect(obstacle.x, 0, OBSTACLE_WIDTH, obstacle.gapY - OBSTACLE_GAP / 2);
      // Bottom obstacle
      ctx.fillRect(
        obstacle.x,
        obstacle.gapY + OBSTACLE_GAP / 2,
        OBSTACLE_WIDTH,
        CANVAS_HEIGHT - (obstacle.gapY + OBSTACLE_GAP / 2)
      );
    });

    // Draw score
    ctx.fillStyle = '#000';
    ctx.font = '24px Arial';
    ctx.fillText(`Score: ${gameState.current.score}`, 10, 30);

    gameState.current.gameLoop++;
    safeRequestAnimationFrame(gameLoop);
  }, [
    CANVAS_HEIGHT,
    CANVAS_WIDTH,
    GOOSE_SIZE,
    GOOSE_X,
    GRAVITY,
    OBSTACLE_GAP,
    OBSTACLE_SPEED,
    OBSTACLE_WIDTH,
    handleGameOver,
    imagesReady,
    safeRequestAnimationFrame,
  ]);

  const startGame = useCallback(() => {
    if (gameState.current.running || !imagesReady || typeof window === 'undefined') return;

    gameState.current = {
      gooseY: CANVAS_HEIGHT / 3,
      velocity: 0,
      obstacles: [],
      gameLoop: 0,
      running: true,
      score: 0,
      isFlapping: false,
      flapEndTime: 0,
    };
    setGameOver(false);
    setDisplayScore(0);
    safeRequestAnimationFrame(gameLoop);
  }, [CANVAS_HEIGHT, imagesReady, safeRequestAnimationFrame, gameLoop]);

  const flap = useCallback(() => {
    if (gameOver) {
      startGame();
      return;
    }
    gameState.current.velocity = FLAP_FORCE;
    gameState.current.isFlapping = true;
    gameState.current.flapEndTime = Date.now() + FLAP_DURATION;
  }, [FLAP_DURATION, FLAP_FORCE, gameOver, startGame]);

  // Load goose images
  useEffect(() => {
    const frames = [svg1, svg7];
    frames.forEach((src, index) => {
      // eslint-disable-next-line no-undef
      const img = new Image() as HTMLImageElement;
      img.src = src;
      img.onload = () => {
        framesLoaded.current += 1;
        if (framesLoaded.current === frames.length) {
          setImagesReady(true);
        }
      };
      gooseImages.current[index] = img;
    });
  }, []);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    canvas.width = CANVAS_WIDTH;
    canvas.height = CANVAS_HEIGHT;

    const handleKeyPress = (e: KeyboardEvent) => {
      if (e.code === 'Space') {
        e.preventDefault();
        flap();
      }
    };

    window.addEventListener('keydown', handleKeyPress);

    if (imagesReady) {
      startGame();
    }

    return () => {
      window.removeEventListener('keydown', handleKeyPress);
      gameState.current.running = false;
    };
  }, [CANVAS_HEIGHT, CANVAS_WIDTH, flap, imagesReady, startGame]);

  return (
    <div
      style={{
        position: 'fixed',
        top: 0,
        left: 0,
        width: '100%',
        height: '100%',
        backgroundColor: 'rgba(0, 0, 0, 0.8)',
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: 1000,
      }}
      onClick={flap}
    >
      <canvas
        ref={(el) => {
          canvasRef.current = el;
        }}
        style={{
          border: '2px solid #333',
          borderRadius: '8px',
          backgroundColor: '#87CEEB',
          maxWidth: '100%',
          maxHeight: '100vh',
        }}
      />
      {!imagesReady && <div style={{ color: 'white', fontSize: '24px' }}>Loading...</div>}
      {gameOver && (
        <div
          style={{
            position: 'absolute',
            color: 'white',
            fontSize: '24px',
            textAlign: 'center',
          }}
        >
          <p>Game Over!</p>
          <p>Score: {displayScore}</p>
          <p>Click or press space to play again</p>
        </div>
      )}
      <button
        onClick={(e) => {
          e.stopPropagation();
          onClose();
        }}
        style={{
          position: 'absolute',
          top: '20px',
          right: '20px',
          padding: '8px 16px',
          backgroundColor: '#ff4444',
          color: 'white',
          border: 'none',
          borderRadius: '4px',
          cursor: 'pointer',
        }}
      >
        Close
      </button>
    </div>
  );
};

export default FlappyGoose;
