import React from 'react';
import { ProjectMetadata } from '../../projects';
import { Card, CardHeader, CardTitle, CardContent } from '../ui/card';
import { Folder, Calendar } from 'lucide-react';
import { formatDistanceToNow } from 'date-fns';

interface ProjectCardProps {
  project: ProjectMetadata;
  onClick: () => void;
  onRefresh: () => void;
}

const ProjectCard: React.FC<ProjectCardProps> = ({ project, onClick }) => {
  return (
    <Card
      className="transition-all duration-200 hover:shadow-default hover:cursor-pointer min-h-[140px] flex flex-col"
      onClick={onClick}
    >
      <CardHeader className="pb-2">
        <CardTitle className="flex items-center gap-2">
          <Folder className="w-4 h-4 text-text-muted flex-shrink-0" />
          {project.name}
        </CardTitle>
      </CardHeader>

      <CardContent className="px-4 text-sm flex-grow flex flex-col justify-between">
        {project.description && (
          <div className="mb-2">
            <span className="text-text-muted line-clamp-2">{project.description}</span>
          </div>
        )}

        <div className="flex items-center gap-4 text-xs text-text-muted mt-auto">
          <div className="flex items-center">
            <Calendar className="w-3 h-3 mr-1 flex-shrink-0" />
            <span>{formatDistanceToNow(new Date(project.updatedAt))} ago</span>
          </div>
          <span>
            {project.sessionCount} {project.sessionCount === 1 ? 'session' : 'sessions'}
          </span>
        </div>
      </CardContent>
    </Card>
  );
};

export default ProjectCard;
