import React, { useState, useEffect, useCallback } from 'react';
import { Project } from '../../projects';
import { Session, fetchSessions } from '../../sessions';
import {
  getProject as fetchProject,
  removeSessionFromProject,
  deleteProject,
  addSessionToProject,
} from '../../projects';
import { Button } from '../ui/button';
import {
  ArrowLeft,
  Loader,
  RefreshCcw,
  Edit,
  Trash2,
  Folder,
  MessageSquareText,
  ChevronLeft,
  LoaderCircle,
  AlertCircle,
  Calendar,
  Target,
} from 'lucide-react';
import { toastError, toastSuccess } from '../../toasts';
import { formatMessageTimestamp } from '../../utils/timeUtils';
import AddSessionToProjectModal from './AddSessionToProjectModal';
import UpdateProjectModal from './UpdateProjectModal';
import { MainPanelLayout } from '../Layout/MainPanelLayout';
import { ScrollArea } from '../ui/scroll-area';
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '../ui/alert-dialog';
import { ChatSmart } from '../icons';
import { View, ViewOptions } from '../../App';
import { Card } from '../ui/card';

interface ProjectDetailsViewProps {
  projectId: string;
  onBack: () => void;
  setView: (view: View, viewOptions?: ViewOptions) => void;
}

// Custom ProjectHeader component similar to SessionHistoryView style
const ProjectHeader: React.FC<{
  onBack: () => void;
  children: React.ReactNode;
  title: string;
  actionButtons?: React.ReactNode;
}> = ({ onBack, children, title, actionButtons }) => {
  return (
    <div className="flex flex-col pb-8">
      <div className="flex items-center pt-13 pb-2">
        <Button onClick={onBack} size="xs" variant="outline">
          <ChevronLeft />
          Back
        </Button>
      </div>
      <h1 className="text-4xl font-light mb-4">{title}</h1>
      <div className="flex items-center">{children}</div>
      {actionButtons && <div className="flex items-center space-x-3 mt-4">{actionButtons}</div>}
    </div>
  );
};

// New component for displaying project sessions with consistent styling
const ProjectSessions: React.FC<{
  sessions: Session[];
  isLoading: boolean;
  error: string | null;
  onRetry: () => void;
  onRemoveSession: (sessionId: string) => void;
  onAddSession: () => void;
}> = ({ sessions, isLoading, error, onRetry }) => {
  return (
    <ScrollArea className="h-full w-full">
      <div className="pb-16">
        <div className="flex flex-col space-y-6">
          {isLoading ? (
            <div className="flex justify-center items-center py-12">
              <LoaderCircle className="animate-spin h-8 w-8 text-textStandard" />
            </div>
          ) : error ? (
            <div className="flex flex-col items-center justify-center py-8 text-textSubtle">
              <div className="text-red-500 mb-4">
                <AlertCircle size={32} />
              </div>
              <p className="text-md mb-2">Error Loading Project Details</p>
              <p className="text-sm text-center mb-4">{error}</p>
              <Button onClick={onRetry} variant="default">
                Try Again
              </Button>
            </div>
          ) : sessions?.length > 0 ? (
            <div className="w-full">
              <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 2xl:grid-cols-5 gap-4">
                {sessions.map((session) => (
                  <Card
                    key={session.id}
                    className="h-full py-3 px-4 hover:shadow-default cursor-pointer transition-all duration-150 flex flex-col justify-between"
                  >
                    <div className="flex-1">
                      <h3 className="text-base truncate mb-1">
                        {session.metadata.description || session.id}
                      </h3>
                      <div className="flex items-center text-text-muted text-xs mb-1">
                        <Calendar className="w-3 h-3 mr-1 flex-shrink-0" />
                        <span>{formatMessageTimestamp(Date.parse(session.modified) / 1000)}</span>
                      </div>
                      <div className="flex items-center text-text-muted text-xs mb-1">
                        <Folder className="w-3 h-3 mr-1 flex-shrink-0" />
                        <span className="truncate">{session.metadata.working_dir}</span>
                      </div>
                    </div>

                    <div className="flex items-center justify-between mt-1 pt-2">
                      <div className="flex items-center space-x-3 text-xs text-text-muted">
                        <div className="flex items-center">
                          <MessageSquareText className="w-3 h-3 mr-1" />
                          <span className="font-mono">{session.metadata.message_count}</span>
                        </div>
                        {session.metadata.total_tokens !== null && (
                          <div className="flex items-center">
                            <Target className="w-3 h-3 mr-1" />
                            <span className="font-mono">
                              {session.metadata.total_tokens.toLocaleString()}
                            </span>
                          </div>
                        )}
                      </div>
                      {/* <Button
                        variant="ghost"
                        size="sm"
                        onClick={(e) => {
                          e.stopPropagation();
                          onRemoveSession(session.id);
                        }}
                        className="text-xs"
                      >
                        Remove
                      </Button> */}
                    </div>
                  </Card>
                ))}
              </div>
            </div>
          ) : (
            <div className="flex flex-col justify-center text-textSubtle">
              <p className="text-lg mb-2">No sessions in this project</p>
              <p className="text-sm mb-4 text-text-muted">
                Add sessions to this project to keep your work organized
              </p>
              {/* <Button onClick={onAddSession}>
                <Plus className="h-4 w-4 mr-2" />
                Add Session
              </Button> */}
            </div>
          )}
        </div>
      </div>
    </ScrollArea>
  );
};

const ProjectDetailsView: React.FC<ProjectDetailsViewProps> = ({ projectId, onBack, setView }) => {
  const [project, setProject] = useState<Project | null>(null);
  const [sessions, setSessions] = useState<Session[]>([]);
  const [allSessions, setAllSessions] = useState<Session[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [isAddSessionModalOpen, setIsAddSessionModalOpen] = useState(false);
  const [isUpdateModalOpen, setIsUpdateModalOpen] = useState(false);
  const [isDeleteDialogOpen, setIsDeleteDialogOpen] = useState(false);
  const [isDeleting, setIsDeleting] = useState(false);

  const loadProjectData = useCallback(async () => {
    setLoading(true);
    setError(null);

    try {
      // Fetch the project details
      const projectData = await fetchProject(projectId);
      setProject(projectData);

      // Fetch all sessions
      const allSessionsData = await fetchSessions();
      setAllSessions(allSessionsData);

      // Filter sessions that belong to this project
      const projectSessions = allSessionsData.filter((session: Session) =>
        projectData.sessionIds.includes(session.id)
      );

      setSessions(projectSessions);
    } catch (err) {
      console.error('Failed to load project data:', err);
      setError('Failed to load project data');
      toastError({ title: 'Error', msg: 'Failed to load project data' });
    } finally {
      setLoading(false);
    }
  }, [projectId]);

  // Fetch project details and associated sessions
  useEffect(() => {
    loadProjectData();
  }, [projectId, loadProjectData]);

  // Set up session creation listener to automatically associate new sessions with this project
  useEffect(() => {
    if (!project) return;

    const handleSessionCreated = async () => {
      console.log(
        'ProjectDetailsView: Session created event received, checking for new sessions...'
      );

      // Wait a bit for the session to be fully created
      setTimeout(async () => {
        try {
          // Fetch all sessions to find the newest one
          const allSessionsData = await fetchSessions();

          // Find sessions that are not in this project but were created recently
          const recentSessions = allSessionsData.filter((session: Session) => {
            const sessionDate = new Date(session.modified);
            const fiveMinutesAgo = new Date(Date.now() - 5 * 60 * 1000);
            const isRecent = sessionDate > fiveMinutesAgo;
            const isNotInProject = !project.sessionIds.includes(session.id);
            const isInProjectDirectory = session.metadata.working_dir === project.defaultDirectory;

            return isRecent && isNotInProject && isInProjectDirectory;
          });

          // Add recent sessions to this project
          for (const session of recentSessions) {
            try {
              await addSessionToProject(project.id, session.id);
              console.log(`Automatically added session ${session.id} to project ${project.id}`);
            } catch (err) {
              console.error(`Failed to add session ${session.id} to project:`, err);
            }
          }

          // Refresh project data if we added any sessions
          if (recentSessions.length > 0) {
            loadProjectData();
          }
        } catch (err) {
          console.error('Error checking for new sessions:', err);
        }
      }, 2000); // Wait 2 seconds for session to be created
    };

    // Listen for session creation events
    window.addEventListener('session-created', handleSessionCreated);
    window.addEventListener('message-stream-finished', handleSessionCreated);

    return () => {
      window.removeEventListener('session-created', handleSessionCreated);
      window.removeEventListener('message-stream-finished', handleSessionCreated);
    };
  }, [project, loadProjectData]);

  const handleRemoveSession = async (sessionId: string) => {
    if (!project) return;

    try {
      await removeSessionFromProject(project.id, sessionId);

      // Update local state
      setProject((prev) => {
        if (!prev) return null;
        return {
          ...prev,
          sessionIds: prev.sessionIds.filter((id) => id !== sessionId),
        };
      });

      setSessions((prev) => prev.filter((s) => s.id !== sessionId));
      toastSuccess({ title: 'Success', msg: 'Session removed from project' });
    } catch (err) {
      console.error('Failed to remove session from project:', err);
      toastError({ title: 'Error', msg: 'Failed to remove session from project' });
    }
  };

  const getSessionsNotInProject = () => {
    if (!project) return [];

    return allSessions.filter((session) => !project.sessionIds.includes(session.id));
  };

  const handleDeleteProject = async () => {
    if (!project) return;

    setIsDeleting(true);
    try {
      await deleteProject(project.id);
      toastSuccess({ title: 'Success', msg: `Project "${project.name}" deleted successfully` });
      onBack(); // Go back to projects list
    } catch (err) {
      console.error('Failed to delete project:', err);
      toastError({ title: 'Error', msg: 'Failed to delete project' });
    } finally {
      setIsDeleting(false);
      setIsDeleteDialogOpen(false);
    }
  };

  const handleNewSession = () => {
    if (!project) return;

    console.log(`Navigating to chat page for project: ${project.name}`);

    // Update the working directory in localStorage to the project's directory
    try {
      const currentConfig = JSON.parse(localStorage.getItem('gooseConfig') || '{}');
      const updatedConfig = {
        ...currentConfig,
        GOOSE_WORKING_DIR: project.defaultDirectory,
      };
      localStorage.setItem('gooseConfig', JSON.stringify(updatedConfig));
    } catch (error) {
      console.error('Failed to update working directory in localStorage:', error);
    }

    // Navigate to the pair page
    setView('pair');

    toastSuccess({
      title: 'New Session',
      msg: `Starting new session in ${project.name}`,
    });
  };

  if (loading) {
    return (
      <MainPanelLayout>
        <div className="flex flex-col h-full w-full items-center justify-center">
          <Loader className="h-10 w-10 animate-spin opacity-70 mb-4" />
          <p className="text-muted-foreground">Loading project...</p>
        </div>
      </MainPanelLayout>
    );
  }

  if (error || !project) {
    return (
      <MainPanelLayout>
        <div className="flex flex-col h-full w-full items-center justify-center">
          <div className="text-center">
            <p className="text-red-500 mb-4">{error || 'Project not found'}</p>
            <div className="flex gap-2">
              <Button onClick={onBack} variant="outline">
                <ArrowLeft className="mr-2 h-4 w-4" /> Back
              </Button>
              <Button onClick={loadProjectData}>
                <RefreshCcw className="mr-2 h-4 w-4" /> Retry
              </Button>
            </div>
          </div>
        </div>
      </MainPanelLayout>
    );
  }

  // Define action buttons
  const actionButtons = (
    <>
      <Button onClick={handleNewSession} size="sm" className="flex items-center gap-1">
        <ChatSmart className="h-4 w-4" />
        <span>New session</span>
      </Button>
      <Button
        onClick={() => setIsUpdateModalOpen(true)}
        size="sm"
        variant="outline"
        className="flex items-center gap-1"
      >
        <Edit className="h-4 w-4" />
        <span>Edit</span>
      </Button>
      <Button
        onClick={() => setIsDeleteDialogOpen(true)}
        size="sm"
        variant="outline"
        className="flex items-center gap-1"
      >
        <Trash2 className="h-4 w-4" />
        <span>Delete</span>
      </Button>
      {/* <Button
        onClick={() => setIsAddSessionModalOpen(true)}
        size="sm"
        className="flex items-center gap-1"
      >
        <Plus className="h-4 w-4" />
        <span>Add Session</span>
      </Button> */}
    </>
  );

  return (
    <>
      <MainPanelLayout>
        <div className="flex-1 flex flex-col min-h-0 px-8">
          <ProjectHeader onBack={onBack} title={project.name} actionButtons={actionButtons}>
            <div className="flex flex-col">
              {!loading && (
                <>
                  <div className="flex items-center text-text-muted text-sm space-x-5 font-mono">
                    <span className="flex items-center">
                      <MessageSquareText className="w-4 h-4 mr-1" />
                      {sessions.length} {sessions.length === 1 ? 'session' : 'sessions'}
                    </span>
                  </div>
                  <div className="flex items-center text-text-muted text-sm mt-1 font-mono">
                    <span className="flex items-center">
                      <Folder className="w-4 h-4 mr-1" />
                      {project.defaultDirectory}
                    </span>
                  </div>
                  {project.description && (
                    <div className="flex items-center text-text-muted text-sm mt-1">
                      <span>{project.description}</span>
                    </div>
                  )}
                </>
              )}
            </div>
          </ProjectHeader>

          <ProjectSessions
            sessions={sessions}
            isLoading={loading}
            error={error}
            onRetry={loadProjectData}
            onRemoveSession={handleRemoveSession}
            onAddSession={() => setIsAddSessionModalOpen(true)}
          />
        </div>
      </MainPanelLayout>

      <AddSessionToProjectModal
        isOpen={isAddSessionModalOpen}
        onClose={() => setIsAddSessionModalOpen(false)}
        project={project}
        availableSessions={getSessionsNotInProject()}
        onSessionsAdded={loadProjectData}
      />

      <UpdateProjectModal
        isOpen={isUpdateModalOpen}
        onClose={() => setIsUpdateModalOpen(false)}
        project={{
          ...project,
          sessionCount: sessions.length,
        }}
        onRefresh={loadProjectData}
      />

      <AlertDialog open={isDeleteDialogOpen} onOpenChange={setIsDeleteDialogOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Are you sure you want to delete this project?</AlertDialogTitle>
            <AlertDialogDescription>
              This will delete the project "{project.name}". The sessions within this project won't
              be deleted, but they will no longer be part of this project.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              className="bg-red-500 hover:bg-red-600"
              onClick={(e) => {
                e.preventDefault();
                handleDeleteProject();
              }}
              disabled={isDeleting}
            >
              {isDeleting ? 'Deleting...' : 'Delete'}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
};

export default ProjectDetailsView;
