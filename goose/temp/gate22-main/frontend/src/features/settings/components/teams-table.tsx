"use client";

import { useEffect, useState, useMemo } from "react";
import { Team } from "@/features/teams/types/team.types";
import { listTeams } from "@/features/teams/api/team";
import { useMetaInfo } from "@/components/context/metainfo";
import { toast } from "sonner";
import { useRouter } from "next/navigation";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Search, Users, Plus } from "lucide-react";

interface TeamsTableProps {
  refreshKey?: number;
}

export function TeamsTable({ refreshKey = 0 }: TeamsTableProps) {
  const { accessToken, activeOrg } = useMetaInfo();
  const router = useRouter();
  const [teams, setTeams] = useState<Team[]>([]);
  const [loading, setLoading] = useState(true);
  const [searchQuery, setSearchQuery] = useState("");

  const fetchTeams = useMemo(
    () => async () => {
      setLoading(true);
      try {
        const data = await listTeams(accessToken, activeOrg.orgId);
        setTeams(data);
      } catch {
        toast.error("Failed to load teams");
      } finally {
        setLoading(false);
      }
    },
    [accessToken, activeOrg.orgId],
  );

  useEffect(() => {
    fetchTeams();
  }, [fetchTeams, refreshKey]);

  const handleViewTeam = (teamId: string) => {
    router.push(`/teams/${teamId}`);
  };

  const filteredTeams = useMemo(() => {
    return teams.filter((team) => {
      const matchesSearch =
        team.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
        team.description?.toLowerCase().includes(searchQuery.toLowerCase());

      return matchesSearch;
    });
  }, [teams, searchQuery]);

  const getInitials = (name: string) => {
    const words = name.split(" ");
    if (words.length >= 2) {
      return `${words[0][0]}${words[1][0]}`.toUpperCase();
    }
    return name.slice(0, 2).toUpperCase();
  };

  if (loading) {
    return (
      <div className="flex h-64 items-center justify-center">
        <div className="text-muted-foreground">Loading teams...</div>
      </div>
    );
  }

  if (teams.length === 0) {
    return (
      <div className="rounded-lg border">
        <div className="flex flex-col items-center justify-center py-12">
          <Users className="mb-4 h-12 w-12 text-muted-foreground" />
          <h3 className="mb-1 font-medium">No teams yet</h3>
          <p className="mb-4 text-sm text-muted-foreground">
            Create your first team to get started
          </p>
          <Button onClick={() => router.push("/teams/new")}>
            <Plus className="mr-2 h-4 w-4" />
            Create Team
          </Button>
        </div>
      </div>
    );
  }

  return (
    <>
      <div className="space-y-4">
        <div className="flex items-center gap-4">
          <div className="relative max-w-sm flex-1">
            <Search className="absolute top-1/2 left-3 h-4 w-4 -translate-y-1/2 transform text-muted-foreground" />
            <Input
              placeholder="Filter..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="pl-9"
            />
          </div>
        </div>

        <div className="rounded-lg border">
          <div className="divide-y">
            {filteredTeams.map((team) => (
              <div
                key={team.team_id}
                className="flex items-center justify-between p-4 transition-colors hover:bg-muted/20"
              >
                <div className="flex flex-1 items-center gap-4">
                  <Avatar className="h-10 w-10">
                    <AvatarFallback className="bg-gray-100 font-medium text-gray-900 dark:bg-gray-800 dark:text-gray-100">
                      {getInitials(team.name)}
                    </AvatarFallback>
                  </Avatar>
                  <div className="flex-1">
                    <div className="font-medium">{team.name}</div>
                    <div className="text-sm text-muted-foreground">
                      {team.description || "No description"}
                    </div>
                  </div>
                </div>

                <Button variant="outline" size="sm" onClick={() => handleViewTeam(team.team_id)}>
                  Manage
                </Button>
              </div>
            ))}
          </div>
        </div>
      </div>
    </>
  );
}
