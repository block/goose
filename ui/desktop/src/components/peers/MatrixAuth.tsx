import React, { useState } from 'react';
import { motion } from 'framer-motion';
import { User, Lock, Server, UserPlus, LogIn } from 'lucide-react';
import { useMatrix } from '../../contexts/MatrixContext';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import { Label } from '../ui/label';
import { MatrixLogo } from '../icons/MatrixLogo';
import { MatrixDancingLines } from '../icons/MatrixDancingLines';
import { cn } from '../../utils';
import { useNavigation } from '../Layout/AppLayout';

interface MatrixAuthProps {
  onClose: () => void;
}

const MatrixAuth: React.FC<MatrixAuthProps> = ({ onClose }) => {
  const { login, register, isConnected } = useMatrix();
  const { isNavExpanded } = useNavigation();
  const [mode, setMode] = useState<'login' | 'register'>('login');
  const [formData, setFormData] = useState({
    username: '',
    password: '',
    homeserver: 'https://matrix.tchncs.de',
  });
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsLoading(true);
    setError(null);

    try {
      if (mode === 'login') {
        await login(formData.username, formData.password);
      } else {
        await register(formData.username, formData.password);
      }
      onClose();
    } catch (err: any) {
      setError(err.message || 'Authentication failed');
    } finally {
      setIsLoading(false);
    }
  };

  const handleInputChange = (field: string, value: string) => {
    setFormData(prev => ({ ...prev, [field]: value }));
  };

  if (isConnected) {
    return (
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        className="absolute inset-0 bg-black/50 flex items-center justify-center z-50 overflow-hidden"
        style={{
          top: isNavExpanded ? 'auto' : 0,
          marginTop: isNavExpanded ? 0 : 0,
        }}
      >
        {/* Matrix Dancing Lines Background */}
        <div className="absolute inset-0 text-text-accent">
          <MatrixDancingLines className="w-full h-full" />
        </div>
        <motion.div
          initial={{ scale: 0.9, opacity: 0 }}
          animate={{ scale: 1, opacity: 1 }}
          className="bg-background-card/95 backdrop-blur-sm border border-border-default rounded-xl p-6 max-w-md w-full mx-4 shadow-default relative z-10"
        >
          <div className="text-center">
            <div className="w-16 h-16 bg-background-success rounded-full flex items-center justify-center mx-auto mb-4">
              <User className="w-8 h-8 text-text-success" />
            </div>
            <h2 className="text-xl font-semibold mb-2 text-text-default">Connected!</h2>
            <p className="text-text-muted mb-6">
              You're now connected to Matrix and ready to chat with friends.
            </p>
            <Button onClick={onClose} className="w-full">
              Continue
            </Button>
          </div>
        </motion.div>
      </motion.div>
    );
  }

  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      className="absolute inset-0 bg-black/50 flex items-center justify-center z-50 overflow-hidden"
      style={{
        top: isNavExpanded ? 'auto' : 0,
        marginTop: isNavExpanded ? 0 : 0,
      }}
    >
      {/* Matrix Dancing Lines Background */}
      <div className="absolute inset-0 text-text-accent">
        <MatrixDancingLines className="w-full h-full" />
      </div>
      <motion.div
        initial={{ scale: 0.9, opacity: 0 }}
        animate={{ scale: 1, opacity: 1 }}
        className="bg-background-card/95 backdrop-blur-sm border border-border-default rounded-xl p-6 max-w-md w-full mx-4 shadow-default relative z-10"
      >
        <div className="text-center mb-6">
          <div className="w-16 h-16 bg-background-default border border-border-default rounded-full flex items-center justify-center mx-auto mb-4">
            <MatrixLogo size={32} className="text-text-default" />
          </div>
          <h2 className="text-xl font-semibold mb-2 text-text-default">Connect to Matrix</h2>
          <p className="text-text-muted">
            Connect to Matrix to chat with friends and collaborate on AI sessions.
          </p>
        </div>

        {/* Mode Toggle */}
        <div className="flex bg-background-muted rounded-lg p-1 mb-6">
          <button
            onClick={() => setMode('login')}
            className={cn(
              "flex-1 py-2 px-4 rounded-md text-sm font-medium transition-colors",
              mode === 'login'
                ? 'bg-background-card text-text-accent shadow-xs'
                : 'text-text-muted hover:text-text-default'
            )}
          >
            <LogIn className="w-4 h-4 inline mr-2" />
            Sign In
          </button>
          <button
            onClick={() => setMode('register')}
            className={cn(
              "flex-1 py-2 px-4 rounded-md text-sm font-medium transition-colors",
              mode === 'register'
                ? 'bg-background-card text-text-accent shadow-xs'
                : 'text-text-muted hover:text-text-default'
            )}
          >
            <UserPlus className="w-4 h-4 inline mr-2" />
            Sign Up
          </button>
        </div>

        <form onSubmit={handleSubmit} className="space-y-4">
          {/* Homeserver */}
          <div className="space-y-2">
            <Label htmlFor="homeserver">Homeserver</Label>
            <div className="relative">
              <Server className="w-5 h-5 text-text-muted absolute left-3 top-1/2 transform -translate-y-1/2 z-10" />
              <Input
                id="homeserver"
                type="url"
                value={formData.homeserver}
                onChange={(e) => handleInputChange('homeserver', e.target.value)}
                className="pl-10"
                placeholder="https://matrix.org"
                required
              />
            </div>
          </div>

          {/* Username */}
          <div className="space-y-2">
            <Label htmlFor="username">Username</Label>
            <div className="relative">
              <User className="w-5 h-5 text-text-muted absolute left-3 top-1/2 transform -translate-y-1/2 z-10" />
              <Input
                id="username"
                type="text"
                value={formData.username}
                onChange={(e) => handleInputChange('username', e.target.value)}
                className="pl-10"
                placeholder="@username:matrix.org"
                required
              />
            </div>
          </div>

          {/* Password */}
          <div className="space-y-2">
            <Label htmlFor="password">Password</Label>
            <div className="relative">
              <Lock className="w-5 h-5 text-text-muted absolute left-3 top-1/2 transform -translate-y-1/2 z-10" />
              <Input
                id="password"
                type="password"
                value={formData.password}
                onChange={(e) => handleInputChange('password', e.target.value)}
                className="pl-10"
                placeholder="••••••••"
                required
              />
            </div>
            {mode === 'register' && (
              <p className="text-xs text-text-muted mt-1">
                Password must be at least 12 characters long
              </p>
            )}
          </div>

          {/* Error Message */}
          {error && (
            <div className="bg-background-danger/10 border border-border-danger rounded-lg p-3">
              <p className="text-text-danger text-sm">{error}</p>
            </div>
          )}

          {/* Submit Button */}
          <Button
            type="submit"
            disabled={isLoading}
            className="w-full"
          >
            {isLoading ? (
              <div className="flex items-center justify-center">
                <div className="w-5 h-5 border-2 border-text-on-accent border-t-transparent rounded-full animate-spin mr-2" />
                Connecting...
              </div>
            ) : mode === 'login' ? (
              'Sign In'
            ) : (
              'Create Account'
            )}
          </Button>
        </form>

        <div className="mt-6 pt-4 border-t border-border-default">
          <Button
            onClick={onClose}
            variant="ghost"
            className="w-full"
          >
            Cancel
          </Button>
        </div>

        {/* Info */}
        <div className="mt-4 text-xs text-text-muted text-center">
          <p>
            Matrix is a secure, decentralized protocol for real-time communication.
            Your messages are end-to-end encrypted.
          </p>
        </div>
      </motion.div>
    </motion.div>
  );
};

export default MatrixAuth;
