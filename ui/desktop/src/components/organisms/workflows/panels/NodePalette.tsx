import {
  ArrowRightLeft,
  Bot,
  GitBranch,
  Globe,
  GripVertical,
  UserCheck,
  Wrench,
  Zap,
} from 'lucide-react';
import type React from 'react';
import { NODE_PALETTE, type NodeKind } from '../types';

const ICONS: Record<string, React.FC<{ size?: number }>> = {
  Zap,
  Bot,
  Wrench,
  GitBranch,
  ArrowRightLeft,
  UserCheck,
  Globe,
};

interface NodePaletteProps {
  onDragStart: (kind: NodeKind) => void;
}

export function NodePalette({ onDragStart }: NodePaletteProps) {
  const handleDragStart = (kind: NodeKind) => (event: React.DragEvent<HTMLButtonElement>) => {
    event.dataTransfer.setData('application/dagnode', kind);
    event.dataTransfer.effectAllowed = 'move';
    onDragStart(kind);
  };

  return (
    <div className="w-56 border-r border-border-default bg-background-default overflow-y-auto">
      <div className="p-3 border-b border-border-muted">
        <h3 className="text-sm font-semibold text-text-default">Nodes</h3>
        <p className="text-xs text-text-muted mt-0.5">Drag to canvas</p>
      </div>
      <div className="p-2 space-y-1">
        {NODE_PALETTE.map((item) => {
          const Icon = ICONS[item.icon];
          return (
            <button
              key={item.kind}
              type="button"
              draggable
              onDragStart={handleDragStart(item.kind)}
              className="flex items-center gap-2 p-2 rounded-md cursor-grab active:cursor-grabbing
                         hover:bg-background-muted transition-colors group"
              aria-label={`Drag ${item.label} to canvas`}
            >
              <GripVertical
                size={12}
                className="text-text-subtle opacity-0 group-hover:opacity-100 transition-opacity"
              />
              <div
                className="flex items-center justify-center w-7 h-7 rounded-md"
                style={{ background: `${item.color}20` }}
              >
                {Icon && <Icon size={14} />}
              </div>
              <div className="flex-1 min-w-0">
                <div className="text-sm font-medium text-text-default">{item.label}</div>
                <div className="text-xs text-text-muted truncate">{item.description}</div>
              </div>
            </button>
          );
        })}
      </div>
    </div>
  );
}
