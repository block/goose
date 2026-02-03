import { redirect } from "next/navigation";

type InvitePageParams = {
  token?: string;
};

type InvitePageProps = {
  params: Promise<InvitePageParams>;
  searchParams?: Promise<Record<string, string | string[] | undefined>>;
};

export default async function InviteRedirectPage({
  params,
  searchParams,
}: InvitePageProps): Promise<never> {
  const resolvedParams = await params;
  const rawToken = resolvedParams.token?.trim();

  if (!rawToken) {
    redirect("/invitations/accept");
  }

  const resolvedSearchParams = searchParams ? await searchParams : undefined;
  const invitationIdParam = resolvedSearchParams?.invitation_id;
  const invitationId = Array.isArray(invitationIdParam)
    ? invitationIdParam.at(0)
    : invitationIdParam;
  const normalizedInvitationId = invitationId && invitationId.trim().length ? invitationId : null;

  const organizationIdParam = resolvedSearchParams?.organization_id;
  const organizationId = Array.isArray(organizationIdParam)
    ? organizationIdParam.at(0)
    : organizationIdParam;
  const normalizedOrganizationId =
    organizationId && organizationId.trim().length ? organizationId : null;

  const redirectParams = new URLSearchParams({ token: rawToken });
  if (normalizedInvitationId) {
    redirectParams.set("invitation_id", normalizedInvitationId);
  }
  if (normalizedOrganizationId) {
    redirectParams.set("organization_id", normalizedOrganizationId);
  }

  redirect(`/invitations/accept?${redirectParams.toString()}`);
}
