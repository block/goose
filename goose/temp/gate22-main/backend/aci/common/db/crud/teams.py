from uuid import UUID

from sqlalchemy.orm import Session
from sqlalchemy.sql import select

from aci.common.db.sql_models import Team, TeamMembership
from aci.common.enums import TeamRole


def create_team(
    db_session: Session,
    organization_id: UUID,
    name: str,
    description: str | None = None,
) -> Team:
    team = Team(organization_id=organization_id, name=name, description=description)
    db_session.add(team)
    db_session.flush()
    db_session.refresh(team)
    return team


def get_teams_by_organization_id(
    db_session: Session,
    organization_id: UUID,
) -> list[Team]:
    return (
        db_session.query(Team)
        .filter(Team.organization_id == organization_id)
        .order_by(Team.created_at.desc())
        .all()
    )


def get_team_by_id(
    db_session: Session,
    team_id: UUID,
) -> Team | None:
    return db_session.query(Team).filter(Team.id == team_id).first()


def get_teams_by_ids(
    db_session: Session,
    team_ids: list[UUID],
) -> list[Team]:
    if not team_ids:
        return []
    statement = select(Team).where(Team.id.in_(team_ids))
    results = db_session.execute(statement).scalars().all()

    # map the rows by id, and use the order of requested ids to map the final results
    results_by_id = {result.id: result for result in results}
    return [results_by_id[team_id] for team_id in team_ids if team_id in results_by_id]


def get_team_by_organization_id_and_name(
    db_session: Session,
    organization_id: UUID,
    name: str,
) -> Team | None:
    return (
        db_session.query(Team)
        .filter(Team.organization_id == organization_id)
        .filter(Team.name == name)
        .first()
    )


def add_team_member(
    db_session: Session,
    organization_id: UUID,
    team_id: UUID,
    user_id: UUID,
) -> None:
    team_member = TeamMembership(
        team_id=team_id, organization_id=organization_id, user_id=user_id, role=TeamRole.MEMBER
    )
    db_session.add(team_member)
    db_session.flush()


def remove_team_member(
    db_session: Session,
    organization_id: UUID,
    team_id: UUID,
    user_id: UUID,
) -> None:
    db_session.query(TeamMembership).filter(
        TeamMembership.team_id == team_id,
        TeamMembership.organization_id == organization_id,
        TeamMembership.user_id == user_id,
    ).delete()


def get_team_members(
    db_session: Session,
    team_id: UUID,
) -> list[TeamMembership]:
    return (
        db_session.query(TeamMembership)
        .filter(TeamMembership.team_id == team_id)
        .order_by(TeamMembership.created_at.desc())
        .all()
    )


def get_teams_by_user_id(
    db_session: Session,
    organization_id: UUID,
    user_id: UUID,
) -> list[Team]:
    return (
        db_session.query(Team)
        .filter(
            Team.organization_id == organization_id,
            Team.memberships.any(TeamMembership.user_id == user_id),
        )
        .order_by(Team.created_at.desc())
        .all()
    )
