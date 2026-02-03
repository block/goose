default:
  just --list

up:
  docker-compose up -d

kill:
  docker-compose kill

build:
  docker-compose build

ps:
  docker-compose ps

exec *args:
  docker-compose exec app {{args}}

logs *args:
    docker-compose logs {{args}} -f

ruff *args:
  docker compose exec app ruff check {{args}} src
  docker compose exec app ruff format src

lint:
  just ruff --fix

backup:
  docker compose exec app_db scripts/backup

# examples:
# "just get-backup dump_name_2021-01-01..backup.gz" to copy particular backup
# "just get-backup" to copy directory (backups) with all dumps
mount-docker-backup *args:
  docker cp app_db:/backups/{{args}} ./{{args}}

restore *args:
    docker compose exec app_db scripts/restore {{args}}

test *args:
    docker compose exec app pytest {{args}}

# Run pytest with this repo's venv (avoids wrong interpreter from another project)
# Windows: just test-local   |  Unix: ./.venv/bin/python -m pytest tests/ -v
test-local *args:
    .\.venv\Scripts\python.exe -m pytest {{args}}

# Database migration commands
# Usage: just db-migrate [cmd] [args]
# Examples:
#   just db-migrate create "add user table"
#   just db-migrate upgrade head
#   just db-migrate downgrade -1
db-migrate *args:
    docker compose exec app scripts/db-migrate.sh {{args}}

# Generate a migration without autogenerate
db-create *args:
    docker compose exec app alembic revision -m "{{args}}"

# Apply all migrations
db-upgrade:
    docker compose exec app alembic upgrade head

# Show current migration version
db-current:
    docker compose exec app alembic current
