# evolving_agents/core/smart_context.py

from typing import Dict, Any, List, Optional, Tuple, Union
from dataclasses import dataclass, field
from datetime import datetime, timezone
import numpy as np

@dataclass
class EmbeddingPair:
    """Stores a pair of embeddings for content and task-relevance."""
    content_embedding: List[float]  # E_orig - original content embedding
    relevance_embedding: List[float]  # E_raz - task applicability embedding
    
    def compute_similarity(self, query_embedding: List[float], is_content: bool = True) -> float:
        """Compute cosine similarity with the appropriate embedding."""
        embedding = self.content_embedding if is_content else self.relevance_embedding
        return cosine_similarity(query_embedding, embedding)

@dataclass
class ContentChunk:
    """Represents a chunk of content with its embeddings and metadata."""
    chunk_id: str
    content: str  # T_orig - original text
    applicability: str  # T_raz - task applicability description
    embeddings: EmbeddingPair
    metadata: Dict[str, Any] = field(default_factory=dict)
    
    def score_for_task(self, content_query: List[float], task_query: List[float], 
                       task_weight: float = 0.6) -> Tuple[float, float, float]:
        """
        Score this chunk for a given query pair.
        
        Args:
            content_query: Content query embedding (E_orig)
            task_query: Task context query embedding (E_raz)
            task_weight: Weight for task relevance score (0.0-1.0)
            
        Returns:
            Tuple of (final_score, content_score, task_score)
        """
        content_score = self.embeddings.compute_similarity(content_query, is_content=True)
        task_score = self.embeddings.compute_similarity(task_query, is_content=False)
        
        # Combined score with weighting
        final_score = (task_weight * task_score) + ((1 - task_weight) * content_score)
        
        return (final_score, content_score, task_score)

@dataclass
class Message:
    """Represents a message within the context history."""
    sender_id: str
    receiver_id: Optional[str] # Can be None for broadcasts or initial prompts
    content: Any
    timestamp: str = field(default_factory=lambda: datetime.now(timezone.utc).isoformat())
    version: int = 1

@dataclass
class ContextEntry:
    """Holds the value and the embedding associated with a semantic key's description."""
    value: Any
    key_embedding: Optional[List[float]] = None # Embedding of the *description* of the key
    semantic_key: str # The human-readable key itself
    source_key: Optional[str] = None # Optional: The original key from global context if different
    
    # New fields for dual embedding
    applicability_embedding: Optional[List[float]] = None  # E_raz
    applicability_description: Optional[str] = None  # T_raz

@dataclass
class SmartContext:
    """
    Enhanced context payload with dual embedding strategy for task-specific retrieval.
    This version supports both traditional key-value access and content chunk storage
    with dual embeddings for more sophisticated retrieval.
    """
    metadata: Dict[str, Any] = field(default_factory=dict)
    """Info like task_id, source_agent_id, target_agent_id."""

    # Keys are human-readable strings (semantic_key). Values are ContextEntry objects.
    data: Dict[str, ContextEntry] = field(default_factory=dict)
    """Semantically relevant data, including key embeddings."""

    messages: List[Message] = field(default_factory=list)
    """Filtered list of relevant messages."""
    
    # New field for storing content chunks with dual embeddings
    content_chunks: List[ContentChunk] = field(default_factory=list)
    """Content chunks with dual embeddings for task-specific retrieval."""
    
    # Current task context for relevance-based retrieval
    current_task: str = ""
    """Description of the current task being performed."""
    
    # Configurable weights for scoring
    task_weight: float = 0.6
    content_weight: float = 0.4

    # --- Convenience Methods for Accessing Data ---
    def get_value(self, key: str, default: Optional[Any] = None) -> Any:
        """Gets the value associated with a semantic key."""
        entry = self.data.get(key)
        return entry.value if entry else default

    def get_embedding(self, key: str) -> Optional[List[float]]:
        """Gets the key embedding associated with a semantic key."""
        entry = self.data.get(key)
        return entry.key_embedding if entry else None
    
    def set_current_task(self, task_description: str) -> None:
        """Sets the current task context for subsequent retrievals."""
        self.current_task = task_description
    
    def add_content_chunk(self, 
                          chunk_id: str,
                          content: str, 
                          applicability: str,
                          content_embedding: List[float],
                          applicability_embedding: List[float],
                          metadata: Optional[Dict[str, Any]] = None) -> None:
        """
        Add a content chunk with dual embeddings to the context.
        
        Args:
            chunk_id: Unique identifier for the chunk
            content: Original text content (T_orig)
            applicability: Task applicability description (T_raz)
            content_embedding: Content embedding (E_orig)
            applicability_embedding: Applicability embedding (E_raz)
            metadata: Additional metadata for the chunk
        """
        embedding_pair = EmbeddingPair(
            content_embedding=content_embedding,
            relevance_embedding=applicability_embedding
        )
        
        chunk = ContentChunk(
            chunk_id=chunk_id,
            content=content,
            applicability=applicability,
            embeddings=embedding_pair,
            metadata=metadata or {}
        )
        
        self.content_chunks.append(chunk)
    
    def retrieve_relevant_chunks(self, 
                                content_query: str,
                                task_query: Optional[str] = None,
                                embed_fn: Optional[callable] = None,
                                limit: int = 5,
                                threshold: float = 0.0) -> List[Dict[str, Any]]:
        """
        Retrieve the most relevant chunks using the dual embedding strategy.
        
        Args:
            content_query: The content query string
            task_query: The task context query string (falls back to current_task if None)
            embed_fn: Function to convert query strings to embeddings (must be provided)
            limit: Maximum number of results to return
            threshold: Minimum similarity threshold
            
        Returns:
            List of dictionaries containing chunk info and scores
        """
        if not embed_fn:
            raise ValueError("embed_fn must be provided to embed queries")
            
        if not self.content_chunks:
            return []
            
        # Use current task if task_query not provided
        effective_task_query = task_query or self.current_task
        
        # Generate embeddings for queries
        content_query_embedding = embed_fn(content_query)
        task_query_embedding = embed_fn(effective_task_query) if effective_task_query else content_query_embedding
        
        # Score all chunks
        scored_chunks = []
        for chunk in self.content_chunks:
            final_score, content_score, task_score = chunk.score_for_task(
                content_query_embedding, 
                task_query_embedding,
                self.task_weight
            )
            
            if final_score >= threshold:
                scored_chunks.append({
                    "chunk_id": chunk.chunk_id,
                    "content": chunk.content,
                    "applicability": chunk.applicability,
                    "metadata": chunk.metadata,
                    "final_score": final_score,
                    "content_score": content_score,
                    "task_score": task_score
                })
        
        # Sort by final score and limit results
        scored_chunks.sort(key=lambda x: x["final_score"], reverse=True)
        return scored_chunks[:limit]

    def __contains__(self, key: str) -> bool:
        return key in self.data

    def __len__(self) -> int:
        return len(self.data)

    def items(self):
        """Iterate over (key, value) pairs."""
        return ((k, v.value) for k, v in self.data.items())

    def entries(self):
         """Iterate over (key, ContextEntry) pairs."""
         return self.data.items()

# Utility function for computing cosine similarity
def cosine_similarity(vec1: List[float], vec2: List[float]) -> float:
    """Compute cosine similarity between two vectors."""
    if not vec1 or not vec2:
        return 0.0
        
    # Convert to numpy arrays
    a = np.array(vec1)
    b = np.array(vec2)
    
    # Compute cosine similarity
    norm_a = np.linalg.norm(a)
    norm_b = np.linalg.norm(b)
    
    if norm_a == 0 or norm_b == 0:
        return 0.0
        
    return float(np.dot(a, b) / (norm_a * norm_b))