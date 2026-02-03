from abc import ABC, abstractmethod

from src.rules.models import Rule


class RuleLoader(ABC):
    """
    Abstract interface for fetching rules from a repository.

    This interface allows us to swap out different rule sources
    (GitHub files, database, etc.) without changing the application logic.
    """

    @abstractmethod
    async def get_rules(self, repository: str, installation_id: int) -> list[Rule]:
        """
        Fetch rules for a specific repository.

        Args:
            repository: The repository in format "owner/repo"
            installation_id: The GitHub App installation ID for authentication

        Returns:
            list of Rule objects for the repository
        """
        pass
