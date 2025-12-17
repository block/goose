import React, { useState } from 'react';
import { motion } from 'framer-motion';
import { 
  Settings, 
  Code, 
  Cpu, 
  HardDrive, 
  Zap, 
  FileText, 
  ChevronDown,
  ChevronRight,
  Info,
  Copy,
  Play
} from 'lucide-react';

interface AxolotlConfigProps {
  onStartTraining: (config: AxolotlTrainingConfig) => void;
  availableModels: string[];
  datasets: string[];
}

interface AxolotlTrainingConfig {
  // Base model configuration
  base_model: string;
  // Optional: explicit local filesystem path to base model
  local_base_model_path?: string;
  model_type: string;
  tokenizer_type?: string;
  
  // Dataset configuration
  datasets: Array<{
    path: string;
    type: string;
    conversation?: string;
  }>;
  
  // Training parameters
  sequence_len: number;
  sample_packing: boolean;
  pad_to_sequence_len: boolean;
  
  // LoRA configuration
  adapter: string;
  lora_r: number;
  lora_alpha: number;
  lora_dropout: number;
  lora_target_modules: string[];
  
  // Training hyperparameters
  micro_batch_size: number;
  gradient_accumulation_steps: number;
  num_epochs: number;
  optimizer: string;
  lr_scheduler: string;
  learning_rate: number;
  train_on_inputs: boolean;
  group_by_length: boolean;
  bf16: boolean;
  fp16: boolean;
  tf32: boolean;
  
  // Evaluation
  val_set_size: number;
  eval_steps: number;
  save_steps: number;
  logging_steps: number;
  
  // Output configuration
  output_dir: string;
  
  // Hardware optimization
  gradient_checkpointing: boolean;
  deepspeed?: string;
  fsdp?: string[];
  special_tokens?: Record<string, string>;
}

export default function AxolotlConfig({ onStartTraining, availableModels, datasets }: AxolotlConfigProps) {
  const [config, setConfig] = useState<AxolotlTrainingConfig>({
    base_model: availableModels[0] || 'llama3.2:3b',
    model_type: 'LlamaForCausalLM',
    datasets: [{ path: datasets[0] || '', type: 'alpaca' }],
    sequence_len: 2048,
    sample_packing: true,
    pad_to_sequence_len: true,
    adapter: 'lora',
    lora_r: 32,
    lora_alpha: 16,
    lora_dropout: 0.05,
    lora_target_modules: ['q_proj', 'v_proj', 'k_proj', 'o_proj', 'gate_proj', 'down_proj', 'up_proj'],
    micro_batch_size: 2,
    gradient_accumulation_steps: 1,
    num_epochs: 4,
    optimizer: 'adamw_bnb_8bit',
    lr_scheduler: 'cosine',
    learning_rate: 0.0002,
    train_on_inputs: false,
    group_by_length: false,
    bf16: true,
    fp16: false,
    tf32: false,
    val_set_size: 0.05,
    eval_steps: 0.05,
    save_steps: 0.05,
    logging_steps: 1,
    output_dir: './outputs',
    gradient_checkpointing: true,
  });

  const [expandedSections, setExpandedSections] = useState<Set<string>>(new Set(['model', 'lora']));
  const [showYamlPreview, setShowYamlPreview] = useState(false);

  const toggleSection = (section: string) => {
    const newExpanded = new Set(expandedSections);
    if (newExpanded.has(section)) {
      newExpanded.delete(section);
    } else {
      newExpanded.add(section);
    }
    setExpandedSections(newExpanded);
  };

  const updateConfig = (path: string, value: any) => {
    setConfig(prev => {
      const newConfig = { ...prev };
      const keys = path.split('.');
      let current: any = newConfig;
      
      for (let i = 0; i < keys.length - 1; i++) {
        if (!(keys[i] in current)) {
          current[keys[i]] = {};
        }
        current = current[keys[i]];
      }
      
      current[keys[keys.length - 1]] = value;
      return newConfig;
    });
  };

  const generateYamlConfig = () => {
    const yamlLines: string[] = [];
    
    // Base model configuration
    yamlLines.push(`base_model: ${config.base_model}`);
    yamlLines.push(`model_type: ${config.model_type}`);
    if (config.tokenizer_type) {
      yamlLines.push(`tokenizer_type: ${config.tokenizer_type}`);
    }
    yamlLines.push('');
    
    // Load in 8bit
    yamlLines.push('load_in_8bit: true');
    yamlLines.push('load_in_4bit: false');
    yamlLines.push('strict: false');
    yamlLines.push('');
    
    // Datasets
    yamlLines.push('datasets:');
    config.datasets.forEach(dataset => {
      yamlLines.push(`  - path: ${dataset.path}`);
      yamlLines.push(`    type: ${dataset.type}`);
      if (dataset.conversation) {
        yamlLines.push(`    conversation: ${dataset.conversation}`);
      }
    });
    yamlLines.push('');
    
    // Dataset preparation
    yamlLines.push(`dataset_prepared_path:`);
    yamlLines.push(`val_set_size: ${config.val_set_size}`);
    yamlLines.push(`output_dir: ${config.output_dir}`);
    yamlLines.push('');
    
    // Sequence length
    yamlLines.push(`sequence_len: ${config.sequence_len}`);
    yamlLines.push(`sample_packing: ${config.sample_packing}`);
    yamlLines.push(`pad_to_sequence_len: ${config.pad_to_sequence_len}`);
    yamlLines.push('');
    
    // LoRA configuration
    yamlLines.push(`adapter: ${config.adapter}`);
    yamlLines.push(`lora_model_dir:`);
    yamlLines.push(`lora_r: ${config.lora_r}`);
    yamlLines.push(`lora_alpha: ${config.lora_alpha}`);
    yamlLines.push(`lora_dropout: ${config.lora_dropout}`);
    yamlLines.push('lora_target_modules:');
    config.lora_target_modules.forEach(module => {
      yamlLines.push(`  - ${module}`);
    });
    yamlLines.push('lora_target_linear: false');
    yamlLines.push('lora_fan_in_fan_out:');
    yamlLines.push('');
    
    // Wandb (optional)
    yamlLines.push('wandb_project:');
    yamlLines.push('wandb_entity:');
    yamlLines.push('wandb_watch:');
    yamlLines.push('wandb_name:');
    yamlLines.push('wandb_log_model:');
    yamlLines.push('');
    
    // Training parameters
    yamlLines.push(`gradient_accumulation_steps: ${config.gradient_accumulation_steps}`);
    yamlLines.push(`micro_batch_size: ${config.micro_batch_size}`);
    yamlLines.push(`num_epochs: ${config.num_epochs}`);
    yamlLines.push(`optimizer: ${config.optimizer}`);
    yamlLines.push(`lr_scheduler: ${config.lr_scheduler}`);
    yamlLines.push(`learning_rate: ${config.learning_rate}`);
    yamlLines.push('');
    
    // Training flags
    yamlLines.push(`train_on_inputs: ${config.train_on_inputs}`);
    yamlLines.push(`group_by_length: ${config.group_by_length}`);
    yamlLines.push(`bf16: ${config.bf16}`);
    yamlLines.push(`fp16: ${config.fp16}`);
    yamlLines.push(`tf32: ${config.tf32}`);
    yamlLines.push('');
    
    // Evaluation and saving
    yamlLines.push(`gradient_checkpointing: ${config.gradient_checkpointing}`);
    yamlLines.push(`early_stopping_patience:`);
    yamlLines.push(`resume_from_checkpoint:`);
    yamlLines.push(`local_rank:`);
    yamlLines.push('');
    
    yamlLines.push(`logging_steps: ${config.logging_steps}`);
    yamlLines.push('xformers_attention:');
    yamlLines.push('flash_attention: true');
    yamlLines.push('');
    
    yamlLines.push('warmup_steps: 10');
    yamlLines.push(`evals_per_epoch: ${config.eval_steps}`);
    yamlLines.push(`saves_per_epoch: ${config.save_steps}`);
    yamlLines.push('debug:');
    yamlLines.push('deepspeed:');
    yamlLines.push('weight_decay: 0.0');
    yamlLines.push('fsdp:');
    yamlLines.push('fsdp_config:');
    yamlLines.push('special_tokens:');
    
    return yamlLines.join('\n');
  };

  const copyYamlToClipboard = () => {
    navigator.clipboard.writeText(generateYamlConfig());
  };

  const ConfigSection = ({ 
    title, 
    icon: Icon, 
    sectionKey, 
    children 
  }: { 
    title: string; 
    icon: React.ComponentType<{ className?: string }>; 
    sectionKey: string; 
    children: React.ReactNode;
  }) => {
    const isExpanded = expandedSections.has(sectionKey);
    
    return (
      <div className="border border-border-default rounded-lg">
        <button
          onClick={() => toggleSection(sectionKey)}
          className="w-full flex items-center justify-between p-4 hover:bg-background-subtle transition-colors"
        >
          <div className="flex items-center gap-3">
            <Icon className="w-5 h-5 text-text-muted" />
            <h3 className="font-medium text-text-default">{title}</h3>
          </div>
          {isExpanded ? (
            <ChevronDown className="w-4 h-4 text-text-muted" />
          ) : (
            <ChevronRight className="w-4 h-4 text-text-muted" />
          )}
        </button>
        
        {isExpanded && (
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: 'auto' }}
            exit={{ opacity: 0, height: 0 }}
            className="border-t border-border-default p-4 space-y-4"
          >
            {children}
          </motion.div>
        )}
      </div>
    );
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-light text-text-default">Axolotl Training Configuration</h2>
          <p className="text-text-muted">Configure advanced fine-tuning parameters for local model training</p>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={() => setShowYamlPreview(!showYamlPreview)}
            className="flex items-center gap-2 px-3 py-2 text-sm bg-background-subtle hover:bg-background-medium rounded-lg transition-colors"
          >
            <FileText className="w-4 h-4" />
            {showYamlPreview ? 'Hide' : 'Show'} YAML
          </button>
          <button
            onClick={() => onStartTraining(config)}
            className="flex items-center gap-2 px-4 py-2 bg-background-accent text-text-on-accent rounded-lg hover:bg-background-accent/90 transition-colors"
          >
            <Play className="w-4 h-4" />
            Start Training
          </button>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Configuration Sections */}
        <div className="space-y-4">
          {/* Model Configuration */}
          <ConfigSection title="Model Configuration" icon={Cpu} sectionKey="model">
            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium text-text-default mb-2">Base Model</label>
                <select
                  value={config.base_model}
                  onChange={(e) => updateConfig('base_model', e.target.value)}
                  className="w-full px-3 py-2 bg-background-default border border-border-default rounded-lg focus:ring-2 focus:ring-background-accent focus:border-transparent"
                >
                  {availableModels.map(model => (
                    <option key={model} value={model}>{model}</option>
                  ))}
                </select>
              </div>
              
              <div>
                <label className="block text-sm font-medium text-text-default mb-2">Model Type</label>
                <select
                  value={config.model_type}
                  onChange={(e) => updateConfig('model_type', e.target.value)}
                  className="w-full px-3 py-2 bg-background-default border border-border-default rounded-lg focus:ring-2 focus:ring-background-accent focus:border-transparent"
                >
                  <option value="LlamaForCausalLM">LlamaForCausalLM</option>
                  <option value="MistralForCausalLM">MistralForCausalLM</option>
                  <option value="CodeLlamaForCausalLM">CodeLlamaForCausalLM</option>
                </select>
              </div>

              <div>
                <label className="block text-sm font-medium text-text-default mb-2">Local Base Model Path</label>
                <input
                  type="text"
                  placeholder="/Users/you/models/qwen2.5-7b"
                  value={config.local_base_model_path || ''}
                  onChange={(e) => updateConfig('local_base_model_path', e.target.value)}
                  className="w-full px-3 py-2 bg-background-default border border-border-default rounded-lg focus:ring-2 focus:ring-background-accent focus:border-transparent"
                />
                <p className="text-xs text-text-muted mt-1">If set, this path is sent as base_model_path to the trainer.</p>
              </div>
              
              <div>
                <label className="block text-sm font-medium text-text-default mb-2">Sequence Length</label>
                <input
                  type="number"
                  value={config.sequence_len}
                  onChange={(e) => updateConfig('sequence_len', parseInt(e.target.value))}
                  className="w-full px-3 py-2 bg-background-default border border-border-default rounded-lg focus:ring-2 focus:ring-background-accent focus:border-transparent"
                />
              </div>
            </div>
          </ConfigSection>

          {/* LoRA Configuration */}
          <ConfigSection title="LoRA Configuration" icon={Zap} sectionKey="lora">
            <div className="space-y-4">
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="block text-sm font-medium text-text-default mb-2">LoRA Rank (r)</label>
                  <input
                    type="number"
                    value={config.lora_r}
                    onChange={(e) => updateConfig('lora_r', parseInt(e.target.value))}
                    className="w-full px-3 py-2 bg-background-default border border-border-default rounded-lg focus:ring-2 focus:ring-background-accent focus:border-transparent"
                  />
                </div>
                <div>
                  <label className="block text-sm font-medium text-text-default mb-2">LoRA Alpha</label>
                  <input
                    type="number"
                    value={config.lora_alpha}
                    onChange={(e) => updateConfig('lora_alpha', parseInt(e.target.value))}
                    className="w-full px-3 py-2 bg-background-default border border-border-default rounded-lg focus:ring-2 focus:ring-background-accent focus:border-transparent"
                  />
                </div>
              </div>
              
              <div>
                <label className="block text-sm font-medium text-text-default mb-2">LoRA Dropout</label>
                <input
                  type="number"
                  step="0.01"
                  value={config.lora_dropout}
                  onChange={(e) => updateConfig('lora_dropout', parseFloat(e.target.value))}
                  className="w-full px-3 py-2 bg-background-default border border-border-default rounded-lg focus:ring-2 focus:ring-background-accent focus:border-transparent"
                />
              </div>
              
              <div>
                <label className="block text-sm font-medium text-text-default mb-2">Target Modules</label>
                <div className="flex flex-wrap gap-2">
                  {['q_proj', 'v_proj', 'k_proj', 'o_proj', 'gate_proj', 'down_proj', 'up_proj'].map(module => (
                    <label key={module} className="flex items-center gap-2">
                      <input
                        type="checkbox"
                        checked={config.lora_target_modules.includes(module)}
                        onChange={(e) => {
                          const modules = e.target.checked 
                            ? [...config.lora_target_modules, module]
                            : config.lora_target_modules.filter(m => m !== module);
                          updateConfig('lora_target_modules', modules);
                        }}
                        className="rounded"
                      />
                      <span className="text-sm text-text-default">{module}</span>
                    </label>
                  ))}
                </div>
              </div>
            </div>
          </ConfigSection>

          {/* Training Parameters */}
          <ConfigSection title="Training Parameters" icon={Settings} sectionKey="training">
            <div className="space-y-4">
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="block text-sm font-medium text-text-default mb-2">Micro Batch Size</label>
                  <input
                    type="number"
                    value={config.micro_batch_size}
                    onChange={(e) => updateConfig('micro_batch_size', parseInt(e.target.value))}
                    className="w-full px-3 py-2 bg-background-default border border-border-default rounded-lg focus:ring-2 focus:ring-background-accent focus:border-transparent"
                  />
                </div>
                <div>
                  <label className="block text-sm font-medium text-text-default mb-2">Gradient Accumulation</label>
                  <input
                    type="number"
                    value={config.gradient_accumulation_steps}
                    onChange={(e) => updateConfig('gradient_accumulation_steps', parseInt(e.target.value))}
                    className="w-full px-3 py-2 bg-background-default border border-border-default rounded-lg focus:ring-2 focus:ring-background-accent focus:border-transparent"
                  />
                </div>
              </div>
              
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="block text-sm font-medium text-text-default mb-2">Learning Rate</label>
                  <input
                    type="number"
                    step="0.0001"
                    value={config.learning_rate}
                    onChange={(e) => updateConfig('learning_rate', parseFloat(e.target.value))}
                    className="w-full px-3 py-2 bg-background-default border border-border-default rounded-lg focus:ring-2 focus:ring-background-accent focus:border-transparent"
                  />
                </div>
                <div>
                  <label className="block text-sm font-medium text-text-default mb-2">Epochs</label>
                  <input
                    type="number"
                    value={config.num_epochs}
                    onChange={(e) => updateConfig('num_epochs', parseInt(e.target.value))}
                    className="w-full px-3 py-2 bg-background-default border border-border-default rounded-lg focus:ring-2 focus:ring-background-accent focus:border-transparent"
                  />
                </div>
              </div>
              
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="block text-sm font-medium text-text-default mb-2">Optimizer</label>
                  <select
                    value={config.optimizer}
                    onChange={(e) => updateConfig('optimizer', e.target.value)}
                    className="w-full px-3 py-2 bg-background-default border border-border-default rounded-lg focus:ring-2 focus:ring-background-accent focus:border-transparent"
                  >
                    <option value="adamw_bnb_8bit">AdamW BNB 8bit</option>
                    <option value="adamw_torch">AdamW Torch</option>
                    <option value="sgd">SGD</option>
                  </select>
                </div>
                <div>
                  <label className="block text-sm font-medium text-text-default mb-2">LR Scheduler</label>
                  <select
                    value={config.lr_scheduler}
                    onChange={(e) => updateConfig('lr_scheduler', e.target.value)}
                    className="w-full px-3 py-2 bg-background-default border border-border-default rounded-lg focus:ring-2 focus:ring-background-accent focus:border-transparent"
                  >
                    <option value="cosine">Cosine</option>
                    <option value="linear">Linear</option>
                    <option value="constant">Constant</option>
                  </select>
                </div>
              </div>
            </div>
          </ConfigSection>
        </div>

        {/* YAML Preview */}
        {showYamlPreview && (
          <div className="lg:col-span-1">
            <div className="sticky top-4">
              <div className="bg-background-muted rounded-lg border border-border-default">
                <div className="flex items-center justify-between p-4 border-b border-border-default">
                  <h3 className="font-medium text-text-default flex items-center gap-2">
                    <Code className="w-4 h-4" />
                    Generated YAML Config
                  </h3>
                  <button
                    onClick={copyYamlToClipboard}
                    className="flex items-center gap-2 px-2 py-1 text-xs bg-background-subtle hover:bg-background-medium rounded transition-colors"
                  >
                    <Copy className="w-3 h-3" />
                    Copy
                  </button>
                </div>
                <div className="p-4">
                  <pre className="text-xs text-text-default bg-background-default p-3 rounded border border-border-default overflow-auto max-h-96">
                    {generateYamlConfig()}
                  </pre>
                </div>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
