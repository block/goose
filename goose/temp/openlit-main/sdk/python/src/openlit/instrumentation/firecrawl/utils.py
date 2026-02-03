"""
Utility functions for Firecrawl instrumentation
"""

import logging
from typing import Dict, Any, Optional

from opentelemetry.trace import Status, StatusCode

from openlit.__helpers import (
    common_framework_span_attributes,
)
from openlit.semcov import SemanticConvention

# Initialize logger for logging potential issues and operations
logger = logging.getLogger(__name__)

# Comprehensive operation mapping for all Firecrawl endpoints following the framework guide pattern
FIRECRAWL_OPERATION_MAP = {
    # === CORE SCRAPING OPERATIONS ===
    "firecrawl.scrape_url": "scrape",
    "firecrawl.async_scrape_url": "scrape",
    # === CRAWLING OPERATIONS ===
    "firecrawl.crawl_url": "crawl",
    "firecrawl.async_crawl_url": "crawl",
    # === STATUS OPERATIONS ===
    "firecrawl.get_crawl_status": "crawl_status",
    "firecrawl.async_get_crawl_status": "crawl_status",
    "firecrawl.get_scrape_status": "scrape_status",
    "firecrawl.async_get_scrape_status": "scrape_status",
    # === MAPPING OPERATIONS ===
    "firecrawl.map_url": "map",
    "firecrawl.async_map_url": "map",
    # === SEARCH OPERATIONS ===
    "firecrawl.search": "search",
    "firecrawl.async_search": "search",
    # === BATCH OPERATIONS ===
    "firecrawl.batch_scrape_urls": "scrape",
    "firecrawl.async_batch_scrape_urls": "scrape",
    # === CANCEL OPERATIONS ===
    "firecrawl.cancel_crawl": "crawl",
    "firecrawl.async_cancel_crawl": "crawl",
}


class FirecrawlInstrumentationContext:
    """Context object to cache expensive extractions and reduce performance overhead."""

    __slots__ = (
        "instance",
        "args",
        "kwargs",
        "version",
        "environment",
        "application_name",
        "_url",
        "_urls",
        "_formats",
        "_api_key_masked",
    )

    def __init__(self, instance, args, kwargs, version, environment, application_name):
        self.instance = instance
        self.args = args
        self.kwargs = kwargs
        self.version = version
        self.environment = environment
        self.application_name = application_name

        # Cache expensive operations with lazy loading
        self._url = None
        self._urls = None
        self._formats = None
        self._api_key_masked = None

    @property
    def url(self) -> str:
        """Get primary URL with caching."""
        if self._url is None:
            if len(self.args) > 0:
                self._url = str(self.args[0])
            elif "url" in self.kwargs:
                self._url = str(self.kwargs["url"])
            else:
                self._url = "unknown"
        return self._url

    @property
    def urls(self) -> Optional[list]:
        """Get URLs list for batch operations with caching."""
        if self._urls is None:
            if len(self.args) > 0 and isinstance(self.args[0], list):
                self._urls = [str(url) for url in self.args[0]]
            elif "urls" in self.kwargs and isinstance(self.kwargs["urls"], list):
                self._urls = [str(url) for url in self.kwargs["urls"]]
            else:
                self._urls = []
        return self._urls

    @property
    def formats(self) -> Optional[list]:
        """Get requested formats with caching."""
        if self._formats is None:
            self._formats = self.kwargs.get("formats", [])
        return self._formats

    @property
    def api_key_masked(self) -> str:
        """Get masked API key for security."""
        if self._api_key_masked is None:
            if hasattr(self.instance, "api_key") and self.instance.api_key:
                key = self.instance.api_key
                if len(key) > 8:
                    self._api_key_masked = f"{key[:4]}...{key[-4:]}"
                else:
                    self._api_key_masked = "***"
            else:
                self._api_key_masked = "unknown"
        return self._api_key_masked


def get_operation_name(endpoint: str) -> str:
    """Get operation name from endpoint using the operation map."""
    return FIRECRAWL_OPERATION_MAP.get(endpoint, "scrape")


def get_span_name(
    operation_name: str, ctx: FirecrawlInstrumentationContext, endpoint: str
) -> str:
    """Create span name following the '{operation_type} {target}' format."""

    # Handle batch operations
    if "batch" in endpoint and ctx.urls:
        if len(ctx.urls) <= 3:
            urls_str = ", ".join(ctx.urls)
            if len(urls_str) <= 80:  # Keep reasonable length
                return f"{operation_name} [{urls_str}]"

        # For larger batches, show count and sample
        sample_urls = ctx.urls[:2]
        remaining_count = len(ctx.urls) - 2
        if remaining_count > 0:
            urls_sample = ", ".join(sample_urls) + f" +{remaining_count} more"
        else:
            urls_sample = ", ".join(sample_urls)
        return f"{operation_name} [{urls_sample}]"

    # Single URL operation
    target_url = ctx.url if ctx.url != "unknown" else "unknown"
    return f"{operation_name} {target_url}"


def set_span_attributes(
    span,
    operation_name: str,
    ctx: FirecrawlInstrumentationContext,
    endpoint: Optional[str] = None,
    pricing_info: Optional[Dict[str, Any]] = None,
    trace_content: bool = True,
    **kwargs,
):
    """Set comprehensive span attributes for Firecrawl operations."""
    try:
        # Set framework attributes using common_framework_span_attributes
        scope = type("GenericScope", (), {})()
        scope._span = span
        scope._start_time = getattr(span, "start_time", None)
        scope._end_time = getattr(span, "end_time", None)

        # Set common framework attributes
        common_framework_span_attributes(
            scope,
            SemanticConvention.GEN_AI_SYSTEM_FIRECRAWL,
            "api.firecrawl.dev",  # server_address
            443,  # server_port (HTTPS)
            ctx.environment,
            ctx.application_name,
            ctx.version,
            operation_name,  # Use operation_name instead of endpoint for clean names
        )

        # Set operation type using semantic conventions
        operation_type_map = {
            "scrape": SemanticConvention.GEN_AI_OPERATION_TYPE_SCRAPE,
            "crawl": SemanticConvention.GEN_AI_OPERATION_TYPE_CRAWL,
            "search": SemanticConvention.GEN_AI_OPERATION_TYPE_SEARCH,
            "map": SemanticConvention.GEN_AI_OPERATION_TYPE_MAP,
            "crawl_status": SemanticConvention.GEN_AI_OPERATION_TYPE_CRAWL_STATUS,
            "scrape_status": SemanticConvention.GEN_AI_OPERATION_TYPE_SCRAPE_STATUS,
        }
        operation_type = operation_type_map.get(operation_name, operation_name)
        span.set_attribute(SemanticConvention.GEN_AI_OPERATION, operation_type)

        # Set agent type for web scraping
        span.set_attribute(
            SemanticConvention.GEN_AI_AGENT_TYPE,
            SemanticConvention.GEN_AI_AGENT_TYPE_BROWSER,
        )

        # URL information
        if "batch" in endpoint and ctx.urls:
            span.set_attribute(SemanticConvention.GEN_AI_CRAWL_URL_COUNT, len(ctx.urls))
            # Set primary URL as browse URL
            if ctx.urls:
                span.set_attribute(
                    SemanticConvention.GEN_AI_AGENT_BROWSE_URL, ctx.urls[0]
                )
        elif ctx.url and ctx.url != "unknown":
            span.set_attribute(SemanticConvention.GEN_AI_AGENT_BROWSE_URL, ctx.url)

    except Exception as e:
        logger.debug("Failed to set span attributes: %s", e)


def format_content(content, max_length: int = 1000) -> str:
    """Format content for tracing with length limits."""
    if content is None:
        return ""

    content_str = str(content)
    if len(content_str) > max_length:
        return content_str[:max_length] + "..."
    return content_str


def process_response(
    span,
    response,
    ctx: FirecrawlInstrumentationContext,
    endpoint: Optional[str] = None,
    pricing_info: Optional[Dict[str, Any]] = None,
    trace_content: bool = True,
    **kwargs,
):
    """Process Firecrawl response and extract business intelligence."""
    try:
        if response is None:
            return

        # Handle different response types
        if isinstance(response, dict):
            # Single scrape/crawl response as dict
            _process_single_response(span, response, ctx, trace_content)
            span.set_status(Status(StatusCode.OK))
        elif isinstance(response, list):
            # Batch response
            span.set_attribute(SemanticConvention.GEN_AI_CRAWL_URL_COUNT, len(response))
            span.set_attribute(SemanticConvention.GEN_AI_CRAWL_RESULT_SUCCESS, True)

            # Process batch metrics
            success_count = sum(
                1
                for item in response
                if isinstance(item, dict) and item.get("success", True)
            )
            span.set_attribute("gen_ai.crawl.result.success_count", success_count)
            span.set_attribute(
                "gen_ai.crawl.result.success_rate",
                success_count / len(response) if response else 0,
            )

            # Capture batch content summary if enabled
            if trace_content:
                completion_summary = (
                    f"Processed {len(response)} URLs, {success_count} successful"
                )
                span.set_attribute(
                    SemanticConvention.GEN_AI_CONTENT_COMPLETION, completion_summary
                )
            span.set_status(Status(StatusCode.OK))
        elif hasattr(response, "__dict__") or hasattr(response, "success"):
            # ScrapeResponse or other response objects
            response_dict = {}

            # Convert object to dict for processing
            if hasattr(response, "__dict__"):
                response_dict = response.__dict__
            else:
                # Extract common attributes
                for attr in [
                    "success",
                    "markdown",
                    "html",
                    "text",
                    "metadata",
                    "links",
                    "screenshot",
                ]:
                    if hasattr(response, attr):
                        response_dict[attr] = getattr(response, attr)

            _process_single_response(span, response_dict, ctx, trace_content)
            span.set_status(Status(StatusCode.OK))
        else:
            # Unknown response type
            span.set_attribute("gen_ai.response.type", type(response).__name__)
            span.set_status(Status(StatusCode.OK))

    except Exception as e:
        logger.debug("Error processing response: %s", e)
        span.set_status(Status(StatusCode.ERROR, str(e)))


def _process_single_response(
    span, response: dict, ctx: FirecrawlInstrumentationContext, trace_content: bool
):
    """Process a single response object."""
    try:
        # === CORE RESPONSE STATUS ===
        success = response.get("success", True)
        span.set_attribute(SemanticConvention.GEN_AI_CRAWL_RESULT_SUCCESS, success)

        # Warning and error messages
        if "warning" in response and response["warning"]:
            span.set_attribute("gen_ai.response.warning", response["warning"])
        if "error" in response and response["error"]:
            span.set_attribute("gen_ai.response.error", response["error"])

        # === METADATA ATTRIBUTES ===
        if "metadata" in response:
            metadata = response["metadata"]
            if "title" in metadata:
                span.set_attribute("gen_ai.response.title", metadata["title"])
            if "description" in metadata:
                span.set_attribute(
                    "gen_ai.response.description", metadata["description"]
                )
            if "statusCode" in metadata:
                span.set_attribute(
                    SemanticConvention.GEN_AI_CRAWL_RESULT_STATUS_CODE,
                    metadata["statusCode"],
                )
            if "error" in metadata:
                span.set_attribute("gen_ai.response.error", metadata["error"])

        # === CONTENT LENGTH METRICS ===
        if "markdown" in response and response["markdown"]:
            span.set_attribute(
                SemanticConvention.GEN_AI_CRAWL_RESULT_MARKDOWN_LENGTH,
                len(response["markdown"]),
            )
        elif "html" in response and response["html"]:
            span.set_attribute(
                SemanticConvention.GEN_AI_CRAWL_RESULT_HTML_LENGTH,
                len(response["html"]),
            )
        elif "rawHtml" in response and response["rawHtml"]:
            span.set_attribute(
                SemanticConvention.GEN_AI_CRAWL_RESULT_HTML_LENGTH,
                len(response["rawHtml"]),
            )

        # === LINKS AND MEDIA COUNT ===
        if "links" in response and response["links"]:
            span.set_attribute(
                SemanticConvention.GEN_AI_CRAWL_RESULT_LINKS_COUNT,
                len(response["links"]),
            )

        # Screenshot and media information
        if "screenshot" in response and response["screenshot"]:
            span.set_attribute("gen_ai.response.has_screenshot", True)

        # === CRAWL PROGRESS AND BUSINESS METRICS ===
        # Progress tracking (for crawl operations)
        if "completed" in response and response["completed"] is not None:
            span.set_attribute(
                "gen_ai.response.progress_completed", response["completed"]
            )
        if "total" in response and response["total"] is not None:
            span.set_attribute("gen_ai.response.progress_total", response["total"])
            # Calculate completion percentage
            if (
                "completed" in response
                and response["completed"] is not None
                and response["total"] > 0
            ):
                completion_rate = (response["completed"] / response["total"]) * 100
                span.set_attribute("gen_ai.response.completion_rate", completion_rate)

        # Credits and cost tracking (business intelligence)
        if "creditsUsed" in response and response["creditsUsed"] is not None:
            span.set_attribute("gen_ai.response.credits_used", response["creditsUsed"])

        # Job status for async operations
        if "status" in response and response["status"]:
            span.set_attribute(
                SemanticConvention.GEN_AI_MONITOR_TASK_STATUS, response["status"]
            )

        # Job ID and timing
        if "id" in response and response["id"]:
            span.set_attribute("gen_ai.response.job_id", response["id"])
        if "expiresAt" in response and response["expiresAt"]:
            span.set_attribute("gen_ai.response.expires_at", str(response["expiresAt"]))

        # Data array information (for search and crawl results)
        if "data" in response and isinstance(response["data"], list):
            span.set_attribute("gen_ai.response.data_count", len(response["data"]))

            # Extract aggregate metrics from data array
            total_links = 0
            total_content_length = 0
            success_count = 0

            for item in response["data"]:
                if isinstance(item, dict):
                    if item.get("success", True):
                        success_count += 1
                    if "links" in item and isinstance(item["links"], list):
                        total_links += len(item["links"])
                    if "markdown" in item and item["markdown"]:
                        total_content_length += len(item["markdown"])
                    elif "html" in item and item["html"]:
                        total_content_length += len(item["html"])

            if total_links > 0:
                span.set_attribute("gen_ai.response.total_links_count", total_links)
            if total_content_length > 0:
                span.set_attribute(
                    "gen_ai.response.total_content_length", total_content_length
                )
            if len(response["data"]) > 0:
                span.set_attribute(
                    "gen_ai.response.success_rate",
                    success_count / len(response["data"]),
                )

        # Capture content summary if enabled (NOT full content)
        if trace_content:
            _capture_content_summary(span, response, ctx)

    except Exception as e:
        logger.debug("Error processing single response: %s", e)


def _capture_content_summary(
    span, response: dict, ctx: FirecrawlInstrumentationContext
):
    """Capture content summary (not full content) for tracing."""
    try:
        # Input: URL as prompt
        span.set_attribute(SemanticConvention.GEN_AI_CONTENT_PROMPT, ctx.url)

        # Output: Content summary (NOT full content)
        if "markdown" in response and response["markdown"]:
            # Just show first 500 chars of markdown as sample
            formatted_content = format_content(response["markdown"], max_length=500)
            span.set_attribute(
                SemanticConvention.GEN_AI_CONTENT_COMPLETION, formatted_content
            )
        elif "html" in response and response["html"]:
            # Show first 300 chars of HTML as sample
            formatted_content = format_content(response["html"], max_length=300)
            span.set_attribute(
                SemanticConvention.GEN_AI_CONTENT_COMPLETION, formatted_content
            )
        elif "text" in response and response["text"]:
            # Show first 500 chars of text as sample
            formatted_content = format_content(response["text"], max_length=500)
            span.set_attribute(
                SemanticConvention.GEN_AI_CONTENT_COMPLETION, formatted_content
            )
        else:
            # Fallback: show what was scraped
            metadata = response.get("metadata", {})
            title = metadata.get("title", "")
            if title:
                span.set_attribute(
                    SemanticConvention.GEN_AI_CONTENT_COMPLETION, f"Scraped: {title}"
                )
            else:
                span.set_attribute(
                    SemanticConvention.GEN_AI_CONTENT_COMPLETION,
                    "Content scraped successfully",
                )

    except Exception as e:
        logger.debug("Error capturing content summary: %s", e)


def handle_firecrawl_error(span, error):
    """Handle exceptions during Firecrawl operations with detailed error analysis."""
    try:
        # Set error status
        span.set_status(Status(StatusCode.ERROR, str(error)))

        # Set error type and message
        span.set_attribute("error.type", type(error).__name__)
        span.set_attribute("error.message", str(error))

        # Handle specific error types
        if hasattr(error, "response"):
            # HTTP error
            response = error.response
            if hasattr(response, "status_code"):
                span.set_attribute("http.status_code", response.status_code)
            if hasattr(response, "text"):
                span.set_attribute(
                    "error.response_text", response.text[:500]
                )  # Limit error text

        # Additional error categorization using standard patterns
        error_message = str(error).lower()
        if "rate limit" in error_message:
            span.set_attribute("error.category", "rate_limit")
        elif "api key" in error_message or "authentication" in error_message:
            span.set_attribute("error.category", "authentication")
        elif "timeout" in error_message:
            span.set_attribute("error.category", "timeout")
        elif "not found" in error_message:
            span.set_attribute("error.category", "not_found")
        else:
            span.set_attribute("error.category", "unknown")

    except Exception as e:
        logger.debug("Error handling firecrawl error: %s", e)
