"""Setup configuration for goose-notion extension."""

from setuptools import setup, find_packages

setup(
    name="goose-notion",
    version="0.1.0",
    description="Notion integration for Goose",
    author="Kyle Woolstenhulme",
    author_email="kyle@block.xyz",
    packages=find_packages(),
    install_requires=[
        "notion-client>=1.0.0",
        "aiohttp>=3.8.0",
        "python-dateutil>=2.8.2"
    ],
    entry_points={
        "goose.extensions": [
            "notion=goose_notion:NotionExtension"
        ]
    },
    classifiers=[
        "Development Status :: 3 - Alpha",
        "Intended Audience :: Developers",
        "License :: OSI Approved :: MIT License",
        "Programming Language :: Python :: 3.8",
        "Programming Language :: Python :: 3.9",
        "Programming Language :: Python :: 3.10",
    ],
    python_requires=">=3.8",
)