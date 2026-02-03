import uuid

import click
from rich.console import Console

from aci.cli import config
from aci.common import utils
from aci.common.db import crud
from aci.common.enums import OrganizationRole, UserIdentityProvider
from aci.common.schemas.auth import ActAsInfo

console = Console()


@click.command()
@click.option(
    "--skip-dry-run",
    is_flag=True,
    help="provide this flag to run the command and apply changes to the database",
)
def create_mock_org_teams_users(
    skip_dry_run: bool,
) -> None:
    """
    Create a mock organization with teams and users in db.

    This creates the following mock settings:

    Organization: ACI Dev Org

    Users:
    - Admin (admin@aci.dev) - Admin role
    - User1 (user1@aci.dev) - Member role
    - User2 (user2@aci.dev) - Member role

    Teams:
    - Team 1 (Admin, User1)
    - Team 2 (Admin, User1, User2)
    """

    short_id = uuid.uuid4().hex[:8]

    with utils.create_db_session(config.DB_FULL_URL) as db_session:
        # Create organization
        organization = crud.organizations.create_organization(
            db_session, f"ACI Dev Org {short_id}", f"ACI Dev Org {short_id} Description"
        )

        # Create users
        admin = crud.users.create_user(
            db_session,
            name="ACI Admin",
            email=f"admin-{short_id}@aci.dev",
            password_hash=utils.hash_user_password("password"),
            identity_provider=UserIdentityProvider.EMAIL,
            email_verified=True,
        )
        user1 = crud.users.create_user(
            db_session,
            name="ACI User 1",
            email=f"user1-{short_id}@aci.dev",
            password_hash=utils.hash_user_password("password"),
            identity_provider=UserIdentityProvider.EMAIL,
            email_verified=True,
        )
        user2 = crud.users.create_user(
            db_session,
            name="ACI User 2",
            email=f"user2-{short_id}@aci.dev",
            password_hash=utils.hash_user_password("password"),
            identity_provider=UserIdentityProvider.EMAIL,
            email_verified=True,
        )

        # Add users to organization
        crud.organizations.add_user_to_organization(
            db_session, organization.id, admin.id, OrganizationRole.ADMIN
        )
        crud.organizations.add_user_to_organization(
            db_session, organization.id, user1.id, OrganizationRole.MEMBER
        )
        crud.organizations.add_user_to_organization(
            db_session, organization.id, user2.id, OrganizationRole.MEMBER
        )

        # Create teams
        team1 = crud.teams.create_team(db_session, organization.id, f"Team 1 - {short_id}")
        crud.teams.add_team_member(db_session, organization.id, team1.id, admin.id)
        crud.teams.add_team_member(db_session, organization.id, team1.id, user1.id)

        team2 = crud.teams.create_team(db_session, organization.id, f"Team 2 - {short_id}")
        crud.teams.add_team_member(db_session, organization.id, team2.id, admin.id)
        crud.teams.add_team_member(db_session, organization.id, team2.id, user1.id)
        crud.teams.add_team_member(db_session, organization.id, team2.id, user2.id)

        # Important: not adding these configs to cli/config.py as these should only be used in local
        # development. These not be used by any other code in cli, and would not be accessible in
        # actual deployment
        jwt_signing_key = utils.check_and_get_env_variable("CONTROL_PLANE_JWT_SIGNING_KEY")
        jwt_algorithm = "HS256"
        jwt_access_token_expire_minutes = 1440

        # Create JWTs
        jwt_admin = utils.sign_token(
            user=admin,
            act_as=ActAsInfo(
                organization_id=organization.id,
                role=OrganizationRole.ADMIN,
            ),
            jwt_signing_key=jwt_signing_key,
            jwt_algorithm=jwt_algorithm,
            jwt_access_token_expire_minutes=jwt_access_token_expire_minutes,
        )
        jwt_user1 = utils.sign_token(
            user=user1,
            act_as=ActAsInfo(
                organization_id=organization.id,
                role=OrganizationRole.MEMBER,
            ),
            jwt_signing_key=jwt_signing_key,
            jwt_algorithm=jwt_algorithm,
            jwt_access_token_expire_minutes=jwt_access_token_expire_minutes,
        )
        jwt_user2 = utils.sign_token(
            user=user2,
            act_as=ActAsInfo(
                organization_id=organization.id,
                role=OrganizationRole.MEMBER,
            ),
            jwt_signing_key=jwt_signing_key,
            jwt_algorithm=jwt_algorithm,
            jwt_access_token_expire_minutes=jwt_access_token_expire_minutes,
        )

        if not skip_dry_run:
            console.rule(
                "[bold green]Provide --skip-dry-run to create mock organization, team and users[/bold green]"  # noqa: E501
            )
            db_session.rollback()
        else:
            db_session.commit()
            console.rule("[bold green]Organization created with following hierarchy[/bold green]")
            console.print(f"""Organization:
ACI Dev Org {short_id} (ID: {organization.id})

Users:
- Admin (admin-{short_id}@aci.dev) (ID: {admin.id}) - Admin role
- User1 (user1-{short_id}@aci.dev) (ID: {user1.id}) - Member role
- User2 (user2-{short_id}@aci.dev) (ID: {user2.id}) - Member role

Teams:
- Team 1 - {short_id} (ID: {team1.id}) (Admin, User1)
- Team 2 - {short_id} (ID: {team2.id}) (Admin, User1, User2)
            """)
            console.print(
                f"Admin ({admin.id}) JWT:\n{jwt_admin}\n",
                style="bold yellow",
                overflow="ignore",
                soft_wrap=True,
            )
            console.print(
                f"User 1 ({user1.id}) JWT:\n{jwt_user1}\n",
                style="bold yellow",
                overflow="ignore",
                soft_wrap=True,
            )
            console.print(
                f"User 2 ({user2.id}) JWT:\n{jwt_user2}\n",
                style="bold yellow",
                overflow="ignore",
                soft_wrap=True,
            )

            console.print("You can create mock configuration by:")

            console.print(
                "docker compose exec runner python -m aci.cli create-mock-mcp-configuration "
                f"--mcp-server NOTION --user-id {admin.id} --team-id {team1.id}",
                style="bold yellow",
                overflow="ignore",
                soft_wrap=True,
            )
