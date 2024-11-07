from pathlib import Path
from typing import Optional, List, Dict
import re

from jinja2 import Environment, FileSystemLoader

from goose.toolkit.base import Toolkit, tool


class Memory(Toolkit):
    """Memory toolkit for storing and retrieving natural
    language memories with categories and tags"""

    def __init__(self, *args: object, **kwargs: dict[str, object]) -> None:
        super().__init__(*args, **kwargs)
        # Setup memory directories
        self.local_memory_dir = Path(".goose/memory")
        self.global_memory_dir = Path.home() / ".config/goose/memory"
        self._ensure_memory_dirs()

    def _get_memories_data(self) -> dict:
        """Get memory data in a format suitable for template rendering"""
        data = {"global": {}, "local": {}, "has_memories": False}

        # Get global memories
        if self.global_memory_dir.exists():
            global_cats = [f.stem for f in self.global_memory_dir.glob("*.txt")]
            for cat in sorted(global_cats):
                memories = self._load_memories(cat, "global")
                if memories:
                    data["global"][cat] = memories
                    data["has_memories"] = True

        # Get local memories
        if self.local_memory_dir.exists():
            local_cats = [f.stem for f in self.local_memory_dir.glob("*.txt")]
            for cat in sorted(local_cats):
                memories = self._load_memories(cat, "local")
                if memories:
                    data["local"][cat] = memories
                    data["has_memories"] = True

        return data

    def system(self) -> str:
        """Get the memory-specific additions to the system prompt"""
        # Get the template directly since we need to render with our own variables
        base_path = Path(__file__).parent / "prompts"
        env = Environment(loader=FileSystemLoader(base_path))
        template = env.get_template("memory.jinja")
        return template.render(memories=self._get_memories_data())

    def _ensure_memory_dirs(self) -> None:
        """Ensure memory directories exist"""
        self.local_memory_dir.parent.mkdir(parents=True, exist_ok=True)
        self.local_memory_dir.mkdir(exist_ok=True)
        self.global_memory_dir.parent.mkdir(parents=True, exist_ok=True)
        self.global_memory_dir.mkdir(exist_ok=True)

    def _get_memory_file(self, category: str, scope: str = "global") -> Path:
        """Get the path to a memory category file"""
        base_dir = self.global_memory_dir if scope == "global" else self.local_memory_dir
        return base_dir / f"{category}.txt"

    def _load_memories(self, category: str, scope: str = "global") -> List[Dict[str, str]]:
        """Load memories from a category file"""
        memory_file = self._get_memory_file(category, scope)
        if not memory_file.exists():
            return []

        memories = []
        content = memory_file.read_text().strip()
        if content:
            for block in content.split("\n\n"):
                if not block.strip():
                    continue
                memory_lines = block.strip().split("\n")
                tags = []
                text = []
                for line in memory_lines:
                    if line.startswith("#"):
                        tags.extend(tag.strip() for tag in line[1:].split())
                    else:
                        text.append(line)
                memories.append({"text": "\n".join(text).strip(), "tags": tags})
        return memories

    def _save_memories(self, memories: List[Dict[str, str]], category: str, scope: str = "global") -> None:
        """Save memories to a category file"""
        memory_file = self._get_memory_file(category, scope)
        content = []
        for memory in memories:
            if memory["tags"]:
                content.append(f"#{' '.join(memory['tags'])}")
            content.append(memory["text"])
            content.append("")  # Empty line between memories
        memory_file.write_text("\n".join(content))

    @tool
    def remember(self, text: str, category: str, tags: Optional[str] = None, scope: str = "global") -> str:
        """Save a memory with optional tags in a specific category

        Args:
            text (str): The memory text to store
            category (str): The category to store the memory under (e.g., development, personal)
            tags (str, optional): Space-separated tags to associate with the memory
            scope (str): Where to store the memory - 'global' or 'local'
        """
        # Clean and validate category name
        category = re.sub(r"[^a-zA-Z0-9_-]", "_", category.lower())

        # Process tags - remove any existing # prefix and store clean tags
        tag_list = []
        if tags:
            tag_list = [tag.strip().lstrip("#") for tag in tags.split() if tag.strip()]

        # Load existing memories
        memories = self._load_memories(category, scope)

        # Add new memory
        memories.append({"text": text, "tags": tag_list})

        # Save updated memories
        self._save_memories(memories, category, scope)

        tag_msg = f" with tags: {', '.join(tag_list)}" if tag_list else ""
        return f"I'll remember that in the {category} category{tag_msg} ({scope} scope)"

    @tool
    def search(self, query: str, category: Optional[str] = None, scope: Optional[str] = None) -> str:
        """Search through memories by text and tags

        Args:
            query (str): Text to search for in memories and tags
            category (str, optional): Specific category to search in
            scope (str, optional): Which scope to search - 'global', 'local', or None (both)
        """
        results = []
        scopes = ["global", "local"] if scope is None else [scope]

        for current_scope in scopes:
            base_dir = self.global_memory_dir if current_scope == "global" else self.local_memory_dir
            if not base_dir.exists():
                continue

            # Get categories to search
            if category:
                categories = [category]
            else:
                categories = [f.stem for f in base_dir.glob("*.txt")]

            # Search in each category
            for cat in categories:
                memories = self._load_memories(cat, current_scope)
                for memory in memories:
                    # Search in text and tags
                    if query.lower() in memory["text"].lower() or any(
                        query.lower() in tag.lower() for tag in memory["tags"]
                    ):
                        tag_str = f" [tags: {', '.join(memory['tags'])}]" if memory["tags"] else ""
                        results.append(f"{current_scope}/{cat}: {memory['text']}{tag_str}")

        if not results:
            return "No matching memories found"

        return "\n\n".join(results)

    @tool
    def list_categories(self, scope: Optional[str] = None) -> str:
        """List all memory categories

        Args:
            scope (str, optional): Which scope to list - 'global', 'local', or None (both)
        """
        categories = []

        if scope in (None, "local") and self.local_memory_dir.exists():
            local_cats = [f.stem for f in self.local_memory_dir.glob("*.txt")]
            if local_cats:
                categories.append("Local categories:")
                categories.extend(f"  - {cat}" for cat in sorted(local_cats))

        if scope in (None, "global") and self.global_memory_dir.exists():
            global_cats = [f.stem for f in self.global_memory_dir.glob("*.txt")]
            if global_cats:
                categories.append("Global categories:")
                categories.extend(f"  - {cat}" for cat in sorted(global_cats))

        if not categories:
            return "No categories found in the specified scope(s)"

        return "\n".join(categories)

    @tool
    def forget_category(self, category: str, scope: str = "global") -> str:
        """Remove an entire category of memories

        Args:
            category (str): The category to remove
            scope (str): Which scope to remove from - 'global' or 'local'
        """
        memory_file = self._get_memory_file(category, scope)
        if not memory_file.exists():
            return f"No {category} category found in {scope} scope"

        memory_file.unlink()
        return f"Successfully removed {category} category from {scope} scope"
