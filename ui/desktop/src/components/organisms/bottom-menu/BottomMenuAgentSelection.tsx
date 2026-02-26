import { Bot } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { listBuiltinAgents, orchestratorStatus } from '@/api';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuTrigger,
} from '@/components/molecules/ui/dropdown-menu';

interface AgentInfo {
  name: string;
  enabled: boolean;
  modes: number;
}

interface OrchestratorInfo {
  enabled: boolean;
  routing_mode: string;
  total_modes: number;
  agents: Array<{ name: string; modes: number; enabled: boolean }>;
}

export const BottomMenuAgentSelection = () => {
  const [agents, setAgents] = useState<AgentInfo[]>([]);
  const [orchestrator, setOrchestrator] = useState<OrchestratorInfo | null>(null);
  const [isOpen, setIsOpen] = useState(false);

  useEffect(() => {
    const fetchAgents = async () => {
      try {
        const [builtinRes, statusRes] = await Promise.all([
          listBuiltinAgents(),
          orchestratorStatus(),
        ]);

        if (statusRes.data) {
          const data = statusRes.data as unknown as OrchestratorInfo;
          setOrchestrator(data);
          if (data.agents) {
            setAgents(
              data.agents.map((a) => ({
                name: a.name,
                enabled: a.enabled,
                modes: a.modes,
              }))
            );
          }
        } else if (builtinRes.data) {
          const builtin = builtinRes.data as unknown as Array<{
            name: string;
            enabled: boolean;
            modes?: Array<unknown>;
          }>;
          setAgents(
            builtin.map((a) => ({
              name: a.name,
              enabled: a.enabled,
              modes: a.modes?.length || 0,
            }))
          );
        }
      } catch {
        setAgents([
          { name: 'Goose Agent', enabled: true, modes: 7 },
          { name: 'Coding Agent', enabled: true, modes: 8 },
        ]);
      }
    };
    fetchAgents();
  }, []);

  const activeCount = useMemo(() => {
    return agents.filter((a) => a.enabled).length;
  }, [agents]);

  const totalModes = useMemo(() => {
    return agents.reduce((sum, a) => sum + a.modes, 0);
  }, [agents]);

  return (
    <DropdownMenu open={isOpen} onOpenChange={setIsOpen}>
      <DropdownMenuTrigger asChild>
        <button
          className="flex items-center [&_svg]:size-4 text-text-default/70 hover:text-text-default hover:scale-100 hover:bg-transparent text-xs cursor-pointer"
          title={`${activeCount} agents · ${totalModes} modes`}
        >
          <Bot className="mr-1 h-4 w-4" />
          <span>{activeCount}</span>
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent side="top" align="center" className="w-64 p-2">
        <div className="text-xs font-medium text-text-default/50 uppercase tracking-wider mb-2 px-1">
          Active Agents
        </div>
        {orchestrator && (
          <div className="flex items-center justify-between px-1 py-1.5 mb-1 rounded bg-surface-subtle">
            <div className="text-xs text-text-default/70">{orchestrator.routing_mode}</div>
          </div>
        )}
        {agents.map((agent) => (
          <div key={agent.name} className="flex items-center justify-between px-1 py-1.5">
            <div className="flex items-center gap-2">
              <div
                className={`w-2 h-2 rounded-full ${agent.enabled ? 'bg-green-400' : 'bg-gray-400'}`}
              />
              <span className="text-sm text-text-default">{agent.name}</span>
            </div>
            <span className="text-xs text-text-default/50">
              {agent.modes} mode{agent.modes !== 1 ? 's' : ''}
            </span>
          </div>
        ))}
        {agents.length === 0 && (
          <div className="px-1 py-2 text-center text-xs text-text-default/50">
            Loading agents...
          </div>
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  );
};
