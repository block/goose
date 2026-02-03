# Watchflow Documentation

This directory contains the documentation for Watchflow, built with [MkDocs](https://www.mkdocs.org/) and the [Material theme](https://squidfunk.github.io/mkdocs-material/). Docs are **maintainer-first** and tech-forward: speak to engineers, not marketing; anchor Watchflow as the immune system for the repo, not another AI tool.

## Local Development

### Prerequisites

- Python 3.12+
- uv (recommended) or pip

### Setup

1. **Install dependencies**
   ```bash
   # Install all dependencies including docs (recommended)
   uv sync

   # Or install just the docs dependencies
   uv add mkdocs mkdocs-material mkdocs-git-revision-date-localized-plugin mkdocs-minify-plugin pymdown-extensions

   # Alternative: using pip (if uv is not available)
   pip install -e ".[docs]"
   ```

2. **Start the development server**
   ```bash
   mkdocs serve
   ```

3. **Open your browser**
   - Navigate to http://127.0.0.1:8000
   - The site will automatically reload when you make changes

### Building

```bash
# Build the site
mkdocs build

# Build and serve
mkdocs serve
```

## Writing Guidelines

### Markdown Format

- Use standard Markdown syntax
- Leverage Material theme features like admonitions, tabs, and code blocks
- Include code examples with proper syntax highlighting

### Content Guidelines

1. **Be clear and concise**
   - Use simple, direct language
   - Break complex topics into digestible sections
   - Include practical examples

2. **Stay consistent**
   - Follow the established tone and style
   - Use consistent terminology (see glossary.md)
   - Maintain consistent formatting

3. **Include examples**
   - Provide real-world use cases
   - Include code snippets and configuration examples
   - Show before/after scenarios

4. **Cross-reference**
   - Link to related sections
   - Reference the glossary for technical terms
   - Include "Next Steps" sections

### Admonitions

Use admonitions to highlight important information:

```markdown
!!! note "Note"
    This is a note with important information.

!!! warning "Warning"
    This is a warning about potential issues.

!!! tip "Tip"
    This is a helpful tip for users.

!!! example "Example"
    This is an example of how to do something.
```

### Code Blocks

Use syntax highlighting for code blocks:

```markdown
```python
def example_function():
    return "Hello, World!"
```

```yaml
# Configuration example
service:
  type: NodePort
  port: 8000
```
```

## Contributing

### Adding New Pages

1. **Create the markdown file** in the appropriate directory
2. **Update navigation** in `mkdocs.yml`
3. **Add cross-references** to related pages
4. **Test locally** with `mkdocs serve`

### Updating Existing Pages

1. **Make your changes** in the markdown file
2. **Test locally** to ensure formatting is correct
3. **Update any cross-references** if needed
4. **Commit with a descriptive message**

### Pull Request Process

1. **Fork the repository**
2. **Create a feature branch**
3. **Make your changes**
4. **Test locally**
5. **Submit a pull request**

## Deployment

The documentation is automatically deployed to GitHub Pages when changes are pushed to the `main` branch. The deployment is handled by the [GitHub Actions workflow](../.github/workflows/docs.yml).

### Manual Deployment

```bash
# Build the site
mkdocs build

# Deploy to GitHub Pages (if configured)
mkdocs gh-deploy
```

## Configuration

The documentation is configured via `mkdocs.yml` in the root directory. Key configuration options:

- **Theme**: Material theme with custom styling
- **Navigation**: Hierarchical structure with sections
- **Plugins**: Search, git revision dates, minification
- **Extensions**: Various Markdown extensions for enhanced features

## Troubleshooting

### Common Issues

**Site not building**
- Check for syntax errors in markdown files
- Verify all referenced files exist
- Check the mkdocs.yml configuration

**Styling issues**
- Ensure Material theme is properly installed
- Check for custom CSS conflicts
- Verify theme configuration in mkdocs.yml

**Navigation problems**
- Check the nav structure in mkdocs.yml
- Ensure all referenced files exist
- Verify file paths are correct

### Getting Help

- 📚 [MkDocs Documentation](https://www.mkdocs.org/)
- 🎨 [Material Theme Documentation](https://squidfunk.github.io/mkdocs-material/)
- 🐛 [GitHub Issues](https://github.com/watchflow/watchflow/issues)
- 💬 [GitHub Discussions](https://github.com/watchflow/watchflow/discussions)

---

*For questions about the documentation, please open an issue or start a discussion on GitHub.*
