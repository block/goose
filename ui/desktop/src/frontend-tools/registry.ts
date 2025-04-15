import { RegisteredTool, ToolExecutor, FrontendToolDefinition } from './types';

class ToolRegistry {
  private tools = new Map<string, RegisteredTool>();

  registerTool(definition: FrontendToolDefinition, executor: ToolExecutor) {
    this.tools.set(definition.name, {
      ...definition,
      execute: executor,
    });
  }

  getTool(name: string): RegisteredTool | undefined {
    return this.tools.get(name);
  }

  getTools(): RegisteredTool[] {
    return Array.from(this.tools.values());
  }

  getToolDefinitions(): FrontendToolDefinition[] {
    return this.getTools().map(({ name, description, inputSchema }) => ({
      name,
      description,
      inputSchema,
    }));
  }
}

// Singleton instance
export const toolRegistry = new ToolRegistry();
