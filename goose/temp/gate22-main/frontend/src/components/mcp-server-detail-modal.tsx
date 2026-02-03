import { MCPIntegration } from "@/data/mcp-integrations";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";
import { ExternalLink, Github, Shield, Wrench } from "lucide-react";
import Image from "next/image";

interface MCPServerDetailModalProps {
  server: MCPIntegration | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function MCPServerDetailModal({ server, open, onOpenChange }: MCPServerDetailModalProps) {
  if (!server) return null;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[80vh] max-w-2xl overflow-y-auto">
        <DialogHeader>
          <div className="mb-4 flex items-center gap-4">
            <div className="relative h-16 w-16 shrink-0">
              <Image
                src={server.iconUrl}
                alt={`${server.name} logo`}
                width={64}
                height={64}
                className="object-contain"
                unoptimized
              />
            </div>
            <div className="flex-1">
              <DialogTitle className="mb-1 text-2xl">{server.name}</DialogTitle>
              <div className="flex items-center gap-4 text-sm text-muted-foreground">
                <span>by {server.provider}</span>
                {server.authType && (
                  <div className="flex items-center gap-1">
                    <Shield className="h-3 w-3" />
                    <span>{server.authType}</span>
                  </div>
                )}
              </div>
            </div>
          </div>
          <DialogDescription className="text-base">{server.description}</DialogDescription>
        </DialogHeader>

        <div className="mt-6 space-y-6">
          {/* Categories */}
          <div>
            <h3 className="mb-2 text-sm font-semibold">Categories</h3>
            <div className="flex flex-wrap gap-2">
              {server.categories.map((category) => (
                <Badge key={category} variant="secondary">
                  {category}
                </Badge>
              ))}
            </div>
          </div>

          <Separator />

          {/* Tools */}
          <div>
            <div className="mb-3 flex items-center gap-2">
              <Wrench className="h-4 w-4" />
              <h3 className="text-sm font-semibold">Available Tools ({server.tools.count})</h3>
            </div>
            <div className="space-y-2">
              {server.tools.examples.map((tool, index) => (
                <div key={index} className="flex items-start gap-2 rounded-md bg-muted/50 p-2">
                  <div className="mt-1.5 h-2 w-2 shrink-0 rounded-full bg-primary" />
                  <div className="flex-1">
                    <div className="text-sm font-medium">{tool}</div>
                    {server.tools.descriptions && server.tools.descriptions[index] && (
                      <div className="mt-0.5 text-xs whitespace-pre-wrap text-muted-foreground">
                        {server.tools.descriptions[index]}
                      </div>
                    )}
                  </div>
                </div>
              ))}
            </div>
          </div>

          <Separator />

          {/* Actions */}
          <div className="flex gap-3">
            {server.repoUrl && (
              <Button variant="outline" asChild>
                <a href={server.repoUrl} target="_blank" rel="noopener noreferrer">
                  <Github className="mr-2 h-4 w-4" />
                  View Repository
                </a>
              </Button>
            )}
            {server.docsUrl && (
              <Button variant="outline" asChild>
                <a href={server.docsUrl} target="_blank" rel="noopener noreferrer">
                  <ExternalLink className="mr-2 h-4 w-4" />
                  Documentation
                </a>
              </Button>
            )}
            <Button className="ml-auto">
              <ExternalLink className="mr-2 h-4 w-4" />
              Configure Server
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
