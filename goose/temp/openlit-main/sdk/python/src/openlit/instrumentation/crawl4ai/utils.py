"""
Utilities for Crawl4AI instrumentation.
Includes operation mapping, telemetry processing, and context caching for 0.7.x support.
"""

import logging
from typing import Any, Optional, List

from opentelemetry.trace import Status, StatusCode
from opentelemetry.sdk.resources import (
    SERVICE_NAME,
    TELEMETRY_SDK_NAME,
    DEPLOYMENT_ENVIRONMENT,
)

from openlit.__helpers import handle_exception
from openlit.semcov import SemanticConvention

# Initialize logger for logging potential issues and operations
logger = logging.getLogger(__name__)

# Operation mapping for Crawl4AI endpoints following the framework guide pattern
CRAWL_OPERATION_MAP = {
    "crawl4ai.arun": "crawl",
    "crawl4ai.arun_many": "crawl",  # Use same operation name for consistency
    "crawl4ai.run": "crawl",
    "crawl4ai.deep_crawl.bfs": "crawl_deep",
    "crawl4ai.deep_crawl.dfs": "crawl_deep",
    "crawl4ai.deep_crawl.best_first": "crawl_deep",
    "crawl4ai.extract": "extract",
    "crawl4ai.scrape": "scrape",
    # LLM and Extraction Strategy Operations (0.7.x) - All map to "extract"
    "crawl4ai.extraction.llm.extract": "extract",
    "crawl4ai.extraction.llm.run": "extract",
    "crawl4ai.extraction.css.extract": "extract",
    "crawl4ai.extraction.xpath.extract": "extract",
    "crawl4ai.extraction.cosine.extract": "extract",
    "crawl4ai.extraction.regex.extract": "extract",
    "crawl4ai.extraction.lxml.extract": "extract",
    # Processing and Analysis Operations
    "crawl4ai.aprocess_html": "process_html",
    "crawl4ai.aseed_urls": "seed_urls",
    # Monitor and Analysis
    "crawl4ai.monitor.update_task": "monitor_task",
    "crawl4ai.monitor.get_summary": "monitor_summary",
}


class Crawl4AIInstrumentationContext:
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
        "_config",
        "_browser_config",
        "_session_id",
        "_cache_mode",
        "_extraction_strategy",
        "_operation_type",
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
        self._config = None
        self._browser_config = None
        self._session_id = None
        self._cache_mode = None
        self._extraction_strategy = None
        self._operation_type = None

    @property
    def url(self) -> Optional[str]:
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
    def urls(self) -> Optional[List[str]]:
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
    def config(self) -> Optional[Any]:
        """Get CrawlerRunConfig with caching."""
        if self._config is None:
            self._config = self.kwargs.get("config")
        return self._config

    @property
    def browser_config(self) -> Optional[Any]:
        """Get BrowserConfig with caching."""
        if self._browser_config is None:
            if hasattr(self.instance, "config"):
                self._browser_config = self.instance.config
        return self._browser_config

    @property
    def session_id(self) -> Optional[str]:
        """Get session ID with caching."""
        if self._session_id is None:
            config = self.config
            if config and hasattr(config, "session_id"):
                self._session_id = config.session_id
        return self._session_id

    @property
    def cache_mode(self) -> Optional[str]:
        """Get cache mode with caching."""
        if self._cache_mode is None:
            config = self.config
            if config and hasattr(config, "cache_mode"):
                self._cache_mode = str(config.cache_mode)
        return self._cache_mode

    @property
    def extraction_strategy(self) -> Optional[str]:
        """Get extraction strategy with caching."""
        if self._extraction_strategy is None:
            config = self.config
            if config and hasattr(config, "extraction_strategy"):
                strategy = config.extraction_strategy
                if strategy:
                    self._extraction_strategy = strategy.__class__.__name__
        return self._extraction_strategy


def get_operation_name(endpoint: str) -> str:
    """Get operation name from endpoint using the operation map."""
    return CRAWL_OPERATION_MAP.get(endpoint, "crawl")


def create_span_name(operation_name: str, target: str) -> str:
    """Create span name following the '{operation_type} {operation_name}' format."""
    return f"{operation_name} {target}"


def create_crawl_span_name(
    operation_name: str, ctx: Crawl4AIInstrumentationContext, gen_ai_endpoint: str
) -> str:
    """Create descriptive span name for crawl operations, handling batch operations specially."""

    # Check if this is a batch operation
    is_batch = "arun_many" in gen_ai_endpoint or (ctx.urls and len(ctx.urls) > 1)

    if is_batch and ctx.urls:
        # For batch operations, show the URLs but limit length
        if len(ctx.urls) <= 3:
            # Show all URLs for small batches
            urls_str = ", ".join(ctx.urls)
            if len(urls_str) <= 80:  # Keep reasonable length
                return f"{operation_name} [{urls_str}]"

        # For larger batches or long URLs, show count and sample
        sample_urls = ctx.urls[:2]
        remaining_count = len(ctx.urls) - 2
        if remaining_count > 0:
            urls_sample = ", ".join(sample_urls) + f" +{remaining_count} more"
        else:
            urls_sample = ", ".join(sample_urls)

        return f"{operation_name} [{urls_sample}]"

    else:
        # Single URL operation
        target_url = ctx.url if ctx.url != "unknown" else "unknown"
        return f"{operation_name} {target_url}"


def set_crawl_attributes(
    span, ctx: Crawl4AIInstrumentationContext, operation_name: str
):
    """Set comprehensive Crawl4AI span attributes."""

    # Core framework attributes
    span.set_attribute(TELEMETRY_SDK_NAME, "openlit")
    span.set_attribute(
        SemanticConvention.GEN_AI_SYSTEM, SemanticConvention.GEN_AI_SYSTEM_CRAWL4AI
    )

    # Map operation to semantic convention
    if operation_name == "crawl":
        span.set_attribute(
            SemanticConvention.GEN_AI_OPERATION,
            SemanticConvention.GEN_AI_OPERATION_TYPE_CRAWL,
        )
    elif operation_name == "crawl_batch":
        span.set_attribute(
            SemanticConvention.GEN_AI_OPERATION,
            SemanticConvention.GEN_AI_OPERATION_TYPE_CRAWL,  # Fixed: use CRAWL not CRAWL_BATCH
        )
    elif operation_name == "crawl_deep":
        span.set_attribute(
            SemanticConvention.GEN_AI_OPERATION,
            SemanticConvention.GEN_AI_OPERATION_TYPE_CRAWL_DEEP,
        )
    elif operation_name == "extract":
        span.set_attribute(
            SemanticConvention.GEN_AI_OPERATION,
            SemanticConvention.GEN_AI_OPERATION_TYPE_EXTRACT,
        )
    elif operation_name == "scrape":
        span.set_attribute(
            SemanticConvention.GEN_AI_OPERATION,
            SemanticConvention.GEN_AI_OPERATION_TYPE_SCRAPE,
        )
    elif operation_name in ["process_html", "seed_urls"]:
        span.set_attribute(
            SemanticConvention.GEN_AI_OPERATION,
            SemanticConvention.GEN_AI_OPERATION_TYPE_CRAWL,  # Processing operations use crawl type
        )
    else:
        # Default fallback for unknown operations
        span.set_attribute(
            SemanticConvention.GEN_AI_OPERATION,
            SemanticConvention.GEN_AI_OPERATION_TYPE_EXTRACT,  # Changed: use extract instead of agent
        )

    # Environment attributes
    span.set_attribute(SERVICE_NAME, ctx.application_name)
    span.set_attribute(DEPLOYMENT_ENVIRONMENT, ctx.environment)
    span.set_attribute(SemanticConvention.GEN_AI_SDK_VERSION, ctx.version)

    # Agent type for browser-based crawling
    span.set_attribute(
        SemanticConvention.GEN_AI_AGENT_TYPE,
        SemanticConvention.GEN_AI_AGENT_TYPE_BROWSER,
    )

    # URL information
    if operation_name == "crawl_batch" and ctx.urls:
        span.set_attribute(SemanticConvention.GEN_AI_CRAWL_URL_COUNT, len(ctx.urls))
        # Set primary URL as browse URL
        if ctx.urls:
            span.set_attribute(SemanticConvention.GEN_AI_AGENT_BROWSE_URL, ctx.urls[0])
    elif ctx.url and ctx.url != "unknown":
        span.set_attribute(SemanticConvention.GEN_AI_AGENT_BROWSE_URL, ctx.url)

    # Configuration attributes
    if ctx.session_id:
        span.set_attribute(SemanticConvention.GEN_AI_CRAWL_SESSION_ID, ctx.session_id)

    if ctx.cache_mode:
        span.set_attribute(SemanticConvention.GEN_AI_CRAWL_CACHE_MODE, ctx.cache_mode)

    if ctx.extraction_strategy:
        span.set_attribute(
            SemanticConvention.GEN_AI_CRAWL_EXTRACTION_STRATEGY, ctx.extraction_strategy
        )
        span.set_attribute(
            SemanticConvention.GEN_AI_AGENT_STRATEGY, ctx.extraction_strategy
        )

    # Browser configuration
    browser_config = ctx.browser_config
    if browser_config:
        if hasattr(browser_config, "browser_type"):
            span.set_attribute(
                SemanticConvention.GEN_AI_CRAWL_BROWSER_TYPE,
                browser_config.browser_type,
            )

        if hasattr(browser_config, "headless"):
            span.set_attribute(
                SemanticConvention.GEN_AI_CRAWL_HEADLESS, browser_config.headless
            )

        if hasattr(browser_config, "viewport_width"):
            span.set_attribute(
                SemanticConvention.GEN_AI_CRAWL_VIEWPORT_WIDTH,
                browser_config.viewport_width,
            )

        if hasattr(browser_config, "viewport_height"):
            span.set_attribute(
                SemanticConvention.GEN_AI_CRAWL_VIEWPORT_HEIGHT,
                browser_config.viewport_height,
            )

        if hasattr(browser_config, "user_agent"):
            span.set_attribute(
                SemanticConvention.GEN_AI_CRAWL_USER_AGENT,
                str(browser_config.user_agent),
            )

    # Run configuration
    config = ctx.config
    if config:
        if hasattr(config, "word_count_threshold"):
            span.set_attribute(
                SemanticConvention.GEN_AI_CRAWL_WORD_COUNT_THRESHOLD,
                config.word_count_threshold,
            )

        if hasattr(config, "css_selector") and config.css_selector:
            span.set_attribute(
                SemanticConvention.GEN_AI_CRAWL_CSS_SELECTOR, config.css_selector
            )

        if hasattr(config, "excluded_tags") and config.excluded_tags:
            span.set_attribute(
                SemanticConvention.GEN_AI_CRAWL_EXCLUDED_TAGS, str(config.excluded_tags)
            )

        if hasattr(config, "screenshot"):
            span.set_attribute(
                SemanticConvention.GEN_AI_CRAWL_SCREENSHOT, config.screenshot
            )

        if hasattr(config, "pdf"):
            span.set_attribute(SemanticConvention.GEN_AI_CRAWL_PDF, config.pdf)

        if hasattr(config, "wait_for") and config.wait_for:
            span.set_attribute(
                SemanticConvention.GEN_AI_CRAWL_WAIT_FOR, str(config.wait_for)
            )

        if hasattr(config, "page_timeout"):
            span.set_attribute(
                SemanticConvention.GEN_AI_CRAWL_PAGE_TIMEOUT, config.page_timeout
            )

        if hasattr(config, "js_code") and config.js_code:
            js_code_str = (
                str(config.js_code)
                if not isinstance(config.js_code, list)
                else "; ".join(config.js_code)
            )
            span.set_attribute(SemanticConvention.GEN_AI_CRAWL_JS_CODE, js_code_str)

        # Deep crawling attributes
        if hasattr(config, "deep_crawl_strategy") and config.deep_crawl_strategy:
            strategy_name = config.deep_crawl_strategy.__class__.__name__
            span.set_attribute(
                SemanticConvention.GEN_AI_CRAWL_DEEP_STRATEGY, strategy_name
            )

            if hasattr(config.deep_crawl_strategy, "max_depth"):
                span.set_attribute(
                    SemanticConvention.GEN_AI_CRAWL_DEPTH,
                    config.deep_crawl_strategy.max_depth,
                )


def process_crawl_response(
    span, response, ctx: Crawl4AIInstrumentationContext, **kwargs
):
    """Process Crawl4AI response and extract business intelligence."""

    try:
        if response is None:
            return

        # Handle single result vs batch results
        if isinstance(response, list):
            # Batch crawling results
            span.set_attribute(SemanticConvention.GEN_AI_CRAWL_URL_COUNT, len(response))

            # Aggregate statistics from batch
            total_html_length = 0
            total_markdown_length = 0
            total_links = 0
            total_images = 0
            success_count = 0

            for result in response:
                if hasattr(result, "success") and result.success:
                    success_count += 1

                if hasattr(result, "html") and result.html:
                    total_html_length += len(result.html)

                if hasattr(result, "markdown") and result.markdown:
                    total_markdown_length += len(result.markdown)

                if hasattr(result, "links") and result.links:
                    total_links += len(result.links)

                if (
                    hasattr(result, "media")
                    and result.media
                    and hasattr(result.media, "images")
                ):
                    total_images += len(result.media.images)

            # Set aggregate metrics
            span.set_attribute(
                SemanticConvention.GEN_AI_CRAWL_RESULT_SUCCESS,
                success_count == len(response),
            )
            span.set_attribute(
                SemanticConvention.GEN_AI_CRAWL_RESULT_HTML_LENGTH, total_html_length
            )
            span.set_attribute(
                SemanticConvention.GEN_AI_CRAWL_RESULT_MARKDOWN_LENGTH,
                total_markdown_length,
            )
            span.set_attribute(
                SemanticConvention.GEN_AI_CRAWL_RESULT_LINKS_COUNT, total_links
            )
            span.set_attribute(
                SemanticConvention.GEN_AI_CRAWL_RESULT_IMAGES_COUNT, total_images
            )

        else:
            # Single result
            if hasattr(response, "success"):
                span.set_attribute(
                    SemanticConvention.GEN_AI_CRAWL_RESULT_SUCCESS, response.success
                )

            if hasattr(response, "status_code") and response.status_code is not None:
                span.set_attribute(
                    SemanticConvention.GEN_AI_CRAWL_RESULT_STATUS_CODE,
                    response.status_code,
                )

            if hasattr(response, "html") and response.html:
                span.set_attribute(
                    SemanticConvention.GEN_AI_CRAWL_RESULT_HTML_LENGTH,
                    len(response.html),
                )

            if hasattr(response, "markdown") and response.markdown:
                span.set_attribute(
                    SemanticConvention.GEN_AI_CRAWL_RESULT_MARKDOWN_LENGTH,
                    len(response.markdown),
                )

            if hasattr(response, "links") and response.links:
                span.set_attribute(
                    SemanticConvention.GEN_AI_CRAWL_RESULT_LINKS_COUNT,
                    len(response.links),
                )

            if (
                hasattr(response, "media")
                and response.media
                and hasattr(response.media, "images")
            ):
                span.set_attribute(
                    SemanticConvention.GEN_AI_CRAWL_RESULT_IMAGES_COUNT,
                    len(response.media.images),
                )

            if hasattr(response, "redirected_url") and response.redirected_url:
                span.set_attribute(
                    SemanticConvention.GEN_AI_CRAWL_RESULT_REDIRECTED_URL,
                    response.redirected_url,
                )

        # Set success status
        span.set_status(Status(StatusCode.OK))

    except Exception as e:
        handle_exception(span, e)
        logger.error("Error processing crawl response: %s", e)


def format_content(content, max_length: int = 1000) -> str:
    """Format content for tracing with length limits."""
    if content is None:
        return ""

    content_str = str(content)
    if len(content_str) > max_length:
        return content_str[:max_length] + "..."
    return content_str


def capture_message_content_if_enabled(
    span, ctx: Crawl4AIInstrumentationContext, response, capture_message_content: bool
):
    """Capture input/output content if message content capture is enabled."""

    if not capture_message_content:
        return

    try:
        # Capture input URL(s) as prompt
        if ctx.url and ctx.url != "unknown":
            span.set_attribute(SemanticConvention.GEN_AI_CONTENT_PROMPT, ctx.url)
        elif ctx.urls:
            urls_str = ", ".join(ctx.urls[:5])  # Limit to first 5 URLs
            if len(ctx.urls) > 5:
                urls_str += f" and {len(ctx.urls) - 5} more"
            span.set_attribute(SemanticConvention.GEN_AI_CONTENT_PROMPT, urls_str)

        # Capture output content based on response type
        if response is None:
            return

        if isinstance(response, list):
            # For batch responses, capture summary
            completion_summary = f"Crawled {len(response)} URLs"
            span.set_attribute(
                SemanticConvention.GEN_AI_CONTENT_COMPLETION, completion_summary
            )
        else:
            # For single responses, capture markdown content (limited)
            if hasattr(response, "markdown") and response.markdown:
                formatted_content = format_content(response.markdown, max_length=2000)
                span.set_attribute(
                    SemanticConvention.GEN_AI_CONTENT_COMPLETION, formatted_content
                )
            elif hasattr(response, "html") and response.html:
                formatted_content = format_content(response.html, max_length=1000)
                span.set_attribute(
                    SemanticConvention.GEN_AI_CONTENT_COMPLETION, formatted_content
                )

    except Exception as e:
        logger.error("Error capturing message content: %s", e)


def process_llm_extraction_response(
    span, extraction_strategy, response, ctx: Crawl4AIInstrumentationContext
):
    """Process LLM extraction strategy response and capture comprehensive metrics."""

    try:
        # Capture LLM extraction strategy details
        if hasattr(extraction_strategy, "__class__"):
            strategy_name = extraction_strategy.__class__.__name__

            # Map strategy class to semantic convention
            if "LLM" in strategy_name:
                span.set_attribute(
                    SemanticConvention.GEN_AI_EXTRACTION_STRATEGY_TYPE,
                    SemanticConvention.GEN_AI_EXTRACTION_STRATEGY_LLM,
                )
            elif "Css" in strategy_name:
                span.set_attribute(
                    SemanticConvention.GEN_AI_EXTRACTION_STRATEGY_TYPE,
                    SemanticConvention.GEN_AI_EXTRACTION_STRATEGY_CSS,
                )
            elif "XPath" in strategy_name:
                span.set_attribute(
                    SemanticConvention.GEN_AI_EXTRACTION_STRATEGY_TYPE,
                    SemanticConvention.GEN_AI_EXTRACTION_STRATEGY_XPATH,
                )
            elif "Cosine" in strategy_name:
                span.set_attribute(
                    SemanticConvention.GEN_AI_EXTRACTION_STRATEGY_TYPE,
                    SemanticConvention.GEN_AI_EXTRACTION_STRATEGY_COSINE,
                )
            elif "Regex" in strategy_name:
                span.set_attribute(
                    SemanticConvention.GEN_AI_EXTRACTION_STRATEGY_TYPE,
                    SemanticConvention.GEN_AI_EXTRACTION_STRATEGY_REGEX,
                )

        # Process LLM-specific attributes for LLMExtractionStrategy
        if hasattr(extraction_strategy, "llm_config"):
            llm_config = extraction_strategy.llm_config
            if llm_config:
                # Provider information
                if hasattr(llm_config, "provider"):
                    provider_parts = str(llm_config.provider).split("/")
                    if len(provider_parts) >= 2:
                        span.set_attribute(
                            SemanticConvention.GEN_AI_LLM_PROVIDER, provider_parts[0]
                        )
                        span.set_attribute(
                            SemanticConvention.GEN_AI_LLM_MODEL, provider_parts[1]
                        )
                    else:
                        span.set_attribute(
                            SemanticConvention.GEN_AI_LLM_PROVIDER,
                            str(llm_config.provider),
                        )

                # Base URL if available
                if hasattr(llm_config, "base_url") and llm_config.base_url:
                    span.set_attribute(
                        SemanticConvention.GEN_AI_LLM_BASE_URL, llm_config.base_url
                    )

        # Extraction configuration
        if hasattr(extraction_strategy, "extraction_type"):
            span.set_attribute(
                SemanticConvention.GEN_AI_EXTRACTION_TYPE,
                extraction_strategy.extraction_type,
            )

        if (
            hasattr(extraction_strategy, "instruction")
            and extraction_strategy.instruction
        ):
            span.set_attribute(
                SemanticConvention.GEN_AI_EXTRACTION_INSTRUCTION,
                format_content(extraction_strategy.instruction, max_length=500),
            )

        if hasattr(extraction_strategy, "input_format"):
            span.set_attribute(
                SemanticConvention.GEN_AI_EXTRACTION_INPUT_FORMAT,
                extraction_strategy.input_format,
            )

        # Chunking configuration
        if hasattr(extraction_strategy, "apply_chunking"):
            span.set_attribute(
                SemanticConvention.GEN_AI_EXTRACTION_APPLY_CHUNKING,
                extraction_strategy.apply_chunking,
            )

        if hasattr(extraction_strategy, "chunk_token_threshold"):
            span.set_attribute(
                SemanticConvention.GEN_AI_EXTRACTION_CHUNK_TOKEN_THRESHOLD,
                extraction_strategy.chunk_token_threshold,
            )

        if hasattr(extraction_strategy, "overlap_rate"):
            span.set_attribute(
                SemanticConvention.GEN_AI_EXTRACTION_OVERLAP_RATE,
                extraction_strategy.overlap_rate,
            )

        # LLM model parameters
        if (
            hasattr(extraction_strategy, "extra_args")
            and extraction_strategy.extra_args
        ):
            extra_args = extraction_strategy.extra_args
            if "temperature" in extra_args:
                span.set_attribute(
                    SemanticConvention.GEN_AI_LLM_TEMPERATURE, extra_args["temperature"]
                )
            if "max_tokens" in extra_args:
                span.set_attribute(
                    SemanticConvention.GEN_AI_LLM_MAX_TOKENS, extra_args["max_tokens"]
                )
            if "top_p" in extra_args:
                span.set_attribute(
                    SemanticConvention.GEN_AI_LLM_TOP_P, extra_args["top_p"]
                )

        # Token usage tracking (critical for business intelligence)
        if (
            hasattr(extraction_strategy, "total_usage")
            and extraction_strategy.total_usage
        ):
            total_usage = extraction_strategy.total_usage
            if hasattr(total_usage, "prompt_tokens"):
                span.set_attribute(
                    SemanticConvention.GEN_AI_TOKEN_USAGE_INPUT,
                    total_usage.prompt_tokens,
                )
            if hasattr(total_usage, "completion_tokens"):
                span.set_attribute(
                    SemanticConvention.GEN_AI_TOKEN_USAGE_OUTPUT,
                    total_usage.completion_tokens,
                )
            if hasattr(total_usage, "total_tokens"):
                span.set_attribute(
                    SemanticConvention.GEN_AI_TOKEN_USAGE_TOTAL,
                    total_usage.total_tokens,
                )

        # Individual chunk usage
        if hasattr(extraction_strategy, "usages") and extraction_strategy.usages:
            chunk_count = len(extraction_strategy.usages)
            span.set_attribute(SemanticConvention.GEN_AI_TOKEN_CHUNK_COUNT, chunk_count)

            # Aggregate chunk usage information
            total_chunk_tokens = 0
            for usage in extraction_strategy.usages:
                if hasattr(usage, "total_tokens"):
                    total_chunk_tokens += usage.total_tokens
            if total_chunk_tokens > 0:
                span.set_attribute(
                    SemanticConvention.GEN_AI_TOKEN_CHUNK_USAGE, total_chunk_tokens
                )

        # Extraction success/failure
        if response is not None:
            span.set_attribute(SemanticConvention.GEN_AI_EXTRACTION_SUCCESS, True)

            # Count extracted items for business intelligence
            if isinstance(response, list):
                span.set_attribute("gen_ai.extraction.items_count", len(response))
            elif isinstance(response, dict):
                span.set_attribute("gen_ai.extraction.items_count", 1)
        else:
            span.set_attribute(SemanticConvention.GEN_AI_EXTRACTION_SUCCESS, False)

    except Exception as e:
        handle_exception(span, e)
        logger.error("Error processing LLM extraction response: %s", e)


def process_crawler_monitor_metrics(span, monitor_data):
    """Process CrawlerMonitor metrics and integrate with OpenLIT telemetry."""

    try:
        if not monitor_data:
            return

        # Task-level metrics
        if "task_id" in monitor_data:
            span.set_attribute(
                SemanticConvention.GEN_AI_MONITOR_TASK_ID, monitor_data["task_id"]
            )

        if "status" in monitor_data:
            span.set_attribute(
                SemanticConvention.GEN_AI_MONITOR_TASK_STATUS, monitor_data["status"]
            )

        # Memory metrics (business intelligence)
        if "memory_usage" in monitor_data:
            span.set_attribute(
                SemanticConvention.GEN_AI_MONITOR_MEMORY_USAGE,
                monitor_data["memory_usage"],
            )

        if "peak_memory" in monitor_data:
            span.set_attribute(
                SemanticConvention.GEN_AI_MONITOR_PEAK_MEMORY,
                monitor_data["peak_memory"],
            )

        # Performance metrics
        if "retry_count" in monitor_data:
            span.set_attribute(
                SemanticConvention.GEN_AI_MONITOR_RETRY_COUNT,
                monitor_data["retry_count"],
            )

        if "wait_time" in monitor_data:
            span.set_attribute(
                SemanticConvention.GEN_AI_MONITOR_WAIT_TIME, monitor_data["wait_time"]
            )

        # Queue and completion metrics
        if "queue_size" in monitor_data:
            span.set_attribute(
                SemanticConvention.GEN_AI_MONITOR_QUEUE_SIZE, monitor_data["queue_size"]
            )

        if "completion_rate" in monitor_data:
            span.set_attribute(
                SemanticConvention.GEN_AI_MONITOR_COMPLETION_RATE,
                monitor_data["completion_rate"],
            )

    except Exception as e:
        logger.error("Error processing crawler monitor metrics: %s", e)


def create_extraction_span_name(
    operation_type: str, strategy_type: str, target: str
) -> str:
    """Create span name for extraction operations following the 'extract {target}' format."""
    # Always use "extract" as the operation name regardless of strategy type
    return f"extract {target}"


def capture_extraction_content(
    span, extraction_input, extraction_output, capture_message_content: bool
):
    """Capture extraction input/output content if enabled."""

    if not capture_message_content:
        return

    try:
        # Capture extraction input as prompt (URL or content snippet)
        if extraction_input:
            if isinstance(extraction_input, str):
                input_content = format_content(extraction_input, max_length=1000)
                span.set_attribute(
                    SemanticConvention.GEN_AI_CONTENT_PROMPT, input_content
                )
            elif hasattr(extraction_input, "__iter__"):
                # Handle list/tuple of inputs
                combined_input = " ".join(str(item)[:200] for item in extraction_input)
                input_content = format_content(combined_input, max_length=1000)
                span.set_attribute(
                    SemanticConvention.GEN_AI_CONTENT_PROMPT, input_content
                )

        # Capture extraction output as completion
        if extraction_output is not None:
            if isinstance(extraction_output, (list, dict)):
                # Format structured output
                output_summary = f"Extracted {len(extraction_output) if hasattr(extraction_output, '__len__') else 1} items"
                span.set_attribute(
                    SemanticConvention.GEN_AI_CONTENT_COMPLETION, output_summary
                )
            else:
                # Handle string output
                output_content = format_content(str(extraction_output), max_length=1000)
                span.set_attribute(
                    SemanticConvention.GEN_AI_CONTENT_COMPLETION, output_content
                )

    except Exception as e:
        logger.error("Error capturing extraction content: %s", e)
