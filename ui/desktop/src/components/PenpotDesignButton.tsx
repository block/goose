import { Palette } from 'lucide-react';
import { Button } from './ui/button';
import { Tooltip, TooltipTrigger, TooltipContent } from './ui/Tooltip';
import { useSidecar } from './SidecarLayout';

interface PenpotDesignButtonProps {
  projectId?: string;
  fileId?: string;
  initialDesign?: string;
  className?: string;
}

export function PenpotDesignButton({ 
  projectId, 
  fileId, 
  initialDesign, 
  className = '' 
}: PenpotDesignButtonProps) {
  const sidecar = useSidecar();

  if (!sidecar) return null;

  const handleOpenPenpot = () => {
    sidecar.showPenpotDesigner(projectId, fileId, initialDesign);
  };

  return (
    <div className={`absolute top-2 right-2 z-50 ${className}`}>
      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            onClick={handleOpenPenpot}
            variant="ghost"
            size="sm"
            className="p-2 bg-background-muted hover:bg-background-subtle border border-borderSubtle rounded-lg transition-all duration-200 hover:scale-105 text-textSubtle hover:text-primary cursor-pointer focus:outline-none focus:ring-1 focus:ring-borderProminent"
          >
            <Palette size={16} />
          </Button>
        </TooltipTrigger>
        <TooltipContent side="left">Open Penpot Designer</TooltipContent>
      </Tooltip>
    </div>
  );
}

export default PenpotDesignButton;
