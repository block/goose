#!/usr/bin/env python3

"""
Goose Session Compression Utility

This utility analyzes and compresses Goose chat sessions by identifying similar content
blocks and creating references to reduce token usage.

Features:
- Configurable compression levels
- Message type filtering
- File validation and backup
- Progress tracking
- Detailed statistics

Author: frankg@squareup.com
Version: 0.2
"""

import dataclasses
from dataclasses import dataclass, field
import hashlib
import json
import logging
import os
import shutil
import sys
import time
from typing import List, Dict, Set, Tuple, Optional, Any

# Configure logging
logging.basicConfig(
    level=logging.INFO, format="%(asctime)s - %(levelname)s - %(message)s"
)
logger = logging.getLogger(__name__)

# Compression level presets
COMPRESSION_PRESETS = {
    "light": {
        "threshold": 200, 
        "similarity_threshold": 0.9, 
        "exclude_types": ["toolRequest", "toolResponse"]
    },
    "medium": {
        "threshold": 100, 
        "similarity_threshold": 0.8, 
        "exclude_types": []
    },
    "aggressive": {
        "threshold": 50, 
        "similarity_threshold": 0.7, 
        "exclude_types": []
    }
}

# Configure logging
logging.basicConfig(
    level=logging.INFO, format="%(asctime)s - %(levelname)s - %(message)s"
)
logger = logging.getLogger(__name__)


@dataclass
class CompressionConfig:
    """Configuration for compression settings."""
    threshold: int = 100
    similarity_threshold: float = 0.8
    exclude_types: List[str] = field(default_factory=list)
    backup: bool = True
    validate: bool = True
    show_progress: bool = True

    @classmethod
    def from_preset(cls, preset: str) -> 'CompressionConfig':
        """Create config from a preset name."""
        if preset not in COMPRESSION_PRESETS:
            raise ValueError(f"Invalid preset: {preset}. Valid options: {list(COMPRESSION_PRESETS.keys())}")
        return cls(**COMPRESSION_PRESETS[preset])

@dataclass
class MessageBlock:
    """Represents a block of message content with metadata."""
    content: str
    token_count: int
    hash: str
    references: List[str] = field(default_factory=list)
    message_type: str = "text"

    def __post_init__(self):
        """Validate message block data."""
        if not isinstance(self.content, str):
            raise ValueError("Content must be a string")
        if not isinstance(self.token_count, int):
            raise ValueError("Token count must be an integer")
        if not isinstance(self.hash, str):
            raise ValueError("Hash must be a string")
        if not isinstance(self.references, list):
            raise ValueError("References must be a list")
        if not isinstance(self.message_type, str):
            raise ValueError("Message type must be a string")


@dataclass
class CompressionStats:
    """Statistics about the compression process."""
    original_tokens: int = 0
    compressed_tokens: int = 0
    deduplicated_blocks: int = 0
    references_created: int = 0
    total_messages: int = 0
    processed_messages: int = 0
    skipped_messages: int = 0
    original_size: int = 0
    compressed_size: int = 0
    start_time: float = field(default_factory=time.time)
    end_time: float = 0

    @property
    def reduction_percentage(self) -> float:
        """Calculate the token reduction percentage."""
        if self.original_tokens == 0:
            return 0.0
        return ((self.original_tokens - self.compressed_tokens) / self.original_tokens) * 100

    @property
    def size_reduction_percentage(self) -> float:
        """Calculate the file size reduction percentage."""
        if self.original_size == 0:
            return 0.0
        return ((self.original_size - self.compressed_size) / self.original_size) * 100

    @property
    def elapsed_time(self) -> float:
        """Calculate elapsed time in seconds."""
        if self.end_time == 0:
            return time.time() - self.start_time
        return self.end_time - self.start_time


class SessionCompressor:
    """Handles compression of chat session content."""

    def __init__(self, config: CompressionConfig):
        """Initialize the compressor with configuration."""
        self.config = config
        self.message_blocks: Dict[str, MessageBlock] = {}
        self.content_index: Dict[str, Set[str]] = {}
        self.stats = CompressionStats()
        self.progress_count = 0
        self.progress_total = 0

    def update_progress(self, message: str = "") -> None:
        """Update progress bar if enabled."""
        if not self.config.show_progress:
            return
        
        self.progress_count += 1
        percentage = (self.progress_count / self.progress_total) * 100
        bar_length = 50
        filled = int(bar_length * self.progress_count / self.progress_total)
        bar = "=" * filled + "-" * (bar_length - filled)
        
        sys.stdout.write(f"\r[{bar}] {percentage:.1f}% {message}")
        sys.stdout.flush()

        if self.progress_count == self.progress_total:
            sys.stdout.write("\n")
            sys.stdout.flush()

    def analyze_message(self, message: str, msg_type: str = "text") -> MessageBlock:
        """
        Analyze a message and create a MessageBlock.

        Args:
            message: The message content to analyze
            msg_type: Type of message (text, toolRequest, etc)

        Returns:
            MessageBlock object with content analysis

        Raises:
            ValueError: If message is not a string
        """
        if not isinstance(message, str):
            raise ValueError("Message must be a string")

        content_hash = hashlib.sha256(message.encode()).hexdigest()
        token_count = self.estimate_tokens(message)

        return MessageBlock(
            content=message, 
            token_count=token_count, 
            hash=content_hash, 
            references=[],
            message_type=msg_type
        )

    def estimate_tokens(self, text: str) -> int:
        """
        Estimate token count for a message.

        This uses a more accurate estimation based on GPT tokenization rules:
        - Splits on whitespace and punctuation
        - Counts numbers as separate tokens
        - Handles common markdown syntax

        Args:
            text: Text to estimate tokens for

        Returns:
            Estimated token count
        """
        if not text:
            return 0

        # Handle JSON strings by parsing and counting structure
        if text.startswith('{') and text.endswith('}'):
            try:
                data = json.loads(text)
                # Count structure tokens (brackets, colons, etc)
                structure_tokens = text.count('{') + text.count('}') + \
                                 text.count('[') + text.count(']') + \
                                 text.count(':') + text.count(',')
                # Recursively count string values
                value_tokens = sum(self.estimate_tokens(str(v)) 
                                 for v in data.values() if v)
                return structure_tokens + value_tokens
            except json.JSONDecodeError:
                pass

        # Split on whitespace and punctuation
        import re
        tokens = re.findall(r'\b\w+\b|[^\w\s]', text)
        
        # Count markdown syntax tokens
        markdown_tokens = text.count('```') + text.count('**') + \
                        text.count('*') + text.count('_') + \
                        text.count('#') + text.count('`') + \
                        text.count('[') + text.count(']') + \
                        text.count('(') + text.count(')')

        # Add 30% overhead for subword tokenization
        word_tokens = len(tokens)
        subword_overhead = int(word_tokens * 0.3)

        return word_tokens + subword_overhead + markdown_tokens

    def find_similar_blocks(self, block: MessageBlock) -> List[str]:
        """
        Find similar message blocks based on content similarity.

        Args:
            block: MessageBlock to compare against existing blocks

        Returns:
            List of hashes for similar blocks
        """
        similar = []
        for hash_val, existing in self.message_blocks.items():
            # Skip blocks of different types if configured
            if block.message_type != existing.message_type:
                continue
                
            score = self.similarity_score(block, existing)
            logger.debug(f"Similarity score between blocks: {score}")
            if score > self.config.similarity_threshold:
                similar.append(hash_val)
        return similar

    @staticmethod
    def similarity_score(block1: MessageBlock, block2: MessageBlock) -> float:
        """
        Calculate similarity between two blocks using Jaccard similarity.

        Args:
            block1: First MessageBlock to compare
            block2: Second MessageBlock to compare

        Returns:
            Similarity score between 0 and 1
        """
        words1 = set(block1.content.lower().split())
        words2 = set(block2.content.lower().split())

        intersection = words1.intersection(words2)
        union = words1.union(words2)

        if not union:
            return 0.0

        return len(intersection) / len(union)

    def extract_message_content(self, msg: dict) -> List[Tuple[str, str]]:
        """
        Extract message content from Goose message formats.

        Args:
            msg: Message dictionary

        Returns:
            List of tuples (content, type)
        """
        if not isinstance(msg, dict):
            return []

        # Skip metadata messages but count their tokens
        if "message_count" in msg:
            return [("metadata", json.dumps(msg))]

        contents = []
        
        # Handle Goose message format
        if "content" in msg:
            content = msg["content"]
            if isinstance(content, list):
                for item in content:
                    if isinstance(item, dict):
                        msg_type = item.get("type", "text")
                        if msg_type == "text":
                            contents.append((msg_type, item.get("text", "")))
                        elif msg_type in ["toolRequest", "toolResponse"]:
                            # Include tool calls in token count if not excluded
                            if msg_type not in self.config.exclude_types:
                                contents.append((msg_type, json.dumps(item)))
            elif isinstance(content, str):
                contents.append(("text", content))

        # Handle simple message format
        elif "message" in msg:
            contents.append(("text", msg["message"]))

        return [(t, c) for t, c in contents if c]  # Filter empty strings

    def compress_session(
        self, messages: List[dict]
    ) -> Tuple[List[dict], CompressionStats]:
        """
        Compress a session of messages.

        Args:
            messages: List of message dictionaries to compress

        Returns:
            Tuple of (compressed messages, compression statistics)
        """
        self.stats = CompressionStats(total_messages=len(messages))
        compressed_messages = []
        self.progress_total = len(messages) * 2  # For both analysis and compression phases
        self.progress_count = 0

        logger.info("Phase 1: Analyzing messages...")
        # First pass - analyze all messages
        for msg in messages:
            self.stats.processed_messages += 1
            self.update_progress("Analyzing messages...")

            # Extract all content parts with their types
            content_parts = self.extract_message_content(msg)
            if not content_parts:
                self.stats.skipped_messages += 1
                compressed_messages.append(msg)
                continue

            total_tokens = 0
            for msg_type, content in content_parts:
                block = self.analyze_message(content, msg_type)
                total_tokens += block.token_count
                self.stats.original_tokens += block.token_count
        
        # Alignment
        print()
        
        logger.info("Phase 2: Compressing messages...")
        # Second pass - compress messages
        for msg in messages:
            self.update_progress("Compressing messages...")
            
            # Extract all content parts with their types
            content_parts = self.extract_message_content(msg)
            if not content_parts:
                continue

            compressed_content = []
            for msg_type, content in content_parts:
                block = self.analyze_message(content, msg_type)

                # Skip compression for excluded types
                if msg_type in self.config.exclude_types:
                    if isinstance(msg.get("content", []), list):
                        if msg_type == "text":
                            compressed_content.append({"type": msg_type, "text": content})
                        else:
                            # Keep original JSON for non-text types
                            data = json.loads(content)
                            compressed_content.append(data)
                    else:
                        compressed_content.append(content)
                    self.stats.compressed_tokens += block.token_count
                    continue

                # Check for similar existing blocks
                similar = self.find_similar_blocks(block)
                if similar and block.token_count > self.config.threshold:
                    # Create reference to existing block
                    ref_block = {
                        "type": "reference",
                        "ref": similar[0],
                        "original_tokens": block.token_count,
                        "original_type": msg_type
                    }
                    compressed_content.append(ref_block)
                    self.stats.references_created += 1

                    # Update references in original block
                    self.message_blocks[similar[0]].references.append(block.hash)
                else:
                    # Store new block
                    self.message_blocks[block.hash] = block
                    if isinstance(msg.get("content", []), list):
                        if msg_type == "text":
                            compressed_content.append({"type": msg_type, "text": content})
                        else:
                            # Keep original JSON for non-text types
                            data = json.loads(content)
                            compressed_content.append(data)
                    else:
                        compressed_content.append(content)
                    self.stats.compressed_tokens += block.token_count

            # Update the message with compressed content
            compressed_msg = msg.copy()
            if isinstance(msg.get("content"), list):
                compressed_msg["content"] = compressed_content
            elif isinstance(msg.get("content"), str):
                compressed_msg["content"] = compressed_content[0] if compressed_content else ""
            compressed_messages.append(compressed_msg)

            # Index content for retrieval
            self.index_content(block)

        self.stats.deduplicated_blocks = len(self.message_blocks)
        self.stats.end_time = time.time()
        return compressed_messages, self.stats

    def index_content(self, block: MessageBlock) -> None:
        """
        Index content for quick retrieval.

        Args:
            block: MessageBlock to index
        """
        words = set(block.content.lower().split())
        for word in words:
            if word not in self.content_index:
                self.content_index[word] = set()
            self.content_index[word].add(block.hash)


def load_session_file(filepath: str) -> List[dict]:
    """
    Load and parse a session file.

    Args:
        filepath: Path to session file

    Returns:
        List of message dictionaries

    Raises:
        FileNotFoundError: If file doesn't exist
        json.JSONDecodeError: If file contains invalid JSON
    """
    try:
        messages = []
        with open(filepath, "r", encoding="utf-8") as f:
            for line in f:
                try:
                    msg = json.loads(line.strip())
                    messages.append(msg)
                except json.JSONDecodeError as e:
                    logger.warning(f"Skipping invalid JSON line: {e}")
                    continue

        if not messages:
            raise ValueError("No valid messages found in session file")

        return messages

    except FileNotFoundError:
        logger.error(f"Session file not found: {filepath}")
        raise
    except Exception as e:
        logger.error(f"Error reading session file: {e}")
        raise


def print_stats(
    stats: CompressionStats, blocks: Optional[Dict[str, MessageBlock]] = None
) -> None:
    """
    Print compression statistics.

    Args:
        stats: CompressionStats object
        blocks: Optional dictionary of message blocks for detailed analysis
    """
    print("\nCompression Statistics:")
    print("----------------------")
    print(f"Total Messages: {stats.total_messages}")
    print(f"Processed Messages: {stats.processed_messages}")
    print(f"Skipped Messages: {stats.skipped_messages}")
    print(f"Processing Time: {stats.elapsed_time:.2f}s")
    print()
    print(f"Original Tokens: {stats.original_tokens:,}")
    print(f"Compressed Tokens: {stats.compressed_tokens:,}")
    print(f"Token Reduction: {stats.reduction_percentage:.1f}%")
    print()
    print(f"Original Size: {stats.original_size / 1024:.1f} KB")
    print(f"Compressed Size: {stats.compressed_size / 1024:.1f} KB")
    print(f"Size Reduction: {stats.size_reduction_percentage:.1f}%")
    print()
    print(f"Deduplicated Blocks: {stats.deduplicated_blocks}")
    print(f"References Created: {stats.references_created}")
    
    if blocks:
        # Sort blocks by token count for better analysis
        sorted_blocks = sorted(
            blocks.items(), 
            key=lambda x: x[1].token_count, 
            reverse=True
        )
        
        print("\nTop 10 Largest Blocks:")
        print("--------------------")
        for hash_val, block in sorted_blocks[:10]:
            print(f"\nBlock {hash_val[:8]} ({block.message_type}):")
            content_preview = (
                (block.content[:100] + "...")
                if len(block.content) > 100
                else block.content
            )
            print(f"Content: {content_preview}")
            print(f"Token Count: {block.token_count:,}")
            print(f"Referenced by: {len(block.references)} other messages")
        
        # Group blocks by type
        type_stats = {}
        for block in blocks.values():
            if block.message_type not in type_stats:
                type_stats[block.message_type] = {
                    "count": 0,
                    "tokens": 0,
                    "refs": 0
                }
            type_stats[block.message_type]["count"] += 1
            type_stats[block.message_type]["tokens"] += block.token_count
            type_stats[block.message_type]["refs"] += len(block.references)
        
        print("\nMessage Type Analysis:")
        print("--------------------")
        for msg_type, type_stat in sorted(
            type_stats.items(),
            key=lambda x: x[1]["tokens"],
            reverse=True
        ):
            print(f"\n{msg_type}:")
            print(f"  Count: {type_stat['count']}")
            print(f"  Total Tokens: {type_stat['tokens']:,}")
            print(f"  Total References: {type_stat['refs']}")
            if type_stat['count'] > 0:
                print(f"  Avg Tokens/Block: {type_stat['tokens']/type_stat['count']:.1f}")
                print(f"  Avg References/Block: {type_stat['refs']/type_stat['count']:.1f}")


def validate_compressed_file(filepath: str) -> bool:
    """
    Validate that a compressed session file can be read and parsed.

    Args:
        filepath: Path to compressed file

    Returns:
        True if valid, False otherwise
    """
    try:
        messages = load_session_file(filepath)
        return len(messages) > 0
    except Exception as e:
        logger.error(f"Validation failed: {e}")
        return False


def create_backup(filepath: str) -> str:
    """
    Create a backup of the original file.

    Args:
        filepath: Path to file to backup

    Returns:
        Path to backup file
    """
    backup_path = f"{filepath}.backup"
    shutil.copy2(filepath, backup_path)
    return backup_path


def main() -> None:
    """Main entry point for the compression utility."""
    try:
        if len(sys.argv) < 2:
            print(
                "Usage: goose_compress.py [--stats] [--output OUTPUT_FILE] [--level LEVEL] "
                "[--no-backup] [--no-validate] [--no-progress] <session_file>"
            )
            print("\nOptions:")
            print("  --stats         Show detailed compression statistics")
            print("  --output FILE   Write compressed output to FILE")
            print("  --level LEVEL   Compression level (light, medium, aggressive)")
            print("  --no-backup     Skip creating backup of original file")
            print("  --no-validate   Skip validation of compressed file")
            print("  --no-progress   Hide progress bar")
            print("\nCompression Levels:")
            for level, settings in COMPRESSION_PRESETS.items():
                print(f"  {level:10} - threshold: {settings['threshold']}, "
                      f"similarity: {settings['similarity']}, "
                      f"excludes: {', '.join(settings['exclude_types'])}")
            sys.exit(1)

        # Parse command line arguments
        args = sys.argv[1:]
        show_stats = "--stats" in args
        no_backup = "--no-backup" in args
        no_validate = "--no-validate" in args
        no_progress = "--no-progress" in args
        output_file = None
        level = "medium"

        i = 0
        while i < len(args):
            if args[i] == "--output" and i + 1 < len(args):
                output_file = args[i + 1]
                i += 2
            elif args[i] == "--level" and i + 1 < len(args):
                level = args[i + 1]
                i += 2
            else:
                i += 1

        # Last argument that isn't a flag or flag value is the input file
        session_file = next(
            arg for arg in reversed(args)
            if not arg.startswith("--")
            and arg not in [output_file, level]
        )

        # Create configuration
        config = CompressionConfig.from_preset(level)
        config.backup = not no_backup
        config.validate = not no_validate
        config.show_progress = not no_progress

        logger.info("Starting compression utility...")
        logger.info(f"Reading session file: {session_file}")

        # Create backup if requested
        if config.backup:
            backup_file = create_backup(session_file)
            logger.info(f"Created backup: {backup_file}")

        # Load and compress
        messages = load_session_file(session_file)
        logger.info(f"Found {len(messages)} messages")

        logger.info(f"Compressing with {level} settings...")
        compressor = SessionCompressor(config)
        compressed_msgs, stats = compressor.compress_session(messages)

        # Save compressed output
        if output_file:
            logger.info(f"Writing compressed session to: {output_file}")
            with open(output_file, "w", encoding="utf-8") as f:
                for msg in compressed_msgs:
                    f.write(json.dumps(msg) + "\n")

            # Validate if requested
            if config.validate:
                logger.info("Validating compressed file...")
                if not validate_compressed_file(output_file):
                    if config.backup:
                        logger.error(
                            "Validation failed! Restoring from backup. "
                            f"Original file preserved at: {backup_file}"
                        )
                        shutil.copy2(backup_file, session_file)
                    else:
                        logger.error(
                            "Validation failed! No backup was created. "
                            "The compressed file may be corrupted."
                        )
                    sys.exit(1)
                logger.info("Validation successful!")

            # Update stats with file sizes
            stats.original_size = os.path.getsize(session_file)
            stats.compressed_size = os.path.getsize(output_file)

            # Print file size stats
            print(f"\nFile Size Statistics:")
            print(f"Original: {stats.original_size / 1024:.1f} KB")
            print(f"Compressed: {stats.compressed_size / 1024:.1f} KB")
            print(f"Reduction: {stats.size_reduction_percentage:.1f}%")
        elif not show_stats:
            logger.warning("No output file specified. Use --output to save compressed session.")

        if show_stats:
            print_stats(stats, compressor.message_blocks if show_stats else None)

    except Exception as e:
        logger.error(f"Error during compression: {e}")
        sys.exit(1)


if __name__ == "__main__":
    main()
