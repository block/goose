import React, { useState } from 'react';
import { Button } from './ui/button';
import { useCodeArtifacts } from '../hooks/useCodeArtifacts';
import CodeArtifactView from './CodeArtifactView';

export const CodeArtifactDemo: React.FC = () => {
  const { addArtifact } = useCodeArtifacts();
  const [showArtifacts, setShowArtifacts] = useState(false);

  const createSampleArtifacts = () => {
    // Create a sample Pomodoro Timer
    addArtifact({
      title: 'Pomodoro Timer',
      language: 'html',
      description: 'A simple Pomodoro timer with HTML, CSS, and JavaScript',
      code: `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Pomodoro Timer</title>
    <script src="https://cdn.tailwindcss.com"></script>
</head>
<body class="bg-gray-100 min-h-screen flex items-center justify-center">
    <div class="bg-white p-8 rounded-lg shadow-lg max-w-md w-full">
        <h1 class="text-3xl font-bold text-center text-gray-800 mb-8">Pomodoro Timer</h1>
        
        <div class="text-center mb-8">
            <div class="text-6xl font-mono text-gray-800 mb-4" id="timer">25:00</div>
            <div class="text-lg text-gray-600 mb-4" id="status">Work Time</div>
        </div>
        
        <div class="flex justify-center space-x-4 mb-6">
            <button id="start" class="bg-green-500 hover:bg-green-600 text-white px-6 py-2 rounded-lg font-medium">
                Start
            </button>
            <button id="pause" class="bg-yellow-500 hover:bg-yellow-600 text-white px-6 py-2 rounded-lg font-medium hidden">
                Pause
            </button>
            <button id="reset" class="bg-red-500 hover:bg-red-600 text-white px-6 py-2 rounded-lg font-medium">
                Reset
            </button>
        </div>
        
        <div class="flex justify-center space-x-4">
            <button id="work" class="bg-blue-500 hover:bg-blue-600 text-white px-4 py-2 rounded-lg text-sm">
                Work (25m)
            </button>
            <button id="shortBreak" class="bg-green-500 hover:bg-green-600 text-white px-4 py-2 rounded-lg text-sm">
                Short Break (5m)
            </button>
            <button id="longBreak" class="bg-purple-500 hover:bg-purple-600 text-white px-4 py-2 rounded-lg text-sm">
                Long Break (15m)
            </button>
        </div>
    </div>

    <script>
        class PomodoroTimer {
            constructor() {
                this.workTime = 25 * 60; // 25 minutes
                this.shortBreakTime = 5 * 60; // 5 minutes
                this.longBreakTime = 15 * 60; // 15 minutes
                this.currentTime = this.workTime;
                this.isRunning = false;
                this.interval = null;
                this.isWorkTime = true;
                
                this.initializeElements();
                this.bindEvents();
                this.updateDisplay();
            }
            
            initializeElements() {
                this.timerElement = document.getElementById('timer');
                this.statusElement = document.getElementById('status');
                this.startButton = document.getElementById('start');
                this.pauseButton = document.getElementById('pause');
                this.resetButton = document.getElementById('reset');
                this.workButton = document.getElementById('work');
                this.shortBreakButton = document.getElementById('shortBreak');
                this.longBreakButton = document.getElementById('longBreak');
            }
            
            bindEvents() {
                this.startButton.addEventListener('click', () => this.start());
                this.pauseButton.addEventListener('click', () => this.pause());
                this.resetButton.addEventListener('click', () => this.reset());
                this.workButton.addEventListener('click', () => this.setWorkTime());
                this.shortBreakButton.addEventListener('click', () => this.setShortBreak());
                this.longBreakButton.addEventListener('click', () => this.setLongBreak());
            }
            
            start() {
                if (!this.isRunning) {
                    this.isRunning = true;
                    this.startButton.classList.add('hidden');
                    this.pauseButton.classList.remove('hidden');
                    
                    this.interval = setInterval(() => {
                        this.currentTime--;
                        this.updateDisplay();
                        
                        if (this.currentTime <= 0) {
                            this.complete();
                        }
                    }, 1000);
                }
            }
            
            pause() {
                this.isRunning = false;
                this.startButton.classList.remove('hidden');
                this.pauseButton.classList.add('hidden');
                clearInterval(this.interval);
            }
            
            reset() {
                this.pause();
                this.currentTime = this.workTime;
                this.isWorkTime = true;
                this.updateDisplay();
                this.updateStatus();
            }
            
            setWorkTime() {
                this.currentTime = this.workTime;
                this.isWorkTime = true;
                this.updateDisplay();
                this.updateStatus();
            }
            
            setShortBreak() {
                this.currentTime = this.shortBreakTime;
                this.isWorkTime = false;
                this.updateDisplay();
                this.updateStatus();
            }
            
            setLongBreak() {
                this.currentTime = this.longBreakTime;
                this.isWorkTime = false;
                this.updateDisplay();
                this.updateStatus();
            }
            
                         updateDisplay() {
                 const minutes = Math.floor(this.currentTime / 60);
                 const seconds = this.currentTime % 60;
                 this.timerElement.textContent = minutes.toString().padStart(2, '0') + ':' + seconds.toString().padStart(2, '0');
             }
            
            updateStatus() {
                this.statusElement.textContent = this.isWorkTime ? 'Work Time' : 'Break Time';
            }
            
            complete() {
                this.pause();
                this.updateDisplay();
                
                if (this.isWorkTime) {
                    this.statusElement.textContent = 'Work session completed!';
                    this.statusElement.className = 'text-lg text-green-600 mb-4';
                } else {
                    this.statusElement.textContent = 'Break completed!';
                    this.statusElement.className = 'text-lg text-blue-600 mb-4';
                }
                
                // Play notification sound if available
                if ('Notification' in window && Notification.permission === 'granted') {
                    new Notification('Pomodoro Timer', {
                        body: this.isWorkTime ? 'Work session completed!' : 'Break completed!',
                        icon: '/favicon.ico'
                    });
                }
            }
        }
        
        // Initialize the timer when the page loads
        document.addEventListener('DOMContentLoaded', () => {
            new PomodoroTimer();
        });
    </script>
</body>
</html>`,
    });

    // Create a sample React component
    addArtifact({
      title: 'React Counter Component',
      language: 'jsx',
      description: 'A simple React counter component with hooks',
      code: `import React, { useState, useEffect } from 'react';

const Counter = ({ initialValue = 0, step = 1 }) => {
  const [count, setCount] = useState(initialValue);
  const [isAutoIncrementing, setIsAutoIncrementing] = useState(false);

  useEffect(() => {
    let interval;
    if (isAutoIncrementing) {
      interval = setInterval(() => {
        setCount(prev => prev + step);
      }, 1000);
    }
    return () => clearInterval(interval);
  }, [isAutoIncrementing, step]);

  const increment = () => setCount(prev => prev + step);
  const decrement = () => setCount(prev => prev - step);
  const reset = () => setCount(initialValue);
  const toggleAutoIncrement = () => setIsAutoIncrementing(prev => !prev);

  return (
    <div className="counter-container">
      <h2>Counter: {count}</h2>
      
      <div className="counter-controls">
        <button onClick={decrement} className="btn btn-decrement">
          -{step}
        </button>
        
        <button onClick={increment} className="btn btn-increment">
          +{step}
        </button>
        
        <button onClick={reset} className="btn btn-reset">
          Reset
        </button>
        
        <button 
          onClick={toggleAutoIncrement} 
          className={'btn ' + (isAutoIncrementing ? 'btn-stop' : 'btn-start')}
        >
          {isAutoIncrementing ? 'Stop Auto' : 'Start Auto'}
        </button>
      </div>
      
      <div className="counter-info">
        <p>Step size: {step}</p>
        <p>Initial value: {initialValue}</p>
        {isAutoIncrementing && (
          <p className="auto-info">Auto-incrementing every second...</p>
        )}
      </div>
      
      <style jsx>{\`
        .counter-container {
          padding: 20px;
          border: 2px solid #e2e8f0;
          border-radius: 8px;
          max-width: 400px;
          margin: 20px auto;
          text-align: center;
          background: white;
        }
        
        .counter-controls {
          display: flex;
          gap: 10px;
          justify-content: center;
          margin: 20px 0;
          flex-wrap: wrap;
        }
        
        .btn {
          padding: 8px 16px;
          border: none;
          border-radius: 4px;
          cursor: pointer;
          font-weight: 500;
          transition: background-color 0.2s;
        }
        
        .btn-increment {
          background-color: #10b981;
          color: white;
        }
        
        .btn-increment:hover {
          background-color: #059669;
        }
        
        .btn-decrement {
          background-color: #ef4444;
          color: white;
        }
        
        .btn-decrement:hover {
          background-color: #dc2626;
        }
        
        .btn-reset {
          background-color: #6b7280;
          color: white;
        }
        
        .btn-reset:hover {
          background-color: #4b5563;
        }
        
        .btn-start {
          background-color: #3b82f6;
          color: white;
        }
        
        .btn-start:hover {
          background-color: #2563eb;
        }
        
        .btn-stop {
          background-color: #f59e0b;
          color: white;
        }
        
        .btn-stop:hover {
          background-color: #d97706;
        }
        
        .counter-info {
          margin-top: 20px;
          padding: 15px;
          background-color: #f8fafc;
          border-radius: 4px;
        }
        
        .auto-info {
          color: #3b82f6;
          font-weight: 500;
        }
      \`}</style>
    </div>
  );
};

export default Counter;`,
    });

    // Create a sample CSS animation
    addArtifact({
      title: 'CSS Loading Animation',
      language: 'css',
      description: 'A beautiful CSS loading spinner animation',
      code: `/* Loading Spinner Container */
.loading-container {
  display: flex;
  justify-content: center;
  align-items: center;
  min-height: 200px;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
}

/* Main Spinner */
.spinner {
  width: 50px;
  height: 50px;
  border: 4px solid rgba(255, 255, 255, 0.3);
  border-top: 4px solid #ffffff;
  border-radius: 50%;
  animation: spin 1s linear infinite;
  position: relative;
}

/* Pulsing Effect */
.spinner::before {
  content: '';
  position: absolute;
  top: -8px;
  left: -8px;
  right: -8px;
  bottom: -8px;
  border: 2px solid rgba(255, 255, 255, 0.1);
  border-radius: 50%;
  animation: pulse 2s ease-in-out infinite;
}

/* Dots Animation */
.loading-dots {
  display: flex;
  gap: 8px;
  margin-top: 20px;
}

.dot {
  width: 12px;
  height: 12px;
  background-color: #ffffff;
  border-radius: 50%;
  animation: bounce 1.4s ease-in-out infinite both;
}

.dot:nth-child(1) { animation-delay: -0.32s; }
.dot:nth-child(2) { animation-delay: -0.16s; }
.dot:nth-child(3) { animation-delay: 0s; }

/* Keyframe Animations */
@keyframes spin {
  0% { transform: rotate(0deg); }
  100% { transform: rotate(360deg); }
}

@keyframes pulse {
  0%, 100% {
    transform: scale(1);
    opacity: 1;
  }
  50% {
    transform: scale(1.1);
    opacity: 0.5;
  }
}

@keyframes bounce {
  0%, 80%, 100% {
    transform: scale(0);
  }
  40% {
    transform: scale(1);
  }
}

/* Loading Text */
.loading-text {
  color: white;
  font-family: 'Arial', sans-serif;
  font-size: 18px;
  font-weight: 500;
  margin-top: 15px;
  text-align: center;
  animation: fadeInOut 2s ease-in-out infinite;
}

@keyframes fadeInOut {
  0%, 100% { opacity: 0.7; }
  50% { opacity: 1; }
}

/* Responsive Design */
@media (max-width: 768px) {
  .spinner {
    width: 40px;
    height: 40px;
    border-width: 3px;
  }
  
  .loading-text {
    font-size: 16px;
  }
}

/* Dark Mode Support */
@media (prefers-color-scheme: dark) {
  .loading-container {
    background: linear-gradient(135deg, #1f2937 0%, #374151 100%);
  }
}

/* High Contrast Mode */
@media (prefers-contrast: high) {
  .spinner {
    border-color: #000000;
    border-top-color: #ffffff;
  }
  
  .dot {
    background-color: #000000;
  }
}`,
    });
  };

  return (
    <div className="p-6">
      <div className="max-w-4xl mx-auto">
        <div className="text-center mb-8">
          <h1 className="text-3xl font-bold text-text-default mb-4">Code Artifacts Demo</h1>
          <p className="text-text-muted mb-6">
            Diese Demo zeigt, wie Code Artifacts in Goose funktionieren. Erstelle Beispiele und
            teste die Live-Preview-Funktionalität.
          </p>

          <div className="flex justify-center gap-4">
            <Button onClick={createSampleArtifacts} className="bg-blue-600 hover:bg-blue-700">
              Beispiele erstellen
            </Button>
            <Button onClick={() => setShowArtifacts(!showArtifacts)} variant="outline">
              {showArtifacts ? 'Demo ausblenden' : 'Artifacts anzeigen'}
            </Button>
          </div>
        </div>

        {showArtifacts && (
          <div className="border border-border-default rounded-lg overflow-hidden">
            <CodeArtifactView />
          </div>
        )}

        <div className="mt-8 p-6 bg-background-muted rounded-lg">
          <h2 className="text-xl font-semibold text-text-default mb-4">Wie es funktioniert:</h2>
          <ul className="space-y-2 text-text-muted">
            <li>• Klicke "Beispiele erstellen" um Demo-Code zu generieren</li>
            <li>• Öffne die Artifacts um sie zu betrachten</li>
            <li>• Nutze die Live-Preview für HTML/CSS/JavaScript</li>
            <li>• Bearbeite und speichere deine Änderungen</li>
            <li>• Exportiere deine Artifacts als Dateien</li>
          </ul>
        </div>
      </div>
    </div>
  );
};

export default CodeArtifactDemo;
