"""
Initializer of Auto Instrumentation of Firecrawl Functions.
Supports comprehensive Firecrawl API operations with enhanced business intelligence.
"""

from typing import Collection
import importlib.metadata
from opentelemetry.instrumentation.instrumentor import BaseInstrumentor
from wrapt import wrap_function_wrapper

from openlit.instrumentation.firecrawl.firecrawl import general_wrap
from openlit.instrumentation.firecrawl.async_firecrawl import async_general_wrap

_instruments = ("firecrawl >= 1.6.3",)


class FireCrawlInstrumentor(BaseInstrumentor):
    """
    An instrumentor for Firecrawl's client library.
    Supports comprehensive Firecrawl API operations including scraping, crawling, searching, and mapping.
    """

    def instrumentation_dependencies(self) -> Collection[str]:
        return _instruments

    def _instrument(self, **kwargs):
        """Instrument the firecrawl-py library functions."""
        application_name = kwargs.get("application_name", "default")
        environment = kwargs.get("environment", "default")
        tracer = kwargs.get("tracer")
        pricing_info = kwargs.get("pricing_info")
        trace_content = kwargs.get("trace_content", True)
        disable_metrics = kwargs.get("disable_metrics")
        version = importlib.metadata.version("firecrawl")
        metrics = kwargs.get("metrics")

        # Core synchronous operations - targeting the correct module path
        try:
            wrap_function_wrapper(
                "firecrawl.firecrawl",
                "FirecrawlApp.scrape_url",
                general_wrap(
                    "firecrawl.scrape_url",
                    version,
                    environment,
                    application_name,
                    tracer,
                    pricing_info,
                    trace_content,
                    metrics,
                    disable_metrics,
                ),
            )
        except Exception:
            pass

        try:
            wrap_function_wrapper(
                "firecrawl.firecrawl",
                "FirecrawlApp.crawl_url",
                general_wrap(
                    "firecrawl.crawl_url",
                    version,
                    environment,
                    application_name,
                    tracer,
                    pricing_info,
                    trace_content,
                    metrics,
                    disable_metrics,
                ),
            )
        except Exception:
            pass

        try:
            wrap_function_wrapper(
                "firecrawl.firecrawl",
                "FirecrawlApp.map_url",
                general_wrap(
                    "firecrawl.map_url",
                    version,
                    environment,
                    application_name,
                    tracer,
                    pricing_info,
                    trace_content,
                    metrics,
                    disable_metrics,
                ),
            )
        except Exception:
            pass

        try:
            wrap_function_wrapper(
                "firecrawl.firecrawl",
                "FirecrawlApp.search",
                general_wrap(
                    "firecrawl.search",
                    version,
                    environment,
                    application_name,
                    tracer,
                    pricing_info,
                    trace_content,
                    metrics,
                    disable_metrics,
                ),
            )
        except Exception:
            pass

        try:
            wrap_function_wrapper(
                "firecrawl.firecrawl",
                "FirecrawlApp.extract",
                general_wrap(
                    "firecrawl.extract",
                    version,
                    environment,
                    application_name,
                    tracer,
                    pricing_info,
                    trace_content,
                    metrics,
                    disable_metrics,
                ),
            )
        except Exception:
            pass

        # Batch operations
        try:
            wrap_function_wrapper(
                "firecrawl.firecrawl",
                "FirecrawlApp.batch_scrape_urls",
                general_wrap(
                    "firecrawl.batch_scrape_urls",
                    version,
                    environment,
                    application_name,
                    tracer,
                    pricing_info,
                    trace_content,
                    metrics,
                    disable_metrics,
                ),
            )
        except Exception:
            pass

        # Monitoring operations
        try:
            wrap_function_wrapper(
                "firecrawl.firecrawl",
                "FirecrawlApp.check_crawl_status",
                general_wrap(
                    "firecrawl.crawl_status",
                    version,
                    environment,
                    application_name,
                    tracer,
                    pricing_info,
                    trace_content,
                    metrics,
                    disable_metrics,
                ),
            )
        except Exception:
            pass

        # Async operations - targeting AsyncFirecrawlApp
        try:
            wrap_function_wrapper(
                "firecrawl.firecrawl",
                "AsyncFirecrawlApp.scrape_url",
                async_general_wrap(
                    "firecrawl.async_scrape_url",
                    version,
                    environment,
                    application_name,
                    tracer,
                    pricing_info,
                    trace_content,
                    metrics,
                    disable_metrics,
                ),
            )
        except Exception:
            pass

        try:
            wrap_function_wrapper(
                "firecrawl.firecrawl",
                "AsyncFirecrawlApp.async_crawl_url",
                async_general_wrap(
                    "firecrawl.async_crawl_url",
                    version,
                    environment,
                    application_name,
                    tracer,
                    pricing_info,
                    trace_content,
                    metrics,
                    disable_metrics,
                ),
            )
        except Exception:
            pass

        try:
            wrap_function_wrapper(
                "firecrawl.firecrawl",
                "AsyncFirecrawlApp.async_extract",
                async_general_wrap(
                    "firecrawl.async_extract",
                    version,
                    environment,
                    application_name,
                    tracer,
                    pricing_info,
                    trace_content,
                    metrics,
                    disable_metrics,
                ),
            )
        except Exception:
            pass

        try:
            wrap_function_wrapper(
                "firecrawl.firecrawl",
                "AsyncFirecrawlApp.async_batch_scrape_urls",
                async_general_wrap(
                    "firecrawl.async_batch_scrape_urls",
                    version,
                    environment,
                    application_name,
                    tracer,
                    pricing_info,
                    trace_content,
                    metrics,
                    disable_metrics,
                ),
            )
        except Exception:
            pass

        try:
            wrap_function_wrapper(
                "firecrawl.firecrawl",
                "AsyncFirecrawlApp.search",
                async_general_wrap(
                    "firecrawl.async_search",
                    version,
                    environment,
                    application_name,
                    tracer,
                    pricing_info,
                    trace_content,
                    metrics,
                    disable_metrics,
                ),
            )
        except Exception:
            pass

        try:
            wrap_function_wrapper(
                "firecrawl.firecrawl",
                "AsyncFirecrawlApp.map_url",
                async_general_wrap(
                    "firecrawl.async_map_url",
                    version,
                    environment,
                    application_name,
                    tracer,
                    pricing_info,
                    trace_content,
                    metrics,
                    disable_metrics,
                ),
            )
        except Exception:
            pass

    def _uninstrument(self, **kwargs):
        pass
