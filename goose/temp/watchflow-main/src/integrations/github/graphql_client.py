import httpx
import structlog
from pydantic import BaseModel

logger = structlog.get_logger(__name__)


class Commit(BaseModel):
    oid: str
    message: str
    author: str


class Review(BaseModel):
    state: str
    author: str


class Comment(BaseModel):
    author: str
    body: str


class PRContext(BaseModel):
    commits: list[Commit]
    reviews: list[Review]
    comments: list[Comment]


class GitHubGraphQLClient:
    def __init__(self, token: str):
        self.token = token
        self.endpoint = "https://api.github.com/graphql"

    def fetch_pr_context(self, owner: str, repo: str, pr_number: int) -> PRContext:
        """
        Fetches the context of a pull request from GitHub's GraphQL API.
        """
        query = """
        query PRContext($owner: String!, $repo: String!, $pr_number: Int!) {
            repository(owner: $owner, name: $repo) {
                pullRequest(number: $pr_number) {
                    commits(first: 100) {
                        nodes {
                            commit {
                                oid
                                message
                                author {
                                    name
                                }
                            }
                        }
                    }
                    reviews(first: 50) {
                        nodes {
                            state
                            author {
                                login
                            }
                        }
                    }
                    comments(first: 100) {
                        nodes {
                            author {
                                login
                            }
                            body
                        }
                    }
                }
            }
        }
        """
        variables = {"owner": owner, "repo": repo, "pr_number": pr_number}
        headers = {"Authorization": f"bearer {self.token}"}

        try:
            with httpx.Client() as client:
                response = client.post(self.endpoint, json={"query": query, "variables": variables}, headers=headers)
                response.raise_for_status()
                data = response.json()

                if "errors" in data:
                    logger.error("GraphQL query failed", errors=data["errors"])
                    raise Exception("GraphQL query failed")

                pr_data = data["data"]["repository"]["pullRequest"]

                commits = [
                    Commit(
                        oid=node["commit"]["oid"],
                        message=node["commit"]["message"],
                        author=node["commit"].get("author", {}).get("name", "Unknown"),
                    )
                    for node in pr_data["commits"]["nodes"]
                ]

                reviews = [
                    Review(
                        state=node["state"],
                        author=node["author"]["login"] if node.get("author") else "unknown",
                    )
                    for node in pr_data["reviews"]["nodes"]
                ]

                comments = [
                    Comment(
                        author=node["author"]["login"] if node.get("author") else "unknown",
                        body=node["body"],
                    )
                    for node in pr_data["comments"]["nodes"]
                ]

                return PRContext(commits=commits, reviews=reviews, comments=comments)

        except httpx.HTTPStatusError as e:
            logger.error("HTTP error fetching PR context", exc_info=e)
            raise
        except Exception as e:
            logger.error("An unexpected error occurred", exc_info=e)
            raise
