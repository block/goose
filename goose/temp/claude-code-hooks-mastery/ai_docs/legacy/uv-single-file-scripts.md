# Running scripts with UV

A Python script is a file intended for standalone execution, e.g., with `python <script>.py`. Using uv to execute scripts ensures that script dependencies are managed without manually managing environments.

## Running a script without dependencies

If your script has no dependencies, you can execute it with `uv run`:

```python
# example.py
print("Hello world")
```

```bash
$ uv run example.py
Hello world
```

Similarly, if your script depends on a module in the standard library, there's nothing more to do.

Arguments may be provided to the script:

```python
# example.py
import sys
print(" ".join(sys.argv[1:]))
```

```bash
$ uv run example.py test
test

$ uv run example.py hello world!
hello world!
```

Additionally, your script can be read directly from stdin.

Note that if you use `uv run` in a _project_, i.e., a directory with a `pyproject.toml`, it will install the current project before running the script. If your script does not depend on the project, use the `--no-project` flag to skip this:

```bash
$ # Note: the `--no-project` flag must be provided _before_ the script name.
$ uv run --no-project example.py
```

## Running a script with dependencies

When your script requires other packages, they must be installed into the environment that the script runs in. Request the dependency using the `--with` option:

```bash
$ uv run --with rich example.py
```

Constraints can be added to the requested dependency if specific versions are needed:

```bash
$ uv run --with 'rich>12,<13' example.py
```

Multiple dependencies can be requested by repeating with `--with` option.

## Creating a Python script

Python recently added a standard format for inline script metadata. It allows for selecting Python versions and defining dependencies. Use `uv init --script` to initialize scripts with the inline metadata:

```bash
$ uv init --script example.py --python 3.12
```

## Declaring script dependencies

The inline metadata format allows the dependencies for a script to be declared in the script itself. Use `uv add --script` to declare the dependencies for the script:

```bash
$ uv add --script example.py 'requests<3' 'rich'
```

This will add a `script` section at the top of the script declaring the dependencies using TOML:

```python
# /// script
# dependencies = [\
#   "requests<3",\
#   "rich",\
# ]
# ///

import requests
from rich.pretty import pprint

resp = requests.get("https://peps.python.org/api/peps.json")
data = resp.json()
pprint([(k, v["title"]) for k, v in data.items()][:10])
```

uv will automatically create an environment with the dependencies necessary to run the script.

## Using a shebang to create an executable file

A shebang can be added to make a script executable without using `uv run`:

```python
#!/usr/bin/env -S uv run --script

print("Hello, world!")
```

Ensure that your script is executable, e.g., with `chmod +x greet`, then run the script.

## Using alternative package indexes

If you wish to use an alternative package index to resolve dependencies, you can provide the index with the `--index` option:

```bash
$ uv add --index "https://example.com/simple" --script example.py 'requests<3' 'rich'
```

## Locking dependencies

uv supports locking dependencies for PEP 723 scripts using the `uv.lock` file format:

```bash
$ uv lock --script example.py
```

Running `uv lock --script` will create a `.lock` file adjacent to the script (e.g., `example.py.lock`).

## Improving reproducibility

In addition to locking dependencies, uv supports an `exclude-newer` field in the `tool.uv` section of inline script metadata to limit uv to only considering distributions released before a specific date:

```python
# /// script
# dependencies = [\
#   "requests",\
# ]
# [tool.uv]
# exclude-newer = "2023-10-16T00:00:00Z"
# ///
```

## Using different Python versions

uv allows arbitrary Python versions to be requested on each script invocation:

```bash
$ # Use a specific Python version
$ uv run --python 3.10 example.py
```

## Using GUI scripts

On Windows `uv` will run your script ending with `.pyw` extension using `pythonw`.