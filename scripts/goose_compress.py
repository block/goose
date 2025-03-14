#!/usr/bin/env python3

"""
Goose Session Compression Utility

This utility analyzes and compresses Goose chat sessions by identifying similar content
blocks and creating references to reduce token usage.

Author: frankg@squareup.com
Version: 0.1 beta
"""

import dataclasses
from dataclasses import dataclass, field
import hashlib
import json
import logging
import sys
from typing import List, Dict, Set, Tuple, Optional

# Configure logging
logging.basicConfig(
    level=logging.INFO, format="%(asctime)s - %(levelname)s - %(message)s"
)
logger = logging.getLogger(__name__)


@dataclass
class MessageBlock:
    """Represents a block of message content with metadata."""

    content: str
    token_count: int
    hash: str
    references: List[str] = field(default_factory=list)

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


@dataclass
class CompressionStats:
    """Statistics about the compression process."""

    original_tokens: int = 0
    compressed_tokens: int = 0
    deduplicated_blocks: int = 0
    references_created: int = 0
    total_messages: int = 0
    processed_messages: int = 0

    @property
    def reduction_percentage(self) -> float:
        """Calculate the token reduction percentage."""
        if self.original_tokens == 0:
            return 0.0
        return (
            (self.original_tokens - self.compressed_tokens) / self.original_tokens
        ) * 100


class SessionCompressor:
    """Handles compression of chat session content."""

    def __init__(self, threshold: int = 100):
        """Initialize the compressor with a similarity threshold."""
        if threshold < 0:
            raise ValueError("Threshold must be non-negative")
        self.threshold = threshold
        self.message_blocks: Dict[str, MessageBlock] = {}
        self.content_index: Dict[str, Set[str]] = {}
        self.stats = CompressionStats()

    def analyze_message(self, message: str) -> MessageBlock:
        """
        Analyze a message and create a MessageBlock.

        Args:
            message: The message content to analyze

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
            content=message, token_count=token_count, hash=content_hash, references=[]
        )

    @staticmethod
    def estimate_tokens(text: str) -> int:
        """
        Estimate token count for a message.

        This is a simplified estimation - in production this should use
        the actual tokenizer used by the LLM.

        Args:
            text: Text to estimate tokens for

        Returns:
            Estimated token count
        """
        if not text:
            return 0
        return len(text.split())

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
            score = self.similarity_score(block, existing)
            logger.debug(f"Similarity score between blocks: {score}")
            if score > 0.8:  # Consider making this threshold configurable
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

    def extract_message_content(self, msg: dict) -> Optional[str]:
        """
        Extract message content from various message formats.

        Args:
            msg: Message dictionary

        Returns:
            Extracted content string or None if no content found
        """
        if not isinstance(msg, dict):
            return None

        # Skip metadata messages
        if "message_count" in msg:
            return None

        # Handle content array format
        if "content" in msg:
            content = msg["content"]
            if isinstance(content, list):
                for item in content:
                    if isinstance(item, dict) and "text" in item:
                        return item["text"]
            elif isinstance(content, str):
                return content

        # Handle simple message format
        elif "message" in msg:
            return msg["message"]

        return None

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

        for msg in messages:
            self.stats.processed_messages += 1

            content = self.extract_message_content(msg)
            if not content:
                continue

            block = self.analyze_message(content)
            self.stats.original_tokens += block.token_count

            # Check for similar existing blocks
            similar = self.find_similar_blocks(block)
            if similar and block.token_count > self.threshold:
                # Create reference to existing block
                ref_msg = {
                    "type": "reference",
                    "ref": similar[0],
                    "original_tokens": block.token_count,
                }
                compressed_messages.append(ref_msg)
                self.stats.references_created += 1

                # Update references in original block
                self.message_blocks[similar[0]].references.append(block.hash)
            else:
                # Store new block
                self.message_blocks[block.hash] = block
                compressed_messages.append(msg)
                self.stats.compressed_tokens += block.token_count

            # Update content index
            self.index_content(block)

        self.stats.deduplicated_blocks = len(self.message_blocks)
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
    print(f"Original Tokens: {stats.original_tokens}")
    print(f"Compressed Tokens: {stats.compressed_tokens}")
    print(f"Reduction: {stats.reduction_percentage:.1f}%")
    print(f"Deduplicated Blocks: {stats.deduplicated_blocks}")
    print(f"References Created: {stats.references_created}")

    if blocks:
        print("\nDetailed Message Analysis:")
        print("-------------------------")
        for hash_val, block in blocks.items():
            print(f"\nBlock {hash_val[:8]}:")
            content_preview = (
                (block.content[:100] + "...")
                if len(block.content) > 100
                else block.content
            )
            print(f"Content: {content_preview}")
            print(f"Token Count: {block.token_count}")
            print(f"Referenced by: {len(block.references)} other messages")


def main() -> None:
    """Main entry point for the compression utility."""
    try:
        if len(sys.argv) < 2:
            print("Usage: goose_compress.py [--stats] <session_file>")
            sys.exit(1)

        show_stats = "--stats" in sys.argv
        session_file = sys.argv[-1]  # Last argument should be the file

        logger.info("Starting compression utility...")
        logger.info(f"Reading session file: {session_file}")

        messages = load_session_file(session_file)
        logger.info(f"Found {len(messages)} messages")

        logger.info("Creating compressor...")
        compressor = SessionCompressor(threshold=5)  # Lower threshold for testing

        logger.info("Compressing session...")
        compressed_msgs, stats = compressor.compress_session(messages)

        print_stats(stats, compressor.message_blocks if show_stats else None)

    except Exception as e:
        logger.error(f"Error during compression: {e}")
        sys.exit(1)


if __name__ == "__main__":
    main()
