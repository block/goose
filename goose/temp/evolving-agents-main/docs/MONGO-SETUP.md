# MongoDB Setup Guide for EAT: Atlas CLI Local Deployment (Dev/Test)

This guide provides instructions for setting up MongoDB as the unified backend for the Evolving Agents Toolkit (EAT), specifically using the **MongoDB Atlas CLI Local Deployment method.** This approach is **highly recommended for local development and testing** as it provides an Atlas-like environment on your machine, including full support for Atlas Vector Search features, without the 3-search-index limit of the M0 cloud free tier.

**Contents:**

1.  [Why Atlas CLI Local Deployment for Dev/Test?](#1-why-atlas-cli-local-deployment-for-devtest)
2.  [Prerequisites](#2-prerequisites)
3.  [Setup Instructions](#3-setup-instructions)
    *   [Step 3.1: Install the Atlas CLI](#step-31-install-the-atlas-cli)
    *   [Step 3.2: Set Up Your Local Atlas Deployment (or Perform a Fresh Install)](#step-32-set-up-your-local-atlas-deployment-or-perform-a-fresh-install)
    *   [Step 3.3: Create Vector Search Indexes (CRITICAL)](#step-33-create-vector-search-indexes-critical)
        *   [Option 3.3.A: Using `mongosh` (Command Line)](#option-33a-using-mongosh-command-line)
        *   [Option 3.3.B: Using MongoDB Compass (GUI)](#option-33b-using-mongodb-compass-gui)
4.  [Configure EAT Project (`.env` file)](#4-configure-eat-project-env-file)
5.  [Running the EAT Application](#5-running-the-eat-application)
6.  [Managing Your Local Atlas Deployment](#6-managing-your-local-atlas-deployment)
7.  [Troubleshooting](#7-troubleshooting)
8.  [Note on Production](#8-note-on-production)

---

## 1. Why Atlas CLI Local Deployment for Dev/Test?

*   **Full Atlas Vector Search Features Locally:** Test all EAT semantic search capabilities.
*   **No Cloud Index Limits During Development:** Create all necessary vector search indexes.
*   **Consistent Environment:** Mirrors MongoDB Atlas cloud deployments.
*   **Local Resources & Control:** Runs on your machine using Docker (managed by Atlas CLI).
*   **Simplified Vector Search Setup:** Easier than traditional self-hosted MongoDB.

**Important:** This local deployment is for **development and testing only.**

---

## 2. Prerequisites

*   **Docker:** Docker Desktop or Docker Engine must be installed and running.
*   **Python 3.11+:** For the EAT framework.
*   **EAT Project Code:** Cloned locally.
*   **OpenAI API Key (or other LLM provider configured):** For EAT's embedding generation.
*   **(Optional, for GUI Index Creation) MongoDB Compass:** Download from [MongoDB Compass Download](https://www.mongodb.com/try/download/compass).

---

## 3. Setup Instructions

### Step 3.1: Install the Atlas CLI

Follow official instructions: [Install the Atlas CLI](https://www.mongodb.com/docs/atlas/cli/stable/install-atlas-cli/)
Verify: `atlas --version`

### Step 3.2: Set Up Your Local Atlas Deployment (or Perform a Fresh Install)

1.  **Login to Atlas (Recommended):**
    ```bash
    atlas auth login
    ```

2.  **Create/Start the Local Deployment:**
    *   **If setting up for the first time OR want a fresh install:**
        Ensure any existing deployment with the same name is deleted first for a truly fresh start:
        ```bash
        # (Optional) Delete existing if you want a completely fresh state
        atlas deployments delete eat-local-dev --force 
        # (Optional) Prune Docker resources if you suspect issues with old volumes/networks
        # docker system prune -a --volumes # Warning: Removes ALL unused Docker data.
        ```
        Then, create the new deployment:
        ```bash
        atlas deployments setup eat-local-dev --type local --port 27017 --mdbVersion 7.0
        ```
    *   **If resuming an existing PAUSED/IDLE deployment:**
        Sometimes `atlas deployments start eat-local-dev` might show an "unexpected state" error if it's `IDLE`. Try `resume` first:
        ```bash
        atlas deployments resume eat-local-dev
        ```
        If `resume` doesn't work or the deployment is `STOPPED`, try `start`:
        ```bash
        atlas deployments start eat-local-dev
        ```
        If neither works and it remains `IDLE` or errors, it's best to perform a **fresh install** as described above (delete then setup).

    *   **Parameters:**
        *   `eat-local-dev`: Your chosen deployment name.
        *   `--port 27017`: Standard MongoDB port. Change if needed.
        *   `--mdbVersion 7.0`: Recommended. Check `atlas deployments availableversions --type local` for options.

    *   **Verify:** After setup/start, check its status:
        ```bash
        atlas deployments list
        ```
        The `STATE` for `eat-local-dev` should be `RUNNING` or `AVAILABLE`. The connection string will be `mongodb://localhost:27017/`.

### Step 3.3: Create Vector Search Indexes (CRITICAL)

Choose **one** of the following methods (mongosh or Compass).

#### Option 3.3.A: Using `mongosh` (Command Line)

1.  **Connect to your Local Atlas Deployment with `mongosh`:**
    ```bash
    atlas deployments connect eat-local-dev --connectWith mongosh
    ```
    (Replace `eat-local-dev` if you used a different name).

2.  **Switch to the EAT Database in `mongosh`:**
    This database name must match your `MONGODB_DATABASE_NAME` in the `.env` file.
    ```javascript
    use evolving_agents_db; // Or your chosen name
    ```

3.  **Create the Four Vector Search Indexes:**
    **IMPORTANT:** Replace `YOUR_EMBEDDING_DIMENSION` with your model's dimension (e.g., **1536** for `text-embedding-3-small`).

    *   **Index 1: `eat_components` - Content Embedding** (`idx_components_content_embedding`)
        ```javascript
        db.eat_components.createSearchIndex({
          "name": "idx_components_content_embedding",
          "definition": { "mappings": { "dynamic": false, "fields": {
            "content_embedding": { "type": "vector", "dimensions": 1536, "similarity": "cosine" },
            "record_type": { "type": "string", "analyzer": "keyword" }, "domain": { "type": "string", "analyzer": "keyword" },
            "status": { "type": "string", "analyzer": "keyword" }, "tags": { "type": "string", "analyzer": "keyword", "multi": true }
          }}}
        });
        ```

    *   **Index 2: `eat_components` - Applicability Embedding** (`applicability_embedding`)
        ```javascript
        db.eat_components.createSearchIndex({
          "name": "applicability_embedding",
          "definition": { "mappings": { "dynamic": false, "fields": {
            "applicability_embedding": { "type": "vector", "dimensions": YOUR_EMBEDDING_DIMENSION, "similarity": "cosine" },
            "record_type": { "type": "string", "analyzer": "keyword" }, "domain": { "type": "string", "analyzer": "keyword" },
            "status": { "type": "string", "analyzer": "keyword" }, "tags": { "type": "string", "analyzer": "keyword", "multi": true }
          }}}
        });
        ```

    *   **Index 3: `eat_agent_registry` - Agent Description Embedding** (`vector_index_agent_description`)
        ```javascript
        db.eat_agent_registry.createSearchIndex({
          "name": "vector_index_agent_description",
          "definition": { "mappings": { "dynamic": false, "fields": {
            "description_embedding": { "type": "vector", "dimensions": YOUR_EMBEDDING_DIMENSION, "similarity": "cosine" },
            "name": { "type": "string", "analyzer": "keyword" }, "status": { "type": "string", "analyzer": "keyword" },
            "type": { "type": "string", "analyzer": "keyword" }
          }}}
        });
        ```

    *   **Index 4: `eat_agent_experiences` - Smart Memory Embeddings** (`vector_index_experiences_default`)
        ```javascript
        db.eat_agent_experiences.createSearchIndex({
          "name": "vector_index_experiences_default",
          "definition": { "mappings": { "dynamic": false, "fields": {
            "embeddings.primary_goal_description_embedding": { "type": "vector", "dimensions": YOUR_EMBEDDING_DIMENSION, "similarity": "cosine" },
            "embeddings.sub_task_description_embedding": { "type": "vector", "dimensions": YOUR_EMBEDDING_DIMENSION, "similarity": "cosine" },
            "embeddings.input_context_summary_embedding": { "type": "vector", "dimensions": YOUR_EMBEDDING_DIMENSION, "similarity": "cosine" },
            "embeddings.output_summary_embedding": { "type": "vector", "dimensions": YOUR_EMBEDDING_DIMENSION, "similarity": "cosine" }
          }}}
        });
        ```
    Wait for each command to return `{ ok: 1.0 }`. Check status with `db.collectionName.getSearchIndexes()`.

4.  **Exit `mongosh`**: `exit` or `Ctrl+D`.

#### Option 3.3.B: Using MongoDB Compass (GUI)

1.  **Connect MongoDB Compass to your Local Atlas Deployment:**
    *   Open MongoDB Compass.
    *   Create a new connection using the URI: `mongodb://localhost:27017/` (adjust port if non-default).
    *   Authentication should be `None`.
    *   Click "Connect".

2.  **Navigate to the Database and Collections:**
    *   Select your EAT database (e.g., `evolving_agents_db`). If it doesn't exist yet, you might need to run an EAT script once to create the collections, or manually create placeholder collections before defining indexes.
    *   For each collection (`eat_components`, `eat_agent_registry`, `eat_agent_experiences`):
        *   Select the collection.
        *   Go to the "Search Indexes" or "Atlas Search" tab (the exact naming/location might vary slightly in Compass versions for local Atlas features).
        *   Click "Create Search Index".

3.  **Define Indexes using the JSON Editor:**
    *   Choose "Atlas Vector Search" as the type if prompted.
    *   Select the "JSON Editor" configuration method.
    *   For each of the four indexes described in **Option 3.3.A (Step 3)**:
        *   Enter the correct **Index Name** (e.g., `idx_components_content_embedding`).
        *   Paste the corresponding **`definition` object** (the part inside `createSearchIndex({ ..., definition: { HERE } })`) into the JSON editor. Compass might only need the content of the `mappings` field or the whole `definition` depending on its UI for local Atlas search indexes. Refer to the structure from Option 3.3.A.
            *Example for `idx_components_content_embedding`'s definition for Compass JSON editor:*
            ```json
            { 
              "mappings": { 
                "dynamic": false, 
                "fields": {
                  "content_embedding": { "type": "vector", "dimensions": YOUR_EMBEDDING_DIMENSION, "similarity": "cosine" },
                  "record_type": { "type": "string", "analyzer": "keyword" }, 
                  "domain": { "type": "string", "analyzer": "keyword" },
                  "status": { "type": "string", "analyzer": "keyword" }, 
                  "tags": { "type": "string", "analyzer": "keyword", "multi": true }
                }
              }
            }
            ```
        *   Replace `YOUR_EMBEDDING_DIMENSION`.
        *   Save/Create the index.
    *   Wait for indexes to build and become "Active".

---

## 4. Configure EAT Project (`.env` file)

Ensure your EAT project connects to this local Atlas deployment.

1.  In your EAT project root, copy `.env.example` to `.env` if not done.
2.  Edit `.env` and set:
    ```env
    MONGODB_URI="mongodb://localhost:27017/" # Or your custom port
    MONGODB_DATABASE_NAME="evolving_agents_db" # Must match the DB used for index creation
    OPENAI_API_KEY="your-openai-api-key"
    # ... other settings ...
    ```

---

## 5. Running the EAT Application

Refer to the `EAT_EXAMPLES_TESTING_TUTORIAL.md` for detailed instructions on how to run EAT examples. In summary:
*   Ensure your Python virtual environment is active.
*   Run scripts directly (e.g., `python examples/invoice_processing/architect_zero_comprehensive_demo.py`).
*   Or, if using `docker-compose` for the EAT app, ensure it's configured to use the `MONGODB_URI` from your `.env` file pointing to `localhost`.

---

## 6. Managing Your Local Atlas Deployment

Use the Atlas CLI:
*   `atlas deployments list`
*   `atlas deployments start eat-local-dev`
*   `atlas deployments stop eat-local-dev`
*   `atlas deployments pause eat-local-dev`
*   `atlas deployments resume eat-local-dev`
*   `atlas deployments delete eat-local-dev --force` (Deletes data!)
*   `atlas deployments logs eat-local-dev -f`

---

## 7. Troubleshooting

*   **`atlas deployments start ... Error: deployment is in unexpected state: IDLE`**:
    *   Try `atlas deployments resume eat-local-dev`.
    *   If that fails, the deployment might be in a corrupted state. The safest is to perform a **fresh install** (delete and re-setup):
        ```bash
        atlas deployments delete eat-local-dev --force
        # (Optional) docker system prune -a --volumes  # BE CAREFUL: removes all unused Docker data
        atlas deployments setup eat-local-dev --type local --port 27017 --mdbVersion 7.0 
        ```
        Then re-create the indexes (Step 3.3).
*   **Cannot connect from EAT/Compass:** Verify deployment is `RUNNING` and the port in `MONGODB_URI` is correct.
*   **Vector Search Index Errors in EAT:** Double-check index names, database name, field paths, and `YOUR_EMBEDDING_DIMENSION` in your index definitions.
*   Ensure Docker is running.

---

## 8. Note on Production

Atlas CLI Local Deployment is **strictly for dev/test.** Use a managed MongoDB Atlas cloud cluster for production.