# Tutorial: Testing EAT Examples with Atlas CLI Local MongoDB

This tutorial guides you through setting up the Evolving Agents Toolkit (EAT) and running its example scripts using a **MongoDB Atlas CLI Local Deployment** for your database. This method provides full Atlas Vector Search features locally for development.

## 1. Prerequisites

*   **Git:** For cloning the repository.
*   **Python 3.11+:** EAT is designed for Python 3.11 or newer.
*   **pip:** Python's package installer.
*   **Docker Desktop (or Docker Engine):** Required by the Atlas CLI to run its local MongoDB instance.
    *   [Install Docker](https://docs.docker.com/get-docker/)
*   **MongoDB Atlas CLI:** For creating and managing your local Atlas MongoDB deployment.
    *   [Install the Atlas CLI](https://www.mongodb.com/docs/atlas/cli/stable/install-atlas-cli/)
*   **(Optional) MongoDB Compass:** For a GUI to interact with your database and potentially create indexes.
*   **OpenAI API Key:** Most EAT examples require an OpenAI API key.

## 2. Setup Instructions

### Step 2.1: Clone the EAT Repository

```bash
git clone https://github.com/matiasmolinas/evolving-agents.git
cd Adaptive-Agents-Framework
```

### Step 2.2: Set Up Python Virtual Environment & Install EAT Dependencies

```bash
python -m venv venv
# macOS/Linux: source venv/bin/activate
# Windows: venv\Scripts\activate
pip install -r requirements.txt
pip install -e .
```

### Step 2.3: Set Up, Start, and Configure MongoDB Atlas CLI Local Deployment

This involves setting up the local MongoDB instance and creating the necessary Vector Search Indexes.

1.  **Set Up & Start Local Atlas Deployment:**
    *   Follow **Section 3, Steps 3.1 and 3.2** of the `docs/MONGO-SETUP.md` guide. This includes:
        *   Installing the Atlas CLI.
        *   Running `atlas deployments setup eat-local-dev --type local --port 27017 --mdbVersion 7.0` (or your preferred name/port).
    *   **Ensure your local deployment is RUNNING.** Check with `atlas deployments list`. If it's `IDLE` or `STOPPED`, use `atlas deployments start eat-local-dev` or `atlas deployments resume eat-local-dev`. If these commands give an "unexpected state: IDLE" error, refer to the "Fresh Install" instructions in `MONGO-SETUP.md` (Step 3.2).

2.  **Create Vector Search Indexes (CRITICAL):**
    *   Follow **Section 3, Step 3.3** of `docs/MONGO-SETUP.md`. You have two options:
        *   **Option A (Recommended): Using `mongosh`** (via `atlas deployments connect eat-local-dev --connectWith mongosh`). Copy-paste the four `db.collection.createSearchIndex({...})` commands provided there.
        *   **Option B: Using MongoDB Compass.** Connect Compass to `mongodb://localhost:27017/`, navigate to the collections, and use the "Search Indexes" / "Atlas Search" tab with the JSON editor to define the four indexes.
    *   **Remember to replace `YOUR_EMBEDDING_DIMENSION`** in the index definitions (e.g., 1536).

### Step 2.4: Configure EAT Environment Variables (`.env` file)

1.  In the EAT project root, copy `.env.example` to `.env`.
2.  Edit `.env`:
    *   Set `OPENAI_API_KEY="your-openai-api-key-here"`
    *   Ensure MongoDB connection points to your local Atlas deployment:
        ```env
        MONGODB_URI="mongodb://localhost:27017/" # Adjust port if changed
        MONGODB_DATABASE_NAME="evolving_agents_db" # Must match DB used for indexes
        ```

## 3. Running EAT Examples Directly on Host

With your Atlas CLI Local MongoDB running and the Python virtual environment activated:

1.  Navigate to the EAT project root (`Adaptive-Agents-Framework`).
2.  Execute example scripts directly:
    ```bash
    python examples/invoice_processing/architect_zero_comprehensive_demo.py
    python scripts/test_smart_memory.py
    # Add other examples you want to test
    ```
    The EAT application will connect to your local Atlas deployment.

## 4. Verifying Results

### Step 4.1: Check Local Output Files

Check project directory for files like `final_processing_output.json`.

### Step 4.2: Inspect MongoDB Collections

*   **Using `mongosh` (via Atlas CLI):**
    ```bash
    atlas deployments connect eat-local-dev --connectWith mongosh
    use evolving_agents_db;
    db.eat_components.findOne();
    // etc.
    ```
*   **Using MongoDB Compass:**
    Connect Compass to `mongodb://localhost:27017/`. Browse your `evolving_agents_db` and its collections (`eat_components`, `eat_agent_experiences`, etc.).

## 5. Troubleshooting

*   **`atlas deployments start eat-local-dev` Error: `deployment is in unexpected state: IDLE`**:
    Try `atlas deployments resume eat-local-dev`. If it still fails, perform a fresh install of the local deployment: `atlas deployments delete eat-local-dev --force` then `atlas deployments setup ...` again. You will need to re-create the search indexes.
*   **EAT App connection issues:**
    *   Ensure Atlas CLI local deployment is `RUNNING`.
    *   Verify `MONGODB_URI` in `.env` is `mongodb://localhost:27017/` (or correct port).
*   **Vector Search Index errors:** Confirm indexes are created correctly in your local Atlas deployment with the right names, database, collection targets, field paths, and embedding dimensions.
*   Refer to `docs/MONGO-SETUP.md` (Section 7) for more troubleshooting.

## 6. Stopping the Environment

1.  **Stop your MongoDB Atlas CLI Local Deployment:**
    ```bash
    atlas deployments stop eat-local-dev
    ```
    (Use `atlas deployments delete eat-local-dev --force` to remove it and its data completely).

## 7. Conclusion

This tutorial outlined using the MongoDB Atlas CLI Local Deployment. This setup provides an Atlas-feature-rich local MongoDB, allowing you to run EAT scripts directly on your host while leveraging powerful vector search.
```

**Key Changes in these Final Updates:**

*   **MONGO-SETUP.md:**
    *   Explicitly added "Perform a Fresh Install" instructions within Step 3.2 for clarity on resolving the "unexpected state: IDLE" issue.
    *   Added **Option B: Using MongoDB Compass (GUI)** within Step 3.3 for creating vector search indexes, providing a GUI alternative to `mongosh`.
*   **EAT_EXAMPLES_TESTING_TUTORIAL.md:**
    *   Directs users to the updated `MONGO-SETUP.md` for the detailed Atlas CLI local setup and index creation, including the "Fresh Install" advice and Compass option.
    *   Removed the "Option 3.B: Run EAT Application in Docker" to fully align with the "avoid Docker if possible" for the EAT app itself. The tutorial now primarily focuses on running Python scripts directly against the Atlas CLI-managed MongoDB. A note about the project's `docker-compose.yml` (for the app only) could be added back if there's a strong desire to keep that option, but the request was to minimize Docker.
    *   Simplified troubleshooting to reflect the current setup.