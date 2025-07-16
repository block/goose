import React, { useState } from 'react';
import ProjectsView from './ProjectsView';
import ProjectDetailsView from './ProjectDetailsView';
import { View, ViewOptions } from '../../App';

interface ProjectsContainerProps {
  setView: (view: View, viewOptions?: ViewOptions) => void;
}

const ProjectsContainer: React.FC<ProjectsContainerProps> = ({ setView }) => {
  const [selectedProjectId, setSelectedProjectId] = useState<string | null>(null);
  const [refreshTrigger, setRefreshTrigger] = useState(0);

  const handleSelectProject = (projectId: string) => {
    setSelectedProjectId(projectId);
  };

  const handleBack = () => {
    setSelectedProjectId(null);
    // Trigger a refresh of the projects list when returning from details
    setRefreshTrigger((prev) => prev + 1);
  };

  if (selectedProjectId) {
    return (
      <ProjectDetailsView projectId={selectedProjectId} onBack={handleBack} setView={setView} />
    );
  }

  return <ProjectsView onSelectProject={handleSelectProject} refreshTrigger={refreshTrigger} />;
};

export default ProjectsContainer;
