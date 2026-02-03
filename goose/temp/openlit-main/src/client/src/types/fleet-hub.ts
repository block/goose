export interface AgentAttribute {
  key: string
  value: { Value: { StringValue: string } }
}

export interface Agent {
  InstanceIdStr: string;
  Status: {
    health: {
      healthy: boolean
      status: string
      component_health_map: Record<string, { healthy: boolean; status: string }>
      last_error?: string
    }
    agent_description: {
      identifying_attributes: AgentAttribute[]
      non_identifying_attributes: AgentAttribute[]
    }
  };
  StartedAt: string;
  EffectiveConfig: string;
  CustomInstanceConfig: string;
}

export enum OpampFilterType {
  "ALL" = "ALL",
  "HEALTHY" = "HEALTHY",
  "UNHEALTHY" = "UNHEALTHY",
}

export type OpampFilters = {
  type: OpampFilterType;
}