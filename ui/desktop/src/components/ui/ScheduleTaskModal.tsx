import React, { useState, useEffect } from 'react';
import { Card } from './card';
import { Clock } from 'lucide-react';

interface ScheduleTaskModalProps {
  onClose: () => void;
  onSchedule: (duration: number, interval: number) => void;
}

export function ScheduleTaskModal({ onClose, onSchedule }: ScheduleTaskModalProps) {
  const [duration, setDuration] = useState<number>(60); // Default 60 minutes
  const [interval, setInterval] = useState<number>(5); // Default 5 minutes

  const handleSchedule = () => {
    onSchedule(duration, interval);
    onClose();
  };

  // Handle Escape key press
  useEffect(() => {
    const handleEscKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose();
      }
    };

    document.addEventListener('keydown', handleEscKey);
    return () => {
      document.removeEventListener('keydown', handleEscKey);
    };
  }, [onClose]);

  return (
    <div className="fixed inset-0 bg-black/20 dark:bg-white/20 backdrop-blur-sm transition-colors flex items-center justify-center p-4 z-50">
      <Card className="relative w-[400px] max-w-full bg-bgApp rounded-xl shadow-lg overflow-hidden">
        <div className="p-6">
          <div className="flex flex-col">
            <h2 className="text-xl font-bold mb-4 text-textStandard flex items-center">
              <Clock className="h-5 w-5 mr-2" />
              Schedule Task
            </h2>

            <div className="space-y-4 mb-6">
              <div className="flex flex-col space-y-2">
                <label htmlFor="duration" className="text-sm font-medium text-textStandard">
                  Duration (minutes)
                </label>
                <input
                  id="duration"
                  type="number"
                  min={1}
                  value={duration}
                  onChange={(e) => setDuration(parseInt(e.target.value) || 1)}
                  className="w-full p-2 border border-borderSubtle rounded-md bg-transparent text-textStandard focus:border-blue-500 focus:outline-none"
                />
              </div>

              <div className="flex flex-col space-y-2">
                <label htmlFor="interval" className="text-sm font-medium text-textStandard">
                  Interval (minutes)
                </label>
                <input
                  id="interval"
                  type="number"
                  min={1}
                  value={interval}
                  onChange={(e) => setInterval(parseInt(e.target.value) || 1)}
                  className="w-full p-2 border border-borderSubtle rounded-md bg-transparent text-textStandard focus:border-blue-500 focus:outline-none"
                />
              </div>

              <div className="text-sm text-textSubtle mt-2">
                This will send "please do that task again" every {interval} minutes for {duration}{' '}
                minutes.
              </div>
            </div>

            <div className="flex space-x-3">
              <button
                onClick={handleSchedule}
                className="flex-1 px-4 py-2 bg-blue-500 text-white rounded-md hover:bg-blue-600 transition-colors"
              >
                Schedule
              </button>
              <button
                onClick={onClose}
                className="flex-1 px-4 py-2 border border-borderSubtle text-textStandard rounded-md hover:bg-bgSubtle transition-colors"
              >
                Cancel
              </button>
            </div>
          </div>
        </div>
      </Card>
    </div>
  );
}
