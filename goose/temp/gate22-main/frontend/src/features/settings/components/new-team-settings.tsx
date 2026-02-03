"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { useCreateTeam } from "../hooks/use-create-team";
import { SETTINGS_ROUTES } from "../constants";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

export function NewTeamSettings() {
  const router = useRouter();
  const [teamName, setTeamName] = useState("");
  const [teamDescription, setTeamDescription] = useState("");
  const { createTeam, isCreating } = useCreateTeam();

  const handleCreateTeam = () => {
    createTeam({
      name: teamName.trim(),
      description: teamDescription.trim() || undefined,
    });
  };

  return (
    <>
      <div className="mb-8">
        <h2 className="text-3xl font-bold tracking-tight">Create New Team</h2>
        <p className="mt-2 text-muted-foreground">Set up a new team for your organization</p>
      </div>

      <div className="space-y-6">
        <Card>
          <CardHeader>
            <CardTitle>Team Details</CardTitle>
            <CardDescription>Configure your team&apos;s name and basic information</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="team-name">Team Name *</Label>
                <Input
                  id="team-name"
                  placeholder="Enter team name (e.g., Engineering, Marketing)"
                  value={teamName}
                  onChange={(e) => setTeamName(e.target.value)}
                  disabled={isCreating}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && !isCreating) {
                      handleCreateTeam();
                    }
                  }}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="team-description">Description (Optional)</Label>
                <Textarea
                  id="team-description"
                  placeholder="Brief description of the team's purpose and responsibilities"
                  value={teamDescription}
                  onChange={(e) => setTeamDescription(e.target.value)}
                  disabled={isCreating}
                  rows={3}
                />
              </div>
            </div>
          </CardContent>
        </Card>

        <div className="flex justify-end gap-3">
          <Button
            variant="outline"
            onClick={() => router.push(SETTINGS_ROUTES.TEAMS)}
            disabled={isCreating}
          >
            Cancel
          </Button>
          <Button onClick={handleCreateTeam} disabled={!teamName.trim() || isCreating}>
            {isCreating ? "Creating..." : "Create Team"}
          </Button>
        </div>
      </div>
    </>
  );
}
