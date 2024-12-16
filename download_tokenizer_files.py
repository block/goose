# /// script
# dependencies = [
#   "huggingface_hub"
# ]
# ///

# Run: `uv run download_tokenizer_files.py`

from huggingface_hub import hf_hub_download
from pathlib import Path

BASE_DIR = Path("tokenizer_files")
BASE_DIR.mkdir(parents=True, exist_ok=True)

for repo_id in [
    "Xenova/gpt-4o",
    "Xenova/claude-tokenizer",
    "Qwen/Qwen2.5-Coder-32B-Instruct",
]:
    download_dir = BASE_DIR / repo_id.replace("/", "--")
    _path = hf_hub_download(repo_id, filename="tokenizer.json", local_dir=download_dir)
    print(f"Downloaded {repo_id} to {_path}")
