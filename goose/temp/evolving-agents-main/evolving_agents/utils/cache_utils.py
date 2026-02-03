# evolving_agents/utils/cache_utils.py

import argparse
import os
import logging
from evolving_agents.core.llm_service import LLMCache

logging.basicConfig(level=logging.INFO, 
                    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

def clear_cache(args):
    """Clear the LLM cache."""
    cache = LLMCache(cache_dir=args.cache_dir)
    older_than = args.older_than * 86400 if args.older_than else None
    count = cache.clear_cache(older_than)
    
    logger.info(f"Cleared {count} cache entries")

def cache_stats(args):
    """Show cache statistics."""
    from pathlib import Path
    import json
    import time
    
    cache_dir = Path(args.cache_dir)
    completion_dir = cache_dir / "completions"
    embedding_dir = cache_dir / "embeddings"
    
    completion_files = list(completion_dir.glob("*.json")) if completion_dir.exists() else []
    embedding_files = list(embedding_dir.glob("*.json")) if embedding_dir.exists() else []
    
    # Get models and counts
    completion_models = {}
    embedding_models = {}
    now = time.time()
    
    for file in completion_files:
        try:
            model = file.name.split("_")[0]
            completion_models[model] = completion_models.get(model, 0) + 1
            
            # Check age
            with open(file, 'r') as f:
                data = json.load(f)
            age_days = (now - data["timestamp"]) / 86400
            
            if args.verbose:
                logger.info(f"Completion cache: {file.name}, Age: {age_days:.1f} days")
        except:
            pass
    
    for file in embedding_files:
        try:
            model = file.name.split("_")[0]
            embedding_models[model] = embedding_models.get(model, 0) + 1
            
            # Check age
            with open(file, 'r') as f:
                data = json.load(f)
            age_days = (now - data["timestamp"]) / 86400
            
            if args.verbose:
                logger.info(f"Embedding cache: {file.name}, Age: {age_days:.1f} days")
        except:
            pass
    
    # Print stats
    logger.info(f"LLM Cache Statistics for {cache_dir}")
    logger.info(f"Completion cache entries: {len(completion_files)}")
    for model, count in completion_models.items():
        logger.info(f"  - {model}: {count} entries")
    
    logger.info(f"Embedding cache entries: {len(embedding_files)}")
    for model, count in embedding_models.items():
        logger.info(f"  - {model}: {count} entries")
    
    total_size = sum(f.stat().st_size for f in completion_files + embedding_files)
    logger.info(f"Total cache size: {total_size / (1024*1024):.2f} MB")

def main():
    parser = argparse.ArgumentParser(description="LLM Cache Management Utilities")
    parser.add_argument("--cache-dir", type=str, default=".llm_cache", 
                        help="Cache directory")
    
    subparsers = parser.add_subparsers(dest="command", help="Command to run")
    
    # Clear cache command
    clear_parser = subparsers.add_parser("clear", help="Clear the cache")
    clear_parser.add_argument("--older-than", type=int, 
                             help="Clear entries older than this many days")
    
    # Stats command
    stats_parser = subparsers.add_parser("stats", help="Show cache statistics")
    stats_parser.add_argument("--verbose", "-v", action="store_true", 
                             help="Show detailed information")
    
    args = parser.parse_args()
    
    if args.command == "clear":
        clear_cache(args)
    elif args.command == "stats":
        cache_stats(args)
    else:
        parser.print_help()

if __name__ == "__main__":
    main()