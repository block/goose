"use client";

import { useParams } from "next/navigation";
import { TeamDetailSettings } from "@/features/settings/components/team-detail-settings";

export default function TeamDetailPage() {
  const params = useParams();
  const teamId = params.teamId as string;

  return (
    <div className="container mx-auto max-w-7xl px-6 py-8">
      <TeamDetailSettings teamId={teamId} />
    </div>
  );
}
