import React, { useState, useCallback, useEffect } from 'react';
import TrainingDataTable from './TrainingDataTable';
import { motion } from 'framer-motion';
import { 
  Upload, 
  Play, 
  Pause, 
  BarChart3, 
  Settings, 
  Download,
  AlertCircle,
  CheckCircle,
  Clock,
  Zap,
  Server,
  HardDrive,
  Wifi,
  WifiOff,
  RefreshCw,
  Plus,
  ExternalLink,
  Wrench,
  ThumbsUp,
  MessageSquare
} from 'lucide-react';
import AxolotlConfig from './AxolotlConfig';
import { useNavigate } from 'react-router-dom';

interface TuningJob {
  id: string;
  name: string;
  status: 'pending' | 'running' | 'completed' | 'failed' | 'paused';
  progress: number;
  baseModel: string;
  datasetSize: number;
  startTime: Date;
  estimatedCompletion?: Date;
  metrics?: {
    loss: number;
    accuracy: number;
    perplexity: number;
  };
}

interface Dataset {
  id: string;
  name: string;
  size: number;
  format: 'jsonl' | 'csv' | 'txt';
  uploadDate: Date;
  validated: boolean;
}

interface LocalModel {
  id: string;
  name: string;
  size: string;
  type: 'ollama' | 'huggingface' | 'custom';
  status: 'available' | 'downloading' | 'error';
  lastUsed?: Date;
  tags?: string[];
  description?: string;
}

interface ModelProvider {
  id: string;
  name: string;
  type: 'ollama' | 'huggingface' | 'openai' | 'custom';
  endpoint: string;
  status: 'connected' | 'disconnected' | 'error';
  models: LocalModel[];
}

export default function LLMTuningSection() {
  const navigate = useNavigate();
  const [activeTab, setActiveTab] = useState<'datasets' | 'jobs' | 'models'>('datasets');
  const [backendUrl, setBackendUrl] = useState<string>('http://localhost:3000');
  const [secretKey, setSecretKey] = useState<string>('');
  const [datasets, setDatasets] = useState<Dataset[]>([]);
  const [feedbackCount, setFeedbackCount] = useState<number>(0);
  const [loadingFeedback, setLoadingFeedback] = useState(false);

  const [tuningJobs, setTuningJobs] = useState<TuningJob[]>([]);

  const [modelProviders, setModelProviders] = useState<ModelProvider[]>([
    {
      id: 'ollama',
      name: 'Ollama',
      type: 'ollama',
      endpoint: 'http://localhost:11434',
      status: 'connected',
      models: [
        {
          id: 'llama3.2:3b',
          name: 'Llama 3.2 3B',
          size: '2.0GB',
          type: 'ollama',
          status: 'available',
          lastUsed: new Date('2024-01-14'),
          tags: ['chat', 'code'],
          description: 'Fast and efficient 3B parameter model'
        },
        {
          id: 'mistral:7b',
          name: 'Mistral 7B',
          size: '4.1GB',
          type: 'ollama',
          status: 'available',
          lastUsed: new Date('2024-01-13'),
          tags: ['chat', 'instruct'],
          description: 'High-quality 7B parameter instruction model'
        },
        {
          id: 'codellama:13b',
          name: 'Code Llama 13B',
          size: '7.3GB',
          type: 'ollama',
          status: 'downloading',
          tags: ['code', 'programming'],
          description: 'Specialized code generation model'
        }
      ]
    }
  ]);

  const [isRefreshing, setIsRefreshing] = useState(false);
  const [showAxolotlConfig, setShowAxolotlConfig] = useState(false);
  const [axolotlInstalled, setAxolotlInstalled] = useState<boolean | null>(null);
  const [checkingAxolotl, setCheckingAxolotl] = useState(false);
  const [installingAxolotl, setInstallingAxolotl] = useState(false);
  const [installLog, setInstallLog] = useState<string | null>(null);

  // Function to fetch feedback count
  const fetchFeedbackCount = useCallback(async () => {
    if (!backendUrl || !secretKey) {
      console.warn('Cannot fetch feedback: missing backendUrl or secretKey', { backendUrl, secretKey });
      return;
    }
    
    setLoadingFeedback(true);
    console.log('Fetching feedback from:', `${backendUrl}/training/examples`);
    try {
      const res = await fetch(`${backendUrl}/training/examples`, {
        headers: {
          'X-Secret-Key': secretKey,
        },
      });
      console.log('Feedback response status:', res.status);
      if (res.ok) {
        const data = await res.json();
        console.log('Feedback data received:', data);
        setFeedbackCount(data.count || 0);
        console.log('Feedback count updated:', data.count);
      } else {
        const text = await res.text();
        console.warn('Failed to fetch feedback data:', res.status, res.statusText, text);
      }
    } catch (e) {
      console.error('Failed to fetch feedback data:', e);
    } finally {
      setLoadingFeedback(false);
    }
  }, [backendUrl, secretKey]);

  // Get backend URL and secret key on mount
  useEffect(() => {
    const init = async () => {
      try {
        const hostPort = await window.electron.getGoosedHostPort();
        const key = await window.electron.getSecretKey();
        
        if (hostPort) {
          // hostPort already includes http://, so use it directly
          setBackendUrl(hostPort);
          console.log('Backend URL set to:', hostPort);
        }
        if (key) {
          setSecretKey(key);
          console.log('Secret key set');
        }
      } catch (e) {
        console.warn('Failed to get backend config:', e);
      }
    };
    init();
  }, []);

  // Fetch feedback count when backend URL and secret key are available
  useEffect(() => {
    if (backendUrl && secretKey) {
      fetchFeedbackCount();
    }
  }, [backendUrl, secretKey, fetchFeedbackCount]);

  // Refresh feedback count when switching to datasets tab
  useEffect(() => {
    if (activeTab === 'datasets' && backendUrl && secretKey) {
      fetchFeedbackCount();
    }
  }, [activeTab, backendUrl, secretKey, fetchFeedbackCount]);

  // Check if Axolotl is installed when switching to jobs tab
  const checkAxolotlInstalled = useCallback(async () => {
    setCheckingAxolotl(true);
    try {
      // Try to check if python can import axolotl
      const res = await fetch(`${backendUrl}/training/check-axolotl`, {
        headers: { 'X-Secret-Key': secretKey }
      });
      if (res.ok) {
        const data = await res.json();
        setAxolotlInstalled(data.installed === true);
      } else {
        // If endpoint doesn't exist, assume not installed
        setAxolotlInstalled(false);
      }
    } catch (e) {
      console.warn('Failed to check Axolotl status:', e);
      setAxolotlInstalled(false);
    } finally {
      setCheckingAxolotl(false);
    }
  }, [backendUrl, secretKey]);

  // Install Axolotl handler
  const handleInstallAxolotl = useCallback(async () => {
    setInstallingAxolotl(true);
    setInstallLog(null);
    try {
      const res = await fetch(`${backendUrl}/training/install-axolotl`, {
        method: 'POST',
        headers: { 'X-Secret-Key': secretKey }
      });
      
      if (res.ok) {
        const data = await res.json();
        setInstallLog(data.log || null);
        
        if (data.success) {
          alert('✅ ' + data.message);
          setAxolotlInstalled(true);
        } else {
          alert('❌ ' + data.message);
        }
      } else {
        alert('❌ Failed to install Axolotl. Check the console for details.');
      }
    } catch (e) {
      console.error('Failed to install Axolotl:', e);
      alert('❌ Failed to install Axolotl: ' + (e as Error).message);
    } finally {
      setInstallingAxolotl(false);
    }
  }, [backendUrl, secretKey]);

  useEffect(() => {
    if (activeTab === 'jobs' && backendUrl && secretKey && axolotlInstalled === null) {
      checkAxolotlInstalled();
    }
  }, [activeTab, backendUrl, secretKey, axolotlInstalled, checkAxolotlInstalled]);

  // Fetch training jobs when switching to jobs or models tab
  const fetchTrainingJobs = useCallback(async () => {
    if (!backendUrl || !secretKey) return;
    
    try {
      const res = await fetch(`${backendUrl}/training/jobs`, {
        headers: { 'X-Secret-Key': secretKey }
      });
      
      if (res.ok) {
        const data = await res.json();
        const jobs = data.jobs || [];
        
        // Convert backend job format to UI format
        const uiJobs: TuningJob[] = jobs.map((job: any) => ({
          id: job.id,
          name: job.name || `Training Job ${job.id.slice(0, 8)}`,
          status: job.status.toLowerCase(),
          progress: job.status === 'completed' ? 100 : job.status === 'failed' ? 0 : 50,
          baseModel: job.base_model_path || 'Unknown',
          datasetSize: 0, // Not provided by backend
          startTime: job.created_at ? new Date(job.created_at) : new Date(),
        }));
        
        setTuningJobs(uiJobs);
      }
    } catch (e) {
      console.error('Failed to fetch training jobs:', e);
    }
  }, [backendUrl, secretKey]);

  // Fetch jobs when switching to jobs or models tab
  useEffect(() => {
    if ((activeTab === 'jobs' || activeTab === 'models') && backendUrl && secretKey) {
      fetchTrainingJobs();
    }
  }, [activeTab, backendUrl, secretKey, fetchTrainingJobs]);

  const handleFileUpload = useCallback((event: React.ChangeEvent<HTMLInputElement>) => {
    const files = event.target.files;
    if (!files) return;

    Array.from(files).forEach((file) => {
      const newDataset: Dataset = {
        id: Date.now().toString(),
        name: file.name,
        size: Math.floor(Math.random() * 20000) + 1000, // Mock size
        format: file.name.endsWith('.jsonl') ? 'jsonl' : file.name.endsWith('.csv') ? 'csv' : 'txt',
        uploadDate: new Date(),
        validated: false,
      };
      setDatasets(prev => [...prev, newDataset]);
    });
  }, []);

  const getStatusIcon = (status: TuningJob['status']) => {
    switch (status) {
      case 'running':
        return <Clock className="w-4 h-4 text-blue-500 animate-spin" />;
      case 'completed':
        return <CheckCircle className="w-4 h-4 text-green-500" />;
      case 'failed':
        return <AlertCircle className="w-4 h-4 text-red-500" />;
      case 'paused':
        return <Pause className="w-4 h-4 text-yellow-500" />;
      default:
        return <Clock className="w-4 h-4 text-gray-500" />;
    }
  };

  const getStatusColor = (status: TuningJob['status']) => {
    switch (status) {
      case 'running':
        return 'text-blue-500 bg-blue-50 dark:bg-blue-900/20';
      case 'completed':
        return 'text-green-500 bg-green-50 dark:bg-green-900/20';
      case 'failed':
        return 'text-red-500 bg-red-50 dark:bg-red-900/20';
      case 'paused':
        return 'text-yellow-500 bg-yellow-50 dark:bg-yellow-900/20';
      default:
        return 'text-gray-500 bg-gray-50 dark:bg-gray-900/20';
    }
  };

  const getModelStatusIcon = (status: LocalModel['status']) => {
    switch (status) {
      case 'available':
        return <CheckCircle className="w-4 h-4 text-green-500" />;
      case 'downloading':
        return <Download className="w-4 h-4 text-blue-500 animate-pulse" />;
      case 'error':
        return <AlertCircle className="w-4 h-4 text-red-500" />;
      default:
        return <Clock className="w-4 h-4 text-gray-500" />;
    }
  };

  const getProviderStatusIcon = (status: ModelProvider['status']) => {
    switch (status) {
      case 'connected':
        return <Wifi className="w-4 h-4 text-green-500" />;
      case 'disconnected':
        return <WifiOff className="w-4 h-4 text-red-500" />;
      case 'error':
        return <AlertCircle className="w-4 h-4 text-red-500" />;
      default:
        return <Clock className="w-4 h-4 text-gray-500" />;
    }
  };

  const refreshModels = useCallback(async () => {
    setIsRefreshing(true);
    // Simulate API call to refresh models
    await new Promise(resolve => setTimeout(resolve, 1000));
    setIsRefreshing(false);
  }, []);

  const handleStartAxolotlTraining = useCallback(async (config: any) => {
    try {
      // Map UI config to server request
      const base_model_path = config.local_base_model_path || config.base_model;
      const body = {
        base_model_path,
        priority: 'normal',
        config_overrides: {
          finetune_method: config.adapter?.toLowerCase() === 'qlora' ? 'QLoRA' : 'LoRA',
          preference_method: 'None',
          rl_method: 'None',
          reward_method: 'None',
          quantization: 'None',
          learning_rate: config.learning_rate || 0.0002,
          batch_size: config.micro_batch_size || 2,
          num_epochs: config.num_epochs || 3,
          warmup_steps: 10,
          max_seq_length: config.sequence_len || 2048,
          gradient_accumulation_steps: config.gradient_accumulation_steps || 1,
          weight_decay: 0.0,
          lora_config: {
            rank: config.lora_r || 16,
            alpha: config.lora_alpha || 32,
            dropout: config.lora_dropout || 0.05,
            target_modules: config.lora_target_modules || ['q_proj','k_proj','v_proj','o_proj','gate_proj','up_proj','down_proj'],
            modules_to_save: [],
            bias: 'none',
            task_type: 'CAUSAL_LM'
          },
          save_steps: 50,
          eval_steps: 50,
          logging_steps: 10,
          early_stopping_patience: 3,
          mixed_precision: true,
        }
      };

      const res = await fetch(`${backendUrl}/training/start`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', 'X-Secret-Key': secretKey },
        body: JSON.stringify(body)
      });
      if (!res.ok) throw new Error(`Failed to start training: ${res.status}`);
      const data = await res.json();
      const jobId = data.job_id as string;

      // Add job locally and switch tab
      const newJob: TuningJob = {
        id: jobId,
        name: `${config.base_model} Fine-tune`,
        status: 'running',
        progress: 0,
        baseModel: config.base_model,
        datasetSize: config.datasets?.length ? config.datasets.length * 1000 : 0,
        startTime: new Date(),
      };
      setTuningJobs(prev => [newJob, ...prev]);
      setShowAxolotlConfig(false);
      setActiveTab('jobs');

      // Start polling progress
      const poll = async () => {
        try {
          const pr = await fetch(`${backendUrl}/training/progress/${jobId}`, {
            headers: { 'X-Secret-Key': secretKey }
          });
          if (pr.ok) {
            const json = await pr.json();
            const updates = json.updates || [];
            if (updates.length) {
              // Simple progress heuristic: last step mod 100
              const last = updates[updates.length - 1];
              const step = last.step || 0;
              setTuningJobs(prev => prev.map(j => j.id === jobId ? { ...j, progress: Math.min(100, step % 100) } : j));
            }
          }
        } catch (e) { /* ignore */ }
        setTimeout(poll, 1000);
      };
      poll();
    } catch (e) {
      console.error(e);
      alert((e as Error).message);
    }
  }, [backendUrl]);

  const handleActivateAdapter = useCallback(async (jobId: string) => {
    try {
      const res = await fetch(`${backendUrl}/training/activate`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', 'X-Secret-Key': secretKey },
        body: JSON.stringify({ job_id: jobId })
      });
      
      if (!res.ok) throw new Error(`Failed to activate adapter: ${res.status}`);
      
      alert(`✅ Adapter activated! Restart Distil to use the tuned model.`);
    } catch (e) {
      console.error(e);
      alert(`❌ Failed to activate adapter: ${(e as Error).message}`);
    }
  }, [backendUrl, secretKey]);

  const handleChatWithModel = useCallback((job: TuningJob) => {
    // Navigate to chat with adapter info in state
    // The adapter path will be: ~/.config/goose/training/job-{id}/adapter_model.safetensors
    const home = window.electron.platform === 'darwin' ? '~' : process.env.HOME || '~';
    const adapterPath = `${home}/.config/goose/training/job-${job.id}`;
    
    console.log('Opening chat with fine-tuned model:', {
      jobId: job.id,
      jobName: job.name,
      baseModel: job.baseModel,
      adapterPath
    });
    
    // Navigate to chat with initial message about the model
    navigate('/pair', {
      state: {
        fineTunedModel: {
          jobId: job.id,
          name: job.name,
          baseModel: job.baseModel,
          adapterPath
        },
        initialMessage: `Using fine-tuned model: ${job.name}`
      }
    });
  }, [navigate]);

  const handleQuickTrainWithFeedback = useCallback(async () => {
    if (feedbackCount === 0) {
      alert('No feedback collected yet. Submit some feedback first!');
      return;
    }

    try {
      // Use the current base model from modelProviders
      const baseModel = modelProviders[0]?.models.find(m => m.status === 'available')?.id || 'qwen2.5:7b-instruct';
      
      // Create a training job with complete config (all fields required)
      const timestamp = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19);
      const body = {
        base_model_path: baseModel,
        priority: 'normal',
        min_quality_score: 0.0,  // Accept all feedback
        max_examples: 10000,
        config_overrides: {
          learning_rate: 0.0002,
          batch_size: 2,
          num_epochs: 3,
          warmup_steps: 10,
          max_seq_length: 2048,
          gradient_accumulation_steps: 1,
          weight_decay: 0.0,
          lora_config: {
            rank: 16,
            alpha: 32.0,
            dropout: 0.05,
            target_modules: ['q_proj', 'k_proj', 'v_proj', 'o_proj', 'gate_proj', 'up_proj', 'down_proj'],
            modules_to_save: [],
            bias: 'none',
            task_type: 'CAUSAL_LM'
          },
          save_steps: 50,
          eval_steps: 50,
          logging_steps: 10,
          early_stopping_patience: 3,
          mixed_precision: true,
          finetune_method: 'LoRA',
          preference_method: 'None',
          rl_method: 'None',
          reward_method: 'None',
          quantization: 'None',
        }
      };

      console.log('Starting training with feedback:', body);

      const res = await fetch(`${backendUrl}/training/start`, {
        method: 'POST',
        headers: { 
          'Content-Type': 'application/json',
          'X-Secret-Key': secretKey,
        },
        body: JSON.stringify(body)
      });

      if (!res.ok) {
        const errorText = await res.text();
        throw new Error(`Failed to start training: ${res.status} - ${errorText}`);
      }

      const data = await res.json();
      const jobId = data.job_id as string;

      // Add job locally and switch to jobs tab
      const newJob: TuningJob = {
        id: jobId,
        name: `Feedback Fine-tune ${timestamp}`,
        status: 'running',
        progress: 0,
        baseModel: baseModel,
        datasetSize: feedbackCount,
        startTime: new Date(),
      };
      setTuningJobs(prev => [newJob, ...prev]);
      setActiveTab('jobs');

      // Start polling progress
      const poll = async () => {
        try {
          const pr = await fetch(`${backendUrl}/training/progress/${jobId}`, {
            headers: { 'X-Secret-Key': secretKey }
          });
          if (pr.ok) {
            const json = await pr.json();
            const updates = json.updates || [];
            if (updates.length) {
              const last = updates[updates.length - 1];
              const step = last.step || 0;
              setTuningJobs(prev => prev.map(j => 
                j.id === jobId ? { ...j, progress: Math.min(100, step % 100) } : j
              ));
            }
          }
        } catch (e) { 
          console.warn('Failed to poll progress:', e);
        }
        setTimeout(poll, 2000);
      };
      poll();

      alert(`✅ Training job started with ${feedbackCount} feedback examples!`);
    } catch (e) {
      console.error('Failed to start training:', e);
      alert(`❌ Failed to start training: ${(e as Error).message}`);
    }
  }, [backendUrl, secretKey, feedbackCount, modelProviders, setActiveTab, setTuningJobs]);

  // Get available models for Axolotl config
  const availableModels = modelProviders
    .flatMap(provider => provider.models)
    .filter(model => model.status === 'available')
    .map(model => model.id);

  // Get available datasets for Axolotl config
  const availableDatasets = datasets
    .filter(dataset => dataset.validated)
    .map(dataset => dataset.name);

  // If showing Axolotl config, render that instead
  if (showAxolotlConfig) {
    return (
      <AxolotlConfig
        onStartTraining={handleStartAxolotlTraining}
        availableModels={availableModels}
        datasets={availableDatasets}
      />
    );
  }

  return (
    <div className="space-y-8">
      {/* Header */}
      <div>
        <h1 className="text-3xl font-light text-text-default mb-2">LLM Tuning</h1>
        <p className="text-text-muted">
          Fine-tune language models with your own datasets for specialized tasks.
        </p>
      </div>

      {/* Tab Navigation */}
      <div className="flex space-x-1 bg-background-muted p-1 rounded-lg w-fit">
        {[
          { id: 'datasets', label: 'Datasets', icon: Upload },
          { id: 'jobs', label: 'Tuning Jobs', icon: Zap },
          { id: 'models', label: 'Fine-tuned Models', icon: BarChart3 },
        ].map(({ id, label, icon: Icon }) => (
          <button
            key={id}
            onClick={() => setActiveTab(id as any)}
            className={`
              flex items-center gap-2 px-4 py-2 rounded-md text-sm font-medium transition-colors
              ${activeTab === id 
                ? 'bg-background-accent text-text-on-accent' 
                : 'text-text-muted hover:text-text-default hover:bg-background-subtle'
              }
            `}
          >
            <Icon className="w-4 h-4" />
            {label}
          </button>
        ))}
      </div>

      {/* Content */}
      <div className="space-y-6">
        {activeTab === 'datasets' && (
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            className="space-y-6"
          >

            {/* Training Data Section with Feedback Summary */}
            <div className="space-y-4">
              {/* Feedback collection summary and quick-train */}
              <div className="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-3 p-4 bg-background-muted border border-border-default rounded-lg">
                <div className="flex items-center gap-3">
                  <div className="w-9 h-9 rounded-full bg-background-accent/10 flex items-center justify-center">
                    <ThumbsUp className="w-5 h-5 text-text-default" />
                  </div>
                  <div>
                    <div className="text-sm text-text-muted">Collected Feedback</div>
                    <div className="flex items-center gap-3">
                      <div className="text-lg font-semibold text-text-default">{feedbackCount}</div>
                      <div className="text-xs text-text-muted">examples</div>
                      <button
                        onClick={fetchFeedbackCount}
                        disabled={loadingFeedback}
                        className="inline-flex items-center gap-1 px-2 py-1 text-xs rounded-md hover:bg-background-subtle transition-colors disabled:opacity-50"
                        title="Refresh feedback count"
                      >
                        <RefreshCw className={`w-3.5 h-3.5 ${loadingFeedback ? 'animate-spin' : ''}`} />
                        Refresh
                      </button>
                    </div>
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  <button
                    onClick={handleQuickTrainWithFeedback}
                    disabled={feedbackCount === 0}
                    className="inline-flex items-center gap-2 px-3 py-2 bg-background-accent text-text-on-accent rounded-md hover:bg-background-accent/90 transition-colors disabled:opacity-50"
                  >
                    <Zap className="w-4 h-4" />
                    Start Training with Feedback
                  </button>
                </div>
              </div>

              {/* Training Data Table */}
              <div>
                {/* @ts-ignore */}
                <TrainingDataTable backendUrl={backendUrl} secretKey={secretKey} />
              </div>
            </div>
          </motion.div>
        )}

        {activeTab === 'jobs' && (
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            className="space-y-6"
          >
            {/* Create New Job Buttons */}
            <div className="flex items-center gap-3">
              <button 
                onClick={() => setShowAxolotlConfig(true)}
                className="flex items-center gap-2 px-4 py-2 bg-background-accent text-text-on-accent rounded-lg hover:bg-background-accent/90 transition-colors"
              >
                <Wrench className="w-4 h-4" />
                Configure Axolotl Training
              </button>
              <button className="flex items-center gap-2 px-4 py-2 bg-background-subtle text-text-default rounded-lg hover:bg-background-medium transition-colors">
                <Play className="w-4 h-4" />
                Quick Start
              </button>
            </div>

            {/* Axolotl Installation Warning */}
            {axolotlInstalled === false && (
              <div className="p-6 bg-gradient-to-br from-orange-50 to-red-50 dark:from-orange-900/20 dark:to-red-900/20 rounded-lg border-2 border-orange-300 dark:border-orange-700">
                <div className="flex items-start gap-4">
                  <AlertCircle className="w-6 h-6 text-orange-600 dark:text-orange-400 flex-shrink-0 mt-1" />
                  <div className="flex-1">
                    <h3 className="text-lg font-semibold text-text-default mb-2">Axolotl Not Installed</h3>
                    <p className="text-sm text-text-muted mb-4">
                      Training requires Axolotl to be installed. Click the button below to install it automatically, or follow the manual steps:
                    </p>
                    <div className="bg-background-default/50 rounded-lg p-4 mb-4 font-mono text-sm">
                      <div className="text-text-muted mb-2"># Manual installation (if automatic fails):</div>
                      <div className="text-text-default mb-2">python3 -m pip cache purge</div>
                      <div className="text-text-default mb-2">python3 -m venv ~/.config/goose/axolotl-venv</div>
                      <div className="text-text-default mb-2">source ~/.config/goose/axolotl-venv/bin/activate</div>
                      <div className="text-text-default">pip install --no-cache-dir axolotl accelerate</div>
                    </div>
                    <div className="flex items-center gap-3 flex-wrap">
                      <button
                        onClick={handleInstallAxolotl}
                        disabled={installingAxolotl}
                        className="flex items-center gap-2 px-4 py-2 bg-green-600 hover:bg-green-700 text-white rounded-lg transition-colors disabled:opacity-50 font-medium"
                      >
                        <Download className={`w-4 h-4 ${installingAxolotl ? 'animate-bounce' : ''}`} />
                        {installingAxolotl ? 'Installing...' : 'Install Axolotl'}
                      </button>
                      <button
                        onClick={checkAxolotlInstalled}
                        disabled={checkingAxolotl || installingAxolotl}
                        className="flex items-center gap-2 px-4 py-2 bg-orange-600 hover:bg-orange-700 text-white rounded-lg transition-colors disabled:opacity-50"
                      >
                        <RefreshCw className={`w-4 h-4 ${checkingAxolotl ? 'animate-spin' : ''}`} />
                        Check Again
                      </button>
                      <a
                        href="https://github.com/OpenAccess-AI-Collective/axolotl#installation"
                        target="_blank"
                        rel="noopener noreferrer"
                        className="flex items-center gap-2 px-4 py-2 bg-background-subtle hover:bg-background-medium text-text-default rounded-lg transition-colors"
                      >
                        <ExternalLink className="w-4 h-4" />
                        Documentation
                      </a>
                    </div>
                    {installingAxolotl && (
                      <div className="mt-4 p-3 bg-blue-50 dark:bg-blue-900/20 rounded-lg">
                        <p className="text-sm text-blue-600 dark:text-blue-400">
                          Installing Axolotl... This may take 5-10 minutes. Please wait.
                        </p>
                      </div>
                    )}
                    {installLog && (
                      <details className="mt-4">
                        <summary className="cursor-pointer text-sm text-text-muted hover:text-text-default">
                          View Installation Log
                        </summary>
                        <pre className="mt-2 p-3 bg-background-default/50 rounded text-xs overflow-auto max-h-64">
                          {installLog}
                        </pre>
                      </details>
                    )}
                  </div>
                </div>
              </div>
            )}

            {checkingAxolotl && axolotlInstalled === null && (
              <div className="p-4 bg-background-muted rounded-lg flex items-center gap-3">
                <RefreshCw className="w-5 h-5 text-text-muted animate-spin" />
                <span className="text-text-muted">Checking if Axolotl is installed...</span>
              </div>
            )}

            {/* Jobs List */}
            <div className="space-y-4">
              {tuningJobs.length === 0 ? (
                <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-0.5">
                  {/* Action tile: Start from feedback */}
                  <motion.div
                    initial={{ opacity: 0, y: 12, scale: 0.98 }}
                    animate={{ opacity: 1, y: 0, scale: 1 }}
                    whileHover={{ scale: 1.02 }}
                    whileTap={{ scale: 0.98 }}
                    onClick={() => setActiveTab('datasets')}
                    className="relative cursor-pointer group bg-background-default px-6 py-6 transition-colors duration-200 hover:bg-background-medium aspect-square flex flex-col items-center justify-center rounded-2xl"
                  >
                    <div className="w-12 h-12 bg-background-accent rounded-full flex items-center justify-center mb-3">
                      <ThumbsUp className="w-6 h-6 text-text-on-accent" />
                    </div>
                    <p className="text-sm font-medium text-text-default text-center">Start from Collected Feedback</p>
                    <p className="text-xs text-text-muted text-center mt-1">Use conversation ratings and corrections</p>
                  </motion.div>

                  {/* Action tile: Configure Axolotl */}
                  <motion.div
                    initial={{ opacity: 0, y: 12, scale: 0.98 }}
                    animate={{ opacity: 1, y: 0, scale: 1 }}
                    whileHover={{ scale: 1.02 }}
                    whileTap={{ scale: 0.98 }}
                    onClick={() => setShowAxolotlConfig(true)}
                    className="relative cursor-pointer group bg-background-default px-6 py-6 transition-colors duration-200 hover:bg-background-medium aspect-square flex flex-col items-center justify-center rounded-2xl"
                  >
                    <div className="w-12 h-12 bg-background-accent rounded-full flex items-center justify-center mb-3">
                      <Wrench className="w-6 h-6 text-text-on-accent" />
                    </div>
                    <p className="text-sm font-medium text-text-default text-center">Configure Axolotl Training</p>
                    <p className="text-xs text-text-muted text-center mt-1">Advanced options and datasets</p>
                  </motion.div>

                  {/* Filler tiles to create a tiled empty state */}
                  {Array.from({ length: 10 }).map((_, i) => (
                    <motion.div
                      key={`empty-job-${i}`}
                      initial={{ opacity: 0, y: 12, scale: 0.98 }}
                      animate={{ opacity: 1, y: 0, scale: 1 }}
                      whileHover={{ scale: 1.02 }}
                      whileTap={{ scale: 0.98 }}
                      className="relative group bg-background-default px-6 py-6 transition-all duration-200 hover:bg-background-medium aspect-square flex items-center justify-center rounded-2xl"
                    >
                      <div className="w-8 h-8 rounded-full border-2 border-dashed border-text-muted/30 flex items-center justify-center">
                        <div className="w-1 h-1 bg-text-muted/30 rounded-full" />
                      </div>
                    </motion.div>
                  ))}
                </div>
              ) : (
                tuningJobs.map((job) => (
                  <div
                    key={job.id}
                    className="p-6 bg-background-muted rounded-lg space-y-4"
                  >
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-3">
                        {getStatusIcon(job.status)}
                        <div>
                          <h4 className="font-medium text-text-default">{job.name}</h4>
                          <p className="text-sm text-text-muted">
                            Base: {job.baseModel} • {job.datasetSize.toLocaleString()} examples
                          </p>
                        </div>
                      </div>
                      <span className={`px-3 py-1 rounded-full text-xs font-medium ${getStatusColor(job.status)}`}>
                        {job.status.toUpperCase()}
                      </span>
                    </div>

                    {/* Progress Bar */}
                    <div className="space-y-2">
                      <div className="flex justify-between text-sm">
                        <span className="text-text-muted">Progress</span>
                        <span className="text-text-default">{job.progress}%</span>
                      </div>
                      <div className="w-full bg-background-subtle rounded-full h-2">
                        <div
                          className="bg-background-accent h-2 rounded-full transition-all duration-300"
                          style={{ width: `${job.progress}%` }}
                        />
                      </div>
                    </div>

                    {/* Metrics */}
                    {job.metrics && (
                      <div className="grid grid-cols-3 gap-4 pt-2 border-t border-border-default">
                        <div className="text-center">
                          <div className="text-lg font-semibold text-text-default">{job.metrics.loss.toFixed(3)}</div>
                          <div className="text-xs text-text-muted">Loss</div>
                        </div>
                        <div className="text-center">
                          <div className="text-lg font-semibold text-text-default">{(job.metrics.accuracy * 100).toFixed(1)}%</div>
                          <div className="text-xs text-text-muted">Accuracy</div>
                        </div>
                        <div className="text-center">
                          <div className="text-lg font-semibold text-text-default">{job.metrics.perplexity.toFixed(1)}</div>
                          <div className="text-xs text-text-muted">Perplexity</div>
                        </div>
                      </div>
                    )}

                    {/* Actions for completed jobs */}
                    {job.status === 'completed' && (
                      <div className="flex items-center gap-2 pt-2 border-t border-border-default">
                        <button
                          onClick={() => handleActivateAdapter(job.id)}
                          className="flex items-center gap-2 px-4 py-2 bg-background-accent text-text-on-accent rounded-lg hover:bg-background-accent/90 transition-colors"
                        >
                          <Zap className="w-4 h-4" />
                          Activate Adapter
                        </button>
                        <span className="text-xs text-text-muted">
                          Use this tuned model for inference
                        </span>
                      </div>
                    )}
                  </div>
                ))
              )}
            </div>
          </motion.div>
        )}

        {activeTab === 'models' && (
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            className="space-y-6"
          >
            {/* Header Actions */}
            <div className="flex items-center justify-between">
              <h3 className="text-lg font-medium text-text-default">Fine-tuned Models</h3>
              <div className="flex items-center gap-2">
                <button
                  onClick={fetchTrainingJobs}
                  disabled={isRefreshing}
                  className="flex items-center gap-2 px-3 py-2 text-sm bg-background-subtle hover:bg-background-medium rounded-lg transition-colors disabled:opacity-50"
                >
                  <RefreshCw className={`w-4 h-4 ${isRefreshing ? 'animate-spin' : ''}`} />
                  Refresh
                </button>
              </div>
            </div>

            {/* Completed Models Grid */}
            {(() => {
              const completedJobs = tuningJobs.filter(job => job.status === 'completed');
              
              if (completedJobs.length === 0) {
                return (
                  <div className="text-center py-12">
                    <BarChart3 className="w-16 h-16 text-text-muted mx-auto mb-4" />
                    <h3 className="text-lg font-medium text-text-default mb-2">No Fine-tuned Models Yet</h3>
                    <p className="text-text-muted mb-4">
                      Complete a training job to see your fine-tuned models here.
                    </p>
                    <button 
                      onClick={() => setActiveTab('datasets')}
                      className="flex items-center gap-2 px-4 py-2 bg-background-accent text-text-on-accent rounded-lg hover:bg-background-accent/90 transition-colors mx-auto"
                    >
                      <Zap className="w-4 h-4" />
                      Start Training
                    </button>
                  </div>
                );
              }

              return (
                <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                  {completedJobs.map((job) => (
                    <div
                      key={job.id}
                      className="p-6 bg-background-default border border-border-default rounded-lg hover:bg-background-subtle transition-colors"
                    >
                      <div className="flex items-start justify-between mb-4">
                        <div className="flex items-center gap-2">
                          <CheckCircle className="w-5 h-5 text-green-500" />
                          <span className="px-2 py-1 rounded text-xs font-medium bg-green-100 text-green-800 dark:bg-green-900/20 dark:text-green-400">
                            READY
                          </span>
                        </div>
                        <span className="text-xs text-text-muted">
                          {job.startTime.toLocaleDateString()}
                        </span>
                      </div>
                      
                      <h5 className="font-medium text-text-default mb-2">{job.name}</h5>
                      <p className="text-sm text-text-muted mb-4">
                        Base: {job.baseModel}
                      </p>
                      
                      {/* Tags */}
                      <div className="flex flex-wrap gap-1 mb-4">
                        <span className="px-2 py-1 text-xs bg-background-muted text-text-muted rounded">
                          LoRA
                        </span>
                        <span className="px-2 py-1 text-xs bg-background-muted text-text-muted rounded">
                          Fine-tuned
                        </span>
                      </div>
                      
                      {/* Actions */}
                      <div className="space-y-2">
                        <button
                          onClick={() => handleChatWithModel(job)}
                          className="w-full flex items-center justify-center gap-2 px-3 py-2 bg-background-accent text-text-on-accent rounded-lg hover:bg-background-accent/90 transition-colors text-sm font-medium"
                        >
                          <MessageSquare className="w-4 h-4" />
                          Chat with Model
                        </button>
                        <button
                          onClick={() => handleActivateAdapter(job.id)}
                          className="w-full flex items-center justify-center gap-2 px-3 py-2 bg-background-subtle text-text-default rounded-lg hover:bg-background-medium transition-colors text-sm font-medium"
                        >
                          <Zap className="w-4 h-4" />
                          Set as Default
                        </button>
                      </div>
                      
                      <div className="mt-3 pt-3 border-t border-border-default">
                        <div className="text-xs text-text-muted">
                          Job ID: {job.id.slice(0, 8)}...
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              );
            })()}

            {/* Model Providers - Keep for reference but hide for now */}
            <div className="hidden space-y-6">
              {modelProviders.map((provider) => (
                <div key={provider.id} className="space-y-4">
                  {/* Provider Header */}
                  <div className="flex items-center justify-between p-4 bg-background-muted rounded-lg">
                    <div className="flex items-center gap-3">
                      <Server className="w-5 h-5 text-text-muted" />
                      <div>
                        <h4 className="font-medium text-text-default flex items-center gap-2">
                          {provider.name}
                          {getProviderStatusIcon(provider.status)}
                        </h4>
                        <p className="text-sm text-text-muted">
                          {provider.endpoint} • {provider.models.length} models
                        </p>
                      </div>
                    </div>
                    <div className="flex items-center gap-2">
                      <span className={`px-2 py-1 rounded text-xs font-medium ${
                        provider.status === 'connected' 
                          ? 'bg-green-100 text-green-800 dark:bg-green-900/20 dark:text-green-400'
                          : provider.status === 'error'
                          ? 'bg-red-100 text-red-800 dark:bg-red-900/20 dark:text-red-400'
                          : 'bg-gray-100 text-gray-800 dark:bg-gray-900/20 dark:text-gray-400'
                      }`}>
                        {provider.status.toUpperCase()}
                      </span>
                      <button className="p-2 hover:bg-background-subtle rounded">
                        <Settings className="w-4 h-4 text-text-muted" />
                      </button>
                    </div>
                  </div>

                  {/* Models Grid */}
                  <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                    {provider.models.map((model) => (
                      <div
                        key={model.id}
                        className="p-4 bg-background-default border border-border-default rounded-lg hover:bg-background-subtle transition-colors"
                      >
                        <div className="flex items-start justify-between mb-3">
                          <div className="flex items-center gap-2">
                            <HardDrive className="w-4 h-4 text-text-muted" />
                            {getModelStatusIcon(model.status)}
                          </div>
                          <span className="text-xs text-text-muted">{model.size}</span>
                        </div>
                        
                        <h5 className="font-medium text-text-default mb-1">{model.name}</h5>
                        <p className="text-sm text-text-muted mb-3 line-clamp-2">
                          {model.description}
                        </p>
                        
                        {/* Tags */}
                        {model.tags && (
                          <div className="flex flex-wrap gap-1 mb-3">
                            {model.tags.map((tag) => (
                              <span
                                key={tag}
                                className="px-2 py-1 text-xs bg-background-muted text-text-muted rounded"
                              >
                                {tag}
                              </span>
                            ))}
                          </div>
                        )}
                        
                        {/* Actions */}
                        <div className="flex items-center justify-between">
                          {model.lastUsed && (
                            <span className="text-xs text-text-muted">
                              Used {model.lastUsed.toLocaleDateString()}
                            </span>
                          )}
                          <div className="flex items-center gap-1">
                            {model.status === 'available' && (
                              <button className="px-2 py-1 text-xs bg-background-accent text-text-on-accent rounded hover:bg-background-accent/90 transition-colors">
                                Fine-tune
                              </button>
                            )}
                            {model.status === 'downloading' && (
                              <button className="px-2 py-1 text-xs bg-blue-100 text-blue-800 dark:bg-blue-900/20 dark:text-blue-400 rounded">
                                Downloading...
                              </button>
                            )}
                            <button className="p-1 hover:bg-background-muted rounded">
                              <ExternalLink className="w-3 h-3 text-text-muted" />
                            </button>
                          </div>
                        </div>
                      </div>
                    ))}
                  </div>

                  {/* Add Model Button */}
                  <button className="w-full p-4 border-2 border-dashed border-border-default rounded-lg text-text-muted hover:text-text-default hover:border-border-medium transition-colors">
                    <Plus className="w-5 h-5 mx-auto mb-2" />
                    <span className="text-sm">Pull New Model from {provider.name}</span>
                  </button>
                </div>
              ))}
            </div>

            {/* Empty State for No Providers */}
            {modelProviders.length === 0 && (
              <div className="text-center py-12">
                <Server className="w-16 h-16 text-text-muted mx-auto mb-4" />
                <h3 className="text-lg font-medium text-text-default mb-2">No Model Providers Connected</h3>
                <p className="text-text-muted mb-4">
                  Connect to Ollama, Hugging Face, or other model providers to start fine-tuning.
                </p>
                <button className="flex items-center gap-2 px-4 py-2 bg-background-accent text-text-on-accent rounded-lg hover:bg-background-accent/90 transition-colors mx-auto">
                  <Plus className="w-4 h-4" />
                  Add Provider
                </button>
              </div>
            )}
          </motion.div>
        )}
      </div>
    </div>
  );
}
