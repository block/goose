# Goose Notion Extension

A Notion integration for Goose that provides seamless interaction with Notion databases and pages.

## Installation

```bash
goose extension install goose-notion
```

## Configuration

Add your Notion integration token to your Goose configuration:

```yaml
notion:
  token: "your-notion-integration-token"
```

To get your integration token:
1. Go to https://www.notion.so/my-integrations
2. Create a new integration
3. Copy the integration token

## Usage

### Query a Database

```python
# Simple query
results = await notion.query_database(
    database_id="your-database-id"
)

# With filters
results = await notion.query_database(
    database_id="your-database-id",
    filter={
        "property": "Status",
        "select": {
            "equals": "Active"
        }
    }
)
```

### Create a Page

```python
page = await notion.create_page(
    parent_id="your-database-id",
    properties={
        "Name": {"title": [{"text": {"content": "New Page"}}]},
        "Status": {"select": {"name": "Active"}}
    }
)
```

## Development

### Setup

1. Clone the repository:
```bash
git clone https://github.com/block/goose-notion
cd goose-notion
```

2. Create a virtual environment:
```bash
python -m venv venv
source venv/bin/activate  # or `venv\Scripts\activate` on Windows
```

3. Install development dependencies:
```bash
pip install -e ".[dev]"
```

### Testing

Run the tests:
```bash
pytest
```

### Code Style

This project uses:
- Black for code formatting
- isort for import sorting
- pylint for linting

Format code:
```bash
black goose_notion
isort goose_notion
```

## Contributing

Please read [CONTRIBUTING.md](CONTRIBUTING.md) for details on our code of conduct and the process for submitting pull requests.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.