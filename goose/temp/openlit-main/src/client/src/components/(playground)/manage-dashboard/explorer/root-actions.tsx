import { Button } from "@/components/ui/button";
import {
  DropdownMenuTrigger,
  DropdownMenuItem,
  DropdownMenu,
  DropdownMenuContent
} from "@/components/ui/dropdown-menu";
import {
  MoreHorizontal, Upload
} from "lucide-react";
import { useState } from "react";
import ImportLayoutModal from "./import-layout-modal";

export default function RootActions({ openAddDialog, importBoardLayout }: {
  openAddDialog: () => void,
  importBoardLayout: (data: any) => Promise<unknown>
}) {
  const [isImportModalOpen, setIsImportModalOpen] = useState(false);

  return (
    <div className="flex gap-2">
      <Button variant="outline" className="bg-primary hover:bg-primary/90 text-white hover:text-white dark:text-white dark:hover:text-white dark:bg-primary dark:hover:bg-primary/90" onClick={() => openAddDialog()}>
        Create new dashboard
      </Button>
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button variant="ghost" size="icon" className="h-8 w-8 self-center">
            <MoreHorizontal className="h-4 w-4" />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          <DropdownMenuItem onClick={() => setIsImportModalOpen(true)}>
            <Upload className="h-4 w-4 mr-2" />
            Import Layout
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>
      <ImportLayoutModal
        open={isImportModalOpen}
        onClose={() => setIsImportModalOpen(false)}
        onImport={importBoardLayout as any}
      />
    </div>
  );
}