import React, { createContext, useContext, useState, useCallback, ReactNode } from 'react';

// Task status type
export type TaskStatus = 'pending' | 'running' | 'completed' | 'error';

// Map of task indices to their status
export type TaskStatusMap = Map<number, TaskStatus>;

// Map of create_task tool call IDs to their task status maps
type TaskExecutionState = Map<string, TaskStatusMap>;

// Map of task IDs (e.g., "task-0") to their parent create_task tool call ID
type TaskIdToCreateTaskMap = Map<string, string>;

interface TaskExecutionContextType {
  getTaskStatuses: (createTaskId: string) => TaskStatusMap | undefined;
  updateTaskStatus: (createTaskId: string, taskIndex: number, status: TaskStatus) => void;
  updateMultipleTaskStatuses: (createTaskId: string, statuses: Array<{ index: number; status: TaskStatus }>) => void;
  clearTaskStatuses: (createTaskId: string) => void;
  registerCreateTask: (createTaskId: string, taskIds: string[]) => void;
  getCreateTaskIdFromTaskId: (taskId: string) => string | undefined;
}

const TaskExecutionContext = createContext<TaskExecutionContextType | undefined>(undefined);

interface TaskExecutionProviderProps {
  children: ReactNode;
}

export const TaskExecutionProvider: React.FC<TaskExecutionProviderProps> = ({ children }) => {
  const [taskExecutionState, setTaskExecutionState] = useState<TaskExecutionState>(new Map());
  const [taskIdMapping, setTaskIdMapping] = useState<TaskIdToCreateTaskMap>(new Map());

  // Register a new create_task with all tasks set to pending and map task IDs to this create_task
  const registerCreateTask = useCallback((createTaskId: string, taskIds: string[]) => {
    setTaskExecutionState((prev) => {
      const newState = new Map(prev);
      const taskStatuses = new Map<number, TaskStatus>();
      
      // Initialize all tasks as pending
      for (let i = 0; i < taskIds.length; i++) {
        taskStatuses.set(i, 'pending');
      }
      
      newState.set(createTaskId, taskStatuses);
      console.log('📋 Registered create_task:', createTaskId, 'with', taskIds.length, 'tasks');
      return newState;
    });
    
    // Map each task ID to this create_task
    setTaskIdMapping((prev) => {
      const newMapping = new Map(prev);
      taskIds.forEach((taskId, index) => {
        newMapping.set(taskId, createTaskId);
        console.log('🔗 Mapped task ID:', taskId, '→ create_task:', createTaskId, 'index:', index);
      });
      return newMapping;
    });
  }, []);

  // Get task statuses for a specific create_task
  const getTaskStatuses = useCallback((createTaskId: string): TaskStatusMap | undefined => {
    return taskExecutionState.get(createTaskId);
  }, [taskExecutionState]);

  // Update a single task's status
  const updateTaskStatus = useCallback((createTaskId: string, taskIndex: number, status: TaskStatus) => {
    setTaskExecutionState((prev) => {
      const newState = new Map(prev);
      const taskStatuses = newState.get(createTaskId) || new Map();
      const updatedStatuses = new Map(taskStatuses);
      
      updatedStatuses.set(taskIndex, status);
      newState.set(createTaskId, updatedStatuses);
      
      console.log('✅ Updated task status:', createTaskId, 'task', taskIndex, '→', status);
      return newState;
    });
  }, []);

  // Update multiple task statuses at once
  const updateMultipleTaskStatuses = useCallback((
    createTaskId: string, 
    statuses: Array<{ index: number; status: TaskStatus }>
  ) => {
    setTaskExecutionState((prev) => {
      const newState = new Map(prev);
      const taskStatuses = newState.get(createTaskId) || new Map();
      const updatedStatuses = new Map(taskStatuses);
      
      statuses.forEach(({ index, status }) => {
        updatedStatuses.set(index, status);
      });
      
      newState.set(createTaskId, updatedStatuses);
      
      console.log('✅ Updated multiple task statuses:', createTaskId, statuses);
      return newState;
    });
  }, []);

  // Clear task statuses for a specific create_task
  const clearTaskStatuses = useCallback((createTaskId: string) => {
    setTaskExecutionState((prev) => {
      const newState = new Map(prev);
      newState.delete(createTaskId);
      console.log('🗑️ Cleared task statuses for:', createTaskId);
      return newState;
    });
  }, []);

  // Get the create_task ID from a task ID
  const getCreateTaskIdFromTaskId = useCallback((taskId: string): string | undefined => {
    return taskIdMapping.get(taskId);
  }, [taskIdMapping]);

  const contextValue: TaskExecutionContextType = {
    getTaskStatuses,
    updateTaskStatus,
    updateMultipleTaskStatuses,
    clearTaskStatuses,
    registerCreateTask,
    getCreateTaskIdFromTaskId,
  };

  return (
    <TaskExecutionContext.Provider value={contextValue}>
      {children}
    </TaskExecutionContext.Provider>
  );
};

export const useTaskExecution = (): TaskExecutionContextType => {
  const context = useContext(TaskExecutionContext);
  if (!context) {
    throw new Error('useTaskExecution must be used within a TaskExecutionProvider');
  }
  return context;
};
