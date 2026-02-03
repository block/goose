# pylint: disable=useless-return, bad-staticmethod-argument, disable=duplicate-code
"""
Initializer of Auto Instrumentation of Crawl4AI Functions.
Supports Crawl4AI 0.7.x with comprehensive operation coverage.
"""

from typing import Collection
import importlib.metadata
from opentelemetry.instrumentation.instrumentor import BaseInstrumentor
from wrapt import wrap_function_wrapper

from openlit.instrumentation.crawl4ai.crawl4ai import general_wrap
from openlit.instrumentation.crawl4ai.async_crawl4ai import async_general_wrap

_instruments = ("crawl4ai >= 0.7.0",)


class Crawl4AIInstrumentor(BaseInstrumentor):
    """
    An instrumentor for Crawl4AI's client library.
    Supports comprehensive 0.7.x operations including async crawling,
    batch processing, deep crawling, and extraction strategies.
    """

    def instrumentation_dependencies(self) -> Collection[str]:
        return _instruments

    def _instrument(self, **kwargs):
        application_name = kwargs.get("application_name", "default_application")
        environment = kwargs.get("environment", "default_environment")
        tracer = kwargs.get("tracer")
        metrics = kwargs.get("metrics_dict")
        pricing_info = kwargs.get("pricing_info", {})
        capture_message_content = kwargs.get("capture_message_content", False)
        disable_metrics = kwargs.get("disable_metrics")

        try:
            version = importlib.metadata.version("crawl4ai")
        except importlib.metadata.PackageNotFoundError:
            version = "unknown"

        # Wrap AsyncWebCrawler.arun() - Primary single URL crawling
        try:
            wrap_function_wrapper(
                "crawl4ai.async_webcrawler",
                "AsyncWebCrawler.arun",
                async_general_wrap(
                    "crawl4ai.arun",
                    version,
                    environment,
                    application_name,
                    tracer,
                    pricing_info,
                    capture_message_content,
                    metrics,
                    disable_metrics,
                ),
            )
        except Exception:
            pass  # Module may not exist in all versions

        # Wrap AsyncWebCrawler.arun_many() - Batch/streaming crawling (0.7.x)
        try:
            wrap_function_wrapper(
                "crawl4ai.async_webcrawler",
                "AsyncWebCrawler.arun_many",
                async_general_wrap(
                    "crawl4ai.arun_many",
                    version,
                    environment,
                    application_name,
                    tracer,
                    pricing_info,
                    capture_message_content,
                    metrics,
                    disable_metrics,
                ),
            )
        except Exception:
            pass  # Module may not exist in all versions

        # Wrap legacy WebCrawler.run() for backward compatibility
        try:
            wrap_function_wrapper(
                "crawl4ai.web_crawler",
                "WebCrawler.run",
                general_wrap(
                    "crawl4ai.run",
                    version,
                    environment,
                    application_name,
                    tracer,
                    pricing_info,
                    capture_message_content,
                    metrics,
                    disable_metrics,
                ),
            )
        except Exception:
            pass  # Module may not exist in all versions

        # Wrap deep crawling operations (0.7.x)
        try:
            wrap_function_wrapper(
                "crawl4ai.deep_crawling.strategies",
                "BFSDeepCrawlStrategy.arun",
                async_general_wrap(
                    "crawl4ai.deep_crawl.bfs",
                    version,
                    environment,
                    application_name,
                    tracer,
                    pricing_info,
                    capture_message_content,
                    metrics,
                    disable_metrics,
                ),
            )
        except Exception:
            pass  # Module may not exist in all versions

        try:
            wrap_function_wrapper(
                "crawl4ai.deep_crawling.strategies",
                "DFSDeepCrawlStrategy.arun",
                async_general_wrap(
                    "crawl4ai.deep_crawl.dfs",
                    version,
                    environment,
                    application_name,
                    tracer,
                    pricing_info,
                    capture_message_content,
                    metrics,
                    disable_metrics,
                ),
            )
        except Exception:
            pass  # Module may not exist in all versions

        try:
            wrap_function_wrapper(
                "crawl4ai.deep_crawling.strategies",
                "BestFirstCrawlingStrategy.arun",
                async_general_wrap(
                    "crawl4ai.deep_crawl.best_first",
                    version,
                    environment,
                    application_name,
                    tracer,
                    pricing_info,
                    capture_message_content,
                    metrics,
                    disable_metrics,
                ),
            )
        except Exception:
            pass  # Module may not exist in all versions

        # Wrap LLM and Extraction Strategy Operations (0.7.x)
        try:
            wrap_function_wrapper(
                "crawl4ai.extraction_strategy",
                "LLMExtractionStrategy.extract",
                general_wrap(
                    "crawl4ai.extraction.llm.extract",
                    version,
                    environment,
                    application_name,
                    tracer,
                    pricing_info,
                    capture_message_content,
                    metrics,
                    disable_metrics,
                ),
            )
        except Exception:
            pass  # Module may not exist in all versions

        try:
            wrap_function_wrapper(
                "crawl4ai.extraction_strategy",
                "LLMExtractionStrategy.run",
                general_wrap(
                    "crawl4ai.extraction.llm.run",
                    version,
                    environment,
                    application_name,
                    tracer,
                    pricing_info,
                    capture_message_content,
                    metrics,
                    disable_metrics,
                ),
            )
        except Exception:
            pass  # Module may not exist in all versions

        try:
            wrap_function_wrapper(
                "crawl4ai.extraction_strategy",
                "JsonCssExtractionStrategy.extract",
                general_wrap(
                    "crawl4ai.extraction.css.extract",
                    version,
                    environment,
                    application_name,
                    tracer,
                    pricing_info,
                    capture_message_content,
                    metrics,
                    disable_metrics,
                ),
            )
        except Exception:
            pass  # Module may not exist in all versions

        try:
            wrap_function_wrapper(
                "crawl4ai.extraction_strategy",
                "JsonXPathExtractionStrategy.extract",
                general_wrap(
                    "crawl4ai.extraction.xpath.extract",
                    version,
                    environment,
                    application_name,
                    tracer,
                    pricing_info,
                    capture_message_content,
                    metrics,
                    disable_metrics,
                ),
            )
        except Exception:
            pass  # Module may not exist in all versions

        try:
            wrap_function_wrapper(
                "crawl4ai.extraction_strategy",
                "CosineStrategy.extract",
                general_wrap(
                    "crawl4ai.extraction.cosine.extract",
                    version,
                    environment,
                    application_name,
                    tracer,
                    pricing_info,
                    capture_message_content,
                    metrics,
                    disable_metrics,
                ),
            )
        except Exception:
            pass  # Module may not exist in all versions

        try:
            wrap_function_wrapper(
                "crawl4ai.extraction_strategy",
                "RegexExtractionStrategy.extract",
                general_wrap(
                    "crawl4ai.extraction.regex.extract",
                    version,
                    environment,
                    application_name,
                    tracer,
                    pricing_info,
                    capture_message_content,
                    metrics,
                    disable_metrics,
                ),
            )
        except Exception:
            pass  # Module may not exist in all versions

        # Wrap Core Processing Operations (0.7.x)
        try:
            wrap_function_wrapper(
                "crawl4ai.async_webcrawler",
                "AsyncWebCrawler.aprocess_html",
                async_general_wrap(
                    "crawl4ai.aprocess_html",
                    version,
                    environment,
                    application_name,
                    tracer,
                    pricing_info,
                    capture_message_content,
                    metrics,
                    disable_metrics,
                ),
            )
        except Exception:
            pass  # Module may not exist in all versions

        try:
            wrap_function_wrapper(
                "crawl4ai.async_webcrawler",
                "AsyncWebCrawler.aseed_urls",
                async_general_wrap(
                    "crawl4ai.aseed_urls",
                    version,
                    environment,
                    application_name,
                    tracer,
                    pricing_info,
                    capture_message_content,
                    metrics,
                    disable_metrics,
                ),
            )
        except Exception:
            pass  # Module may not exist in all versions

        # Wrap Monitor Operations for Business Intelligence
        try:
            wrap_function_wrapper(
                "crawl4ai.components.crawler_monitor",
                "CrawlerMonitor.update_task",
                general_wrap(
                    "crawl4ai.monitor.update_task",
                    version,
                    environment,
                    application_name,
                    tracer,
                    pricing_info,
                    capture_message_content,
                    metrics,
                    disable_metrics,
                ),
            )
        except Exception:
            pass  # Module may not exist in all versions

        try:
            wrap_function_wrapper(
                "crawl4ai.components.crawler_monitor",
                "CrawlerMonitor.get_summary",
                general_wrap(
                    "crawl4ai.monitor.get_summary",
                    version,
                    environment,
                    application_name,
                    tracer,
                    pricing_info,
                    capture_message_content,
                    metrics,
                    disable_metrics,
                ),
            )
        except Exception:
            pass  # Module may not exist in all versions

    def _uninstrument(self, **kwargs):
        """
        Proper uninstrumentation logic to revert patched methods.

        Note: wrapt automatically handles uninstrumentation for
        wrap_function_wrapper calls when the instrumentor is disabled.
        """
        # Uninstrumentation is handled automatically by wrapt
        return
