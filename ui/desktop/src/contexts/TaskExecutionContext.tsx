import React, { createContext, useContext, useState, useCallback, ReactNode } from 'react';

// Task status type
export type TaskStatus = 'pending' | 'running' | 'completed' | 'error';

// Task info including status and ID
export type TaskInfo = {
  status: TaskStatus;
  taskId: string;
};

// Map of task indices to their info
export type TaskInfoMap = Map<number, TaskInfo>;

// Map of create_task tool call IDs to their task info maps
type TaskExecutionState = Map<string, TaskInfoMap>;

// Map of task IDs (e.g., "task-0") to their parent create_task tool call ID
type TaskIdToCreateTaskMap = Map<string, string>;

interface TaskExecutionContextType {
  getTaskInfos: (createTaskId: string) => TaskInfoMap | undefined;
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
      const taskInfos = new Map<number, TaskInfo>();
      
      // Initialize all tasks as pending with their task IDs
      for (let i = 0; i < taskIds.length; i++) {
        taskInfos.set(i, {
          status: 'pending',
          taskId: taskIds[i]
        });
      }
      
      newState.set(createTaskId, taskInfos);
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

  // Get task infos for a specific create_task
  const getTaskInfos = useCallback((createTaskId: string): TaskInfoMap | undefined => {
    return taskExecutionState.get(createTaskId);
  }, [taskExecutionState]);

  // Update a single task's status
  const updateTaskStatus = useCallback((createTaskId: string, taskIndex: number, status: TaskStatus) => {
    setTaskExecutionState((prev) => {
      const newState = new Map(prev);
      const taskInfos = newState.get(createTaskId) || new Map();
      const updatedInfos = new Map(taskInfos);
      
      const existingInfo = updatedInfos.get(taskIndex);
      if (existingInfo) {
        updatedInfos.set(taskIndex, {
          ...existingInfo,
          status
        });
      }
      
      newState.set(createTaskId, updatedInfos);
      
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
    getTaskInfos,
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
