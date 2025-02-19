"""Test cases for the web scrape tool."""

# Prompts that should trigger valid web scrape tool usage
valid_prompts = [
    "Fetch the content from https://example.com",
    "Download the HTML from this webpage",
    "Get JSON data from this API endpoint",
    "Save this image from the web",
    "Scrape text content from this URL",
    "Download this webpage as text",
    "Get the JSON response from this API",
    "Save this binary file from the web",
    "Fetch and cache this webpage",
    "Download this document as text",
]

# Prompts that should not trigger web scrape tool usage based on tool description
invalid_prompts = [
    "Scrape a complex web application with dynamic content",
    "Extract data from a JavaScript-heavy website",
    "Scrape content that requires login",
    "Download content from multiple pages at once",
    "Extract data from a site with anti-scraping measures",
    "Scrape content that requires user interaction",
    "Download content from a protected API",
    "Extract data from pages requiring authentication",
    "Scrape content from multiple URLs simultaneously",
    "Download data from a site requiring cookies",
]