# evolving_agents/core/mongodb_client.py
import motor.motor_asyncio
import os
import logging
from evolving_agents import config as eat_config # Assuming config.py loads .env
import pymongo # For pymongo constants

logger = logging.getLogger(__name__)

class MongoDBClient:
    _instance = None
    # _initialized_instance_flag = False # Renamed for clarity

    def __new__(cls, *args, **kwargs):
        if not cls._instance:
            cls._instance = super(MongoDBClient, cls).__new__(cls)
            cls._instance._init_has_run = False # Initialize flag here for __init__
        return cls._instance

    def __init__(self, uri: str = None, db_name: str = None):
        if hasattr(self, '_init_has_run') and self._init_has_run:
            return

        self.uri = uri or os.getenv("MONGODB_URI")
        self.db_name = db_name or eat_config.MONGODB_DATABASE_NAME # Use centralized config

        if not self.uri:
            logger.error("MongoDB URI not provided or found in environment variables (MONGODB_URI).")
            raise ValueError("MongoDB URI not provided or found in environment variables (MONGODB_URI).")
        
        if not self.db_name:
            logger.error("MongoDB Database Name not provided or found (MONGODB_DATABASE_NAME in config).")
            raise ValueError("MongoDB Database Name not provided or found (MONGODB_DATABASE_NAME in config).")

        self.client = None # Initialize to None
        self.db = None     # Initialize to None

        try:
            logger.info(f"Attempting to connect to MongoDB with Motor using URI (credentials hidden) and DB: '{self.db_name}'")
            self.client = motor.motor_asyncio.AsyncIOMotorClient(self.uri)
            # Ping will be done explicitly after initialization in the setup script
            self.db = self.client[self.db_name] # This is now an AsyncIOMotorDatabase
            logger.info(f"Motor client created for MongoDB. Database: '{self.db_name}'")
            self._init_has_run = True
        except pymongo.errors.ConfigurationError as ce:
            logger.error(f"MongoDB configuration error (often URI format, auth, or DNS issue): {ce}")
            raise ValueError(f"MongoDB configuration error. Check your MONGODB_URI and credentials. Details: {ce}") from ce
        except pymongo.errors.ConnectionFailure as cf: # motor.errors.ConnectionFailure might also be relevant
            logger.error(f"Failed to connect to MongoDB server (network issue, server down, or IP whitelist): {cf}")
            raise ConnectionError(f"Failed to connect to MongoDB server. Check server status and network access (e.g., Atlas IP Whitelist). Details: {cf}") from cf
        except Exception as e:
            logger.error(f"An unexpected error occurred while initializing Motor MongoDBClient: {e}", exc_info=True)
            raise Exception(f"Unexpected MongoDB (Motor) client initialization error: {e}") from e

    def get_collection(self, collection_name: str) -> motor.motor_asyncio.AsyncIOMotorCollection:
        # CORRECTED CHECK:
        if self.db is None:
            logger.critical("MongoDBClient.db is None. Cannot get collection. This indicates MongoDBClient __init__ failed or was not completed.")
            raise RuntimeError("MongoDBClient is not properly initialized; database connection attribute 'db' is missing.")
        return self.db[collection_name]

    async def ping_server(self):
        """Async method to explicitly ping the server to check connection."""
        if self.client is None: # Check if client was initialized
            raise ConnectionError("Motor client not initialized. Cannot ping server.")
        try:
            await self.client.admin.command('ping')
            logger.info("MongoDB server ping successful (Motor).")
            return True
        except Exception as e:
            logger.error(f"MongoDB server ping failed (Motor): {e}")
            return False

    def close(self):
        """Close the MongoDB client connection."""
        if self.client:
            self.client.close()
            logger.info("MongoDB client connection closed.")
            self.client = None
            self.db = None
            self._init_has_run = False # Allow re-initialization if needed later