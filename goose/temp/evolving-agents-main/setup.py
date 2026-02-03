# setup.py
import os
from setuptools import setup, find_packages

# Optionally read your README for the long description
with open("README.md", "r", encoding="utf-8") as f:
    long_desc = f.read()

setup(
    name="evolving_agents_framework",
    version="0.1.0",
    description="A production-grade framework for creating, managing, and evolving AI agents",
    long_description=long_desc,
    long_description_content_type="text/markdown",
    author="Matias Molinas",
    author_email="matias.molinas@gmail.com",
    url="https://github.com/matiasmolinas/evolving-agents-framework",
    packages=find_packages(exclude=["examples", "tests", "docs"]),
    include_package_data=True,
    python_requires=">=3.11",
    install_requires=[
        "beeai-framework",
        "numpy"
    ],
    classifiers=[
        "Programming Language :: Python :: 3.11",
        "Operating System :: OS Independent",
    ],
)

