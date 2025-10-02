import React, { useState, useEffect } from 'react';
import { FolderOpen, Clock, Trash2, RefreshCw, Sparkles, Plus } from 'lucide-react';
import { format } from 'date-fns';
import { toast } from 'react-toastify';
import { listProjects, removeProject } from '../../api';
import type { ProjectInfoDisplay } from '../../api';
import { MainPanelLayout } from '../Layout/MainPanelLayout';
import { Card } from '../ui/card';
import { Button } from '../ui/button';
import { ConfirmationModal } from '../ui/ConfirmationModal';

interface ProjectsViewProps {
  onSelectProject?: (projectPath: string, sessionId?: string) => void;
}

const ProjectsView: React.FC<ProjectsViewProps> = ({ onSelectProject }) => {
  const [projects, setProjects] = useState<ProjectInfoDisplay[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Delete confirmation modal state
  const [showDeleteConfirmation, setShowDeleteConfirmation] = useState(false);
  const [projectToDelete, setProjectToDelete] = useState<ProjectInfoDisplay | null>(null);

  const loadProjects = async () => {
    try {
      setLoading(true);
      setError(null);

      const response = await listProjects();

      if (response.data) {
        setProjects(response.data.projects || []);
      } else {
        throw new Error('Failed to load projects');
      }
    } catch (err) {
      console.error('Error loading projects:', err);
      setError('Failed to load projects. Please try again.');
      toast.error('Failed to load projects');
    } finally {
      setLoading(false);
    }
  };

  const handleDeleteProject = (project: ProjectInfoDisplay) => {
    setProjectToDelete(project);
    setShowDeleteConfirmation(true);
  };

  const handleConfirmDelete = async () => {
    if (!projectToDelete) return;

    setShowDeleteConfirmation(false);
    const projectPath = projectToDelete.path;
    const projectName = formatPath(projectToDelete.path);
    setProjectToDelete(null);

    try {
      const response = await removeProject({
        path: { path: projectPath },
      });

      if (response.response.ok) {
        toast.success(`Project "${projectName}" removed successfully`);
        loadProjects(); // Reload the list
      } else {
        throw new Error('Failed to remove project');
      }
    } catch (err) {
      console.error('Error removing project:', err);
      toast.error(`Failed to remove project "${projectName}"`);
    }
  };

  const handleCancelDelete = () => {
    setShowDeleteConfirmation(false);
    setProjectToDelete(null);
  };

  const handleProjectSelect = (project: ProjectInfoDisplay) => {
    if (onSelectProject) {
      onSelectProject(project.path, project.last_session_id || undefined);
    }
  };

  const formatDate = (dateString: string) => {
    try {
      return format(new Date(dateString), 'PPP p');
    } catch {
      return 'Invalid Date';
    }
  };

  const formatPath = (path: string) => {
    const parts = path.split('/').filter(Boolean);
    if (parts.length <= 2) {
      return path;
    }
    return `.../${parts.slice(-2).join('/')}`;
  };

  useEffect(() => {
    loadProjects();
  }, []);

  return (
    <>
      <MainPanelLayout>
        <div className="flex-1 flex flex-col min-h-0">
          <div className="bg-background-default px-8 pb-8 pt-16">
            <div className="flex flex-col page-transition">
              <div className="flex justify-between items-center mb-1">
                <h1 className="text-4xl font-light">Projects</h1>
                <Button onClick={loadProjects} disabled={loading} size="sm" className="h-8">
                  <RefreshCw className={`w-4 h-4 mr-2 ${loading ? 'animate-spin' : ''}`} />
                  Refresh
                </Button>
              </div>
              <p className="text-sm text-text-muted mb-4">
                Manage your Goose projects and resume previous sessions. Projects are automatically
                tracked when you use Goose in different directories.
              </p>
            </div>
          </div>

          <div className="flex-1 min-h-0 relative px-8">
            <div className="h-full relative">
              {loading ? (
                <div className="flex items-center justify-center h-64">
                  <div className="flex items-center space-x-2">
                    <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-blue-500"></div>
                    <span className="text-text-muted">Loading projects...</span>
                  </div>
                </div>
              ) : error ? (
                <div className="flex flex-col items-center justify-center h-64 space-y-4">
                  <div className="text-red-600 text-center">
                    <p className="text-lg font-medium">Failed to load projects</p>
                    <p className="text-sm">{error}</p>
                  </div>
                  <Button onClick={loadProjects} variant="default">
                    Try Again
                  </Button>
                </div>
              ) : projects.length === 0 ? (
                <div className="flex flex-col items-center justify-center h-64 space-y-4">
                  <FolderOpen className="w-16 h-16 text-text-muted" />
                  <div className="text-center max-w-md">
                    <p className="text-lg font-medium text-text-standard mb-2">No projects found</p>
                    <p className="text-sm text-text-muted mb-4">
                      Projects are automatically tracked when you start conversations with Goose in
                      different directories. Each project remembers your last session and
                      instructions for easy resumption.
                    </p>
                    <p className="text-xs text-text-muted">
                      💡 Tip: Start a new chat session in any directory to create your first
                      project!
                    </p>
                  </div>
                </div>
              ) : (
                <div className="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-4 pb-8">
                  {projects.map((project) => (
                    <Card key={project.path} className="p-6 hover:bg-accent/50 transition-colors">
                      <div className="flex items-start justify-between">
                        <div className="flex-1 min-w-0">
                          <div className="flex items-center mb-3">
                            <FolderOpen className="w-5 h-5 text-text-muted mr-2 flex-shrink-0" />
                            <h3 className="font-medium text-text-standard truncate">
                              {formatPath(project.path)}
                            </h3>
                          </div>

                          <div className="flex items-center text-sm text-text-muted mb-2">
                            <Clock className="w-4 h-4 mr-1 flex-shrink-0" />
                            <span className="truncate">{formatDate(project.last_accessed)}</span>
                          </div>

                          <div className="flex items-center space-x-2 mb-3">
                            {project.last_session_id ? (
                              <>
                                <Button
                                  onClick={(e) => {
                                    e.stopPropagation();
                                    handleProjectSelect(project);
                                  }}
                                  size="sm"
                                  className="h-8"
                                  title="Continue your previous conversation in this project"
                                >
                                  <Sparkles className="w-4 h-4 mr-1" />
                                  Resume
                                </Button>
                                <Button
                                  onClick={(e) => {
                                    e.stopPropagation();
                                    // Start fresh session (no session ID)
                                    if (onSelectProject) {
                                      onSelectProject(project.path, undefined);
                                    }
                                  }}
                                  size="sm"
                                  variant="outline"
                                  className="h-8"
                                  title="Start a new conversation in this project directory"
                                >
                                  <Plus className="w-4 h-4 mr-1" />
                                  Start Fresh
                                </Button>
                              </>
                            ) : (
                              <Button
                                onClick={(e) => {
                                  e.stopPropagation();
                                  handleProjectSelect(project);
                                }}
                                size="sm"
                                className="h-8"
                                title="Start a new conversation in this project directory"
                              >
                                <Plus className="w-4 h-4 mr-1" />
                                Use
                              </Button>
                            )}

                            {project.last_session_id && (
                              <span
                                className="text-xs text-text-muted bg-background-muted px-2 py-1 rounded"
                                title={`Session ID: ${project.last_session_id}`}
                              >
                                Session: {project.last_session_id.slice(0, 8)}...
                              </span>
                            )}
                          </div>
                        </div>

                        <Button
                          onClick={(e) => {
                            e.stopPropagation();
                            handleDeleteProject(project);
                          }}
                          variant="ghost"
                          size="sm"
                          className="h-8 text-text-muted hover:text-red-500 hover:bg-red-50 dark:hover:bg-red-900/20 flex-shrink-0 transition-colors"
                          title="Remove project"
                        >
                          <Trash2 className="w-4 h-4" />
                        </Button>
                      </div>
                    </Card>
                  ))}
                </div>
              )}
            </div>
          </div>
        </div>
      </MainPanelLayout>

      <ConfirmationModal
        isOpen={showDeleteConfirmation}
        title="Remove Project"
        message={`Are you sure you want to remove the project "${projectToDelete ? formatPath(projectToDelete.path) : ''}"? This will only remove it from your projects list - the actual files will not be deleted.`}
        confirmLabel="Remove Project"
        cancelLabel="Cancel"
        confirmVariant="destructive"
        onConfirm={handleConfirmDelete}
        onCancel={handleCancelDelete}
      />
    </>
  );
};

export default ProjectsView;
