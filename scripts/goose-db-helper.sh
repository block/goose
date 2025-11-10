#!/usr/bin/env bash

set -euo pipefail

BACKUP_DIR="${HOME}/.local/share/goose/goose-db-backups"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

DRY_RUN=false
SKIP_CONFIRM=false
CLEAN_GENERATE=false

MIGRATIONS_DIR="${HOME}/.local/share/goose/migrations"
RUST_SESSION_MANAGER="crates/goose/src/session/session_manager.rs"

get_latest_version() {
    if [[ ! -d "${MIGRATIONS_DIR}" ]]; then
        echo "0"
        return
    fi

    local latest=$(find "${MIGRATIONS_DIR}" -mindepth 1 -maxdepth 1 -type d -name "[0-9]*" 2>/dev/null | \
                   sed 's/.*\/\([0-9]*\).*/\1/' | \
                   sed 's/^0*//' | \
                   sort -n | \
                   tail -1)

    echo "${latest:-0}"
}

find_migration_dir() {
    local version=$1

    if [[ ! -d "${MIGRATIONS_DIR}" ]]; then
        return
    fi

    local version_num=$(echo "${version}" | sed 's/^0*//')

    for dir in "${MIGRATIONS_DIR}"/*; do
        if [[ -d "${dir}" ]]; then
            local dir_version=$(basename "${dir}" | sed 's/^\([0-9]*\).*/\1/' | sed 's/^0*//')
            if [[ "${dir_version}" == "${version_num}" ]]; then
                echo "${dir}"
                return
            fi
        fi
    done
}

get_migration_info() {
    local version=$1

    if [[ "${version}" == "0" ]]; then
        echo "Initial schema (no schema_version table)"
        return
    fi

    local migration_dir=$(find_migration_dir "${version}")
    if [[ -z "${migration_dir}" ]]; then
        echo "Unknown migration"
        return
    fi

    local metadata_file="${migration_dir}/metadata.txt"
    if [[ -f "${metadata_file}" ]]; then
        local description=$(grep "^DESCRIPTION=" "${metadata_file}" | cut -d= -f2-)
        echo "${description}"
    else
        echo "Migration ${version}"
    fi
}

list_available_migrations() {
    echo -e "${BLUE}=== Available Migrations ===${NC}"
    echo ""
    echo -e "${CYAN}Version 0:${NC} Initial schema (no schema_version table)"
    echo ""

    if [[ ! -d "${MIGRATIONS_DIR}" ]]; then
        echo -e "${YELLOW}No migration files found in ${MIGRATIONS_DIR}${NC}"
        return
    fi

    for migration_dir in $(find "${MIGRATIONS_DIR}" -mindepth 1 -maxdepth 1 -type d -name "[0-9]*" | sort -V); do
        local version=$(basename "${migration_dir}" | sed 's/^\([0-9]*\).*/\1/')
        local info=$(get_migration_info "${version}")

        echo -e "${CYAN}Version ${version}:${NC} ${info}"
        echo ""
    done
}

get_goose_db_path() {
    if [[ -n "${GOOSE_PATH_ROOT:-}" ]]; then
        echo "${GOOSE_PATH_ROOT}/data/sessions/sessions.db"
    else
        local possible_paths=(
            "${HOME}/.local/share/goose/sessions/sessions.db"
            "${HOME}/Library/Application Support/Block/goose/data/sessions/sessions.db"
        )

        for path in "${possible_paths[@]}"; do
            if [[ -f "${path}" ]]; then
                echo "${path}"
                return
            fi
        done

        echo "${possible_paths[0]}"
    fi
}

DB_PATH=$(get_goose_db_path)

confirm_action() {
    local action="$1"

    if [[ "${SKIP_CONFIRM}" == "true" ]]; then
        return 0
    fi

    echo -e "${YELLOW}You are about to: ${action}${NC}"
    read -p "Continue? (y/N) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        return 0
    else
        return 1
    fi
}

check_db_exists() {
    if [[ ! -f "${DB_PATH}" ]]; then
        echo -e "${RED}ERROR: Database not found at ${DB_PATH}${NC}" >&2
        exit 1
    fi
}

get_schema_version() {
    check_db_exists
    local version=$(sqlite3 "${DB_PATH}" "SELECT MAX(version) FROM schema_version;" 2>/dev/null || echo "0")
    echo "${version}"
}

check_column_exists() {
    local table=$1
    local column=$2
    check_db_exists
    sqlite3 "${DB_PATH}" "PRAGMA table_info(${table});" | grep -q "^[0-9]*|${column}|"
}

get_table_schema() {
    local table=$1
    check_db_exists
    sqlite3 "${DB_PATH}" "PRAGMA table_info(${table});" 2>/dev/null || echo ""
}

create_backup() {
    check_db_exists
    mkdir -p "${BACKUP_DIR}"
    local timestamp=$(date +%Y%m%d_%H%M%S)
    local backup_path="${BACKUP_DIR}/sessions_v$(get_schema_version)_${timestamp}.db"
    cp "${DB_PATH}" "${backup_path}"
    echo -e "${GREEN}✓ Backup created: ${backup_path}${NC}"
    echo "${backup_path}"
}

show_version_history() {
    list_available_migrations
}

show_status() {
    echo -e "${BLUE}=== Goose Database Status ===${NC}"
    echo "Database path: ${DB_PATH}"
    echo ""

    if [[ ! -f "${DB_PATH}" ]]; then
        echo -e "${YELLOW}Status: No database found${NC}"
        echo ""
        echo "This is normal if you haven't run Goose yet."
        echo "Once you run Goose, a database will be created automatically."
        return
    fi

    local version=$(get_schema_version)
    local version_info=$(get_migration_info "${version}")
    local latest_version=$(get_latest_version)

    echo -e "Current schema version: ${CYAN}${version}${NC}"
    echo -e "Version info: ${version_info}"
    echo ""

    echo -e "${BLUE}Sessions table schema:${NC}"
    get_table_schema "sessions" | while IFS='|' read -r cid name type notnull dflt_value pk; do
        echo "  - ${name} (${type})"
    done
    echo ""

    local session_count=$(sqlite3 "${DB_PATH}" "SELECT COUNT(*) FROM sessions;" 2>/dev/null || echo "0")
    local message_count=$(sqlite3 "${DB_PATH}" "SELECT COUNT(*) FROM messages;" 2>/dev/null || echo "0")
    echo -e "${BLUE}Database contents:${NC}"
    echo "  Sessions: ${session_count}"
    echo "  Messages: ${message_count}"
    echo ""

    if [[ ${version} -eq ${latest_version} ]]; then
        echo -e "${GREEN}✓ Database is at the latest schema version${NC}"
    elif [[ ${version} -lt ${latest_version} ]]; then
        echo -e "${YELLOW}⚠ Database can be upgraded to v${latest_version}${NC}"
        echo "  Run: $0 migrate-to ${latest_version}"
    fi
}

apply_migration() {
    local target_version=$1

    if [[ "${target_version}" == "0" ]]; then
        echo -e "${RED}ERROR: Cannot migrate forward to version 0${NC}" >&2
        return 1
    fi

    local migration_dir=$(find_migration_dir "${target_version}")
    if [[ -z "${migration_dir}" ]]; then
        echo -e "${RED}ERROR: Migration files not found for version ${target_version}${NC}" >&2
        echo -e "${YELLOW}Expected to find directory: ${MIGRATIONS_DIR}/${target_version}_*${NC}"
        return 1
    fi

    local up_sql="${migration_dir}/up.sql"
    if [[ ! -f "${up_sql}" ]]; then
        echo -e "${RED}ERROR: Migration file not found: ${up_sql}${NC}" >&2
        return 1
    fi

    if ! sqlite3 "${DB_PATH}" < "${up_sql}"; then
        echo -e "${RED}ERROR: Migration to v${target_version} failed${NC}" >&2
        echo -e "${YELLOW}Check the SQL file: ${up_sql}${NC}"
        return 1
    fi
}

rollback_migration() {
    local from_version=$1

    if [[ "${from_version}" == "0" ]]; then
        echo -e "${RED}ERROR: Cannot rollback from version 0${NC}" >&2
        return 1
    fi

    local migration_dir=$(find_migration_dir "${from_version}")
    if [[ -z "${migration_dir}" ]]; then
        echo -e "${RED}ERROR: Migration files not found for version ${from_version}${NC}" >&2
        echo -e "${YELLOW}Expected to find directory: ${MIGRATIONS_DIR}/${from_version}_*${NC}"
        return 1
    fi

    local down_sql="${migration_dir}/down.sql"
    if [[ ! -f "${down_sql}" ]]; then
        echo -e "${RED}ERROR: Rollback file not found: ${down_sql}${NC}" >&2
        return 1
    fi

    if ! sqlite3 "${DB_PATH}" < "${down_sql}"; then
        echo -e "${RED}ERROR: Rollback from v${from_version} failed${NC}" >&2
        echo -e "${YELLOW}Check the SQL file: ${down_sql}${NC}"
        return 1
    fi
}

migrate_to_version() {
    local target_version=$1
    local latest_version=$(get_latest_version)

    if [[ -z "${target_version}" ]]; then
        echo -e "${RED}ERROR: Please specify a target version${NC}" >&2
        echo "Usage: $0 migrate-to <version>"
        echo ""
        echo "Available versions: 0 to ${latest_version}"
        return 1
    fi

    if [[ ! "${target_version}" =~ ^[0-9]+$ ]] || [[ ${target_version} -lt 0 ]] || [[ ${target_version} -gt ${latest_version} ]]; then
        echo -e "${RED}ERROR: Invalid version: ${target_version}${NC}" >&2
        echo "Valid versions are: 0 to ${latest_version}"
        return 1
    fi

    check_db_exists
    local current_version=$(get_schema_version)

    if [[ ${current_version} -eq ${target_version} ]]; then
        echo -e "${YELLOW}Already at version ${target_version}${NC}"
        return 0
    fi

    echo -e "${BLUE}=== Migrating database from v${current_version} to v${target_version} ===${NC}"
    echo ""

    if [[ "${DRY_RUN}" == "true" ]]; then
        echo -e "${CYAN}[DRY RUN] Would perform the following actions:${NC}"
        echo ""
        echo "1. Create backup at: ${BACKUP_DIR}/sessions_v${current_version}_<timestamp>.db"
        echo ""

        if [[ ${target_version} -gt ${current_version} ]]; then
            echo "2. Apply forward migrations:"
            for version in $(seq $((current_version + 1)) ${target_version}); do
                local migration_info=$(get_migration_info "${version}")
                local migration_dir=$(find_migration_dir "${version}")
                echo "   - Migrate to v${version}: ${migration_info}"
                echo "     SQL file: ${migration_dir}/up.sql"
            done
        else
            echo "2. Apply rollback migrations:"
            for version in $(seq ${current_version} -1 $((target_version + 1))); do
                local migration_info=$(get_migration_info "${version}")
                local migration_dir=$(find_migration_dir "${version}")
                echo "   - Rollback from v${version}: ${migration_info}"
                echo "     SQL file: ${migration_dir}/down.sql"
            done
        fi

        echo ""
        echo "3. Update schema_version table to ${target_version}"
        echo ""
        echo -e "${CYAN}[DRY RUN] No changes were made${NC}"
        return 0
    fi

    if ! confirm_action "migrate database from v${current_version} to v${target_version}"; then
        echo -e "${YELLOW}Migration cancelled${NC}"
        return 2
    fi

    local backup_path=$(create_backup)
    echo ""

    if [[ ${target_version} -gt ${current_version} ]]; then
        for version in $(seq $((current_version + 1)) ${target_version}); do
            local migration_info=$(get_migration_info "${version}")
            echo -e "Applying migration to v${version}..."
            apply_migration ${version}
            echo -e "${GREEN}✓ Migrated to v${version}: ${migration_info}${NC}"
        done
    else
        for version in $(seq ${current_version} -1 $((target_version + 1))); do
            local migration_info=$(get_migration_info "${version}")
            echo -e "Rolling back from v${version}..."
            rollback_migration ${version}
            echo -e "${GREEN}✓ Rolled back from v${version}${NC}"
        done
    fi

    echo ""
    echo -e "${GREEN}✓ Migration complete!${NC}"
    echo -e "Database is now at version ${target_version}"
    echo ""
    echo "Backup saved at: ${backup_path}"
}

list_backups() {
    if [[ ! -d "${BACKUP_DIR}" ]] || [[ -z "$(ls -A "${BACKUP_DIR}" 2>/dev/null)" ]]; then
        echo -e "${YELLOW}No backups found${NC}"
        return
    fi

    echo -e "${BLUE}=== Available Backups ===${NC}"
    echo ""
    ls -lh "${BACKUP_DIR}" | tail -n +2 | while read -r line; do
        local filename=$(echo "${line}" | awk '{print $NF}')
        local size=$(echo "${line}" | awk '{print $5}')
        local date=$(echo "${line}" | awk '{print $6, $7, $8}')

        if [[ "${filename}" =~ _v([0-9]+)_ ]]; then
            local version="${BASH_REMATCH[1]}"
            echo -e "${filename}"
            echo -e "  Size: ${size}, Date: ${date}, Schema: v${version}"
            echo ""
        else
            echo -e "${filename}"
            echo -e "  Size: ${size}, Date: ${date}"
            echo ""
        fi
    done
}

restore_backup() {
    local backup_file=$1

    if [[ -z "${backup_file}" ]]; then
        echo -e "${RED}ERROR: Please specify a backup file to restore${NC}" >&2
        echo "Usage: $0 restore <backup-file>"
        echo ""
        list_backups
        exit 1
    fi

    if [[ ! -f "${backup_file}" ]]; then
        echo -e "${RED}ERROR: Backup file not found: ${backup_file}${NC}" >&2
        exit 1
    fi

    check_db_exists

    if [[ "${DRY_RUN}" == "true" ]]; then
        echo -e "${CYAN}[DRY RUN] Would perform the following actions:${NC}"
        echo ""
        echo "1. Create backup of current database at: ${BACKUP_DIR}/sessions_v<current-version>_<timestamp>.db"
        echo "2. Restore backup from: ${backup_file}"
        echo "3. Replace current database at: ${DB_PATH}"
        echo ""
        echo -e "${CYAN}[DRY RUN] No changes were made${NC}"
        return 0
    fi

    if ! confirm_action "restore backup from ${backup_file} (this will replace your current database)"; then
        echo -e "${YELLOW}Restore cancelled${NC}"
        return 2
    fi

    local current_backup=$(create_backup)
    echo ""

    cp "${backup_file}" "${DB_PATH}"
    echo -e "${GREEN}✓ Restored backup from: ${backup_file}${NC}"
    echo "Current database backed up to: ${current_backup}"
}

validate_sql_syntax() {
    local sql=$1
    local file_desc=$2

    if [[ -z "$sql" ]]; then
        echo -e "${YELLOW}⚠ WARNING: Empty SQL in $file_desc${NC}" >&2
        return 1
    fi

    if ! echo "$sql" | grep -q ";"; then
        echo -e "${YELLOW}⚠ WARNING: No semicolons found in $file_desc${NC}" >&2
        return 1
    fi

    local lines=$(echo "$sql" | grep -v "^--" | grep -v "^BEGIN" | grep -v "^COMMIT" | grep -v "^$")
    while IFS= read -r line; do
        if [[ -n "$line" ]]; then
            if ! echo "$line" | grep -q ";$"; then
                local next_line=$(echo "$lines" | grep -A1 "^$line$" | tail -1)
                if [[ -n "$next_line" && ! "$next_line" =~ ^(BEGIN|COMMIT|INSERT|DELETE|$) ]]; then
                    echo -e "${YELLOW}⚠ WARNING: Possible missing semicolon in $file_desc:${NC}" >&2
                    echo "  $line" >&2
                    return 1
                fi
            fi
        fi
    done <<< "$lines"

    return 0
}

extract_migration_sql() {
    local version=$1
    local rust_file=$2

    awk -v ver="$version" '
        BEGIN { in_migration=0; sql=""; query_count=0; current_query="" }
        /async fn apply_migration/ { found_func=1 }
        found_func && $0 ~ ver " =>" { in_migration=1; next }
        in_migration && /}$/ && !/=>/ { exit }
        in_migration && /sqlx::query/ {
            getline
            if ($0 ~ /r#"/) {
                if (current_query != "") {
                    if (sql != "") sql = sql ";\n"
                    sql = sql current_query
                    current_query = ""
                }
                getline
                while ($0 !~ /"#/) {
                    if (current_query != "") current_query = current_query "\n"
                    current_query = current_query $0
                    getline
                }
                query_count++
            }
        }
        END {
            if (current_query != "") {
                if (sql != "") sql = sql ";\n"
                sql = sql current_query
            }
            if (sql != "") print sql ";"
        }
    ' "$rust_file"
}

generate_rollback_sql() {
    local version=$1
    local up_sql=$2

    echo "BEGIN TRANSACTION;"
    echo ""

    local statements=()
    mapfile -d $'\0' -t statements < <(echo "$up_sql" | awk 'BEGIN{RS=";"} {gsub(/^[ \t\n]+|[ \t\n]+$/, ""); if (length($0) > 0) {print $0; printf "%c", 0}}')

    local rollback_stmts=()
    local has_unsupported=false

    for stmt in "${statements[@]}"; do
        if echo "$stmt" | grep -q "CREATE TABLE.*schema_version"; then
            rollback_stmts+=("DROP TABLE IF EXISTS schema_version;")
        elif echo "$stmt" | grep -q "RENAME COLUMN"; then
            local table=$(echo "$stmt" | sed -n 's/.*ALTER TABLE \([^ ]*\).*/\1/p')
            local old_col=$(echo "$stmt" | sed -n 's/.*RENAME COLUMN \([^ ]*\) TO.*/\1/p')
            local new_col=$(echo "$stmt" | sed -n 's/.*TO \([^ ;]*\).*/\1/p')
            rollback_stmts+=("ALTER TABLE $table RENAME COLUMN $new_col TO $old_col;")
        elif echo "$stmt" | grep -q "ADD COLUMN"; then
            local table=$(echo "$stmt" | sed -n 's/.*ALTER TABLE \([^ ]*\).*/\1/p')
            local column=$(echo "$stmt" | sed -n 's/.*ADD COLUMN \([^ ]*\).*/\1/p')
            rollback_stmts+=("ALTER TABLE $table DROP COLUMN $column;")
        else
            rollback_stmts+=("-- TODO: Unable to auto-generate rollback for: $stmt")
            has_unsupported=true
        fi
    done

    for ((i=${#rollback_stmts[@]}-1; i>=0; i--)); do
        echo "${rollback_stmts[$i]}"
    done

    echo ""
    echo "DELETE FROM schema_version WHERE version = $version;"
    echo ""
    echo "COMMIT;"

    if [[ "$has_unsupported" == "true" ]]; then
        return 1
    fi
}

generate_metadata() {
    local version=$1
    local sql=$2
    local author=${USER:-system}
    local date=$(date +%Y-%m-%d)

    local description="Migration $version"
    if echo "$sql" | grep -q "CREATE TABLE.*schema_version"; then
        description="Added schema_version tracking"
    elif echo "$sql" | grep -q "ALTER TABLE.*ADD COLUMN"; then
        local column=$(echo "$sql" | sed -n 's/.*ADD COLUMN \([^ ]*\).*/\1/p')
        description="Added $column column"
    elif echo "$sql" | grep -q "RENAME COLUMN"; then
        local old_col=$(echo "$sql" | sed -n 's/.*RENAME COLUMN \([^ ]*\) TO.*/\1/p')
        local new_col=$(echo "$sql" | sed -n 's/.*TO \([^ ]*\).*/\1/p')
        description="Renamed $old_col to $new_col"
    fi

    cat <<EOF
DESCRIPTION=$description
AUTHOR=$author
DATE=$date
NOTES=Auto-generated from session_manager.rs
EOF
}

generate_migrations() {
    if [[ ! -f "${RUST_SESSION_MANAGER}" ]]; then
        echo -e "${RED}ERROR: Rust source file not found: ${RUST_SESSION_MANAGER}${NC}" >&2
        echo "Make sure you're running this from the goose repository root."
        exit 1
    fi

    echo -e "${BLUE}=== Generating Migrations from Rust Source ===${NC}"
    echo ""
    echo "Reading migrations from: ${RUST_SESSION_MANAGER}"
    echo "Output directory: ${MIGRATIONS_DIR}"
    echo ""

    if [[ "${CLEAN_GENERATE}" == "true" ]]; then
        if [[ -d "${MIGRATIONS_DIR}" ]]; then
            local migration_count=$(find "${MIGRATIONS_DIR}" -mindepth 1 -maxdepth 1 -type d -name "[0-9]*" 2>/dev/null | wc -l)
            if [[ ${migration_count} -gt 0 ]]; then
                echo -e "${YELLOW}⚠ Clean mode: This will remove all ${migration_count} existing migration(s)${NC}"
                if ! confirm_action "remove all existing migrations and regenerate from source"; then
                    echo -e "${YELLOW}Generation cancelled${NC}"
                    return 2
                fi
                echo "Removing existing migrations..."
                rm -rf "${MIGRATIONS_DIR}"
            fi
        fi
    fi

    mkdir -p "${MIGRATIONS_DIR}"

    local max_version=$(grep -E '^\s+[0-9]+ =>' "${RUST_SESSION_MANAGER}" | \
                         sed 's/[^0-9]//g' | \
                         sort -n | \
                         tail -1)

    if [[ -z "$max_version" ]]; then
        max_version=2
    fi

    local generated_count=0
    local skipped_count=0

    for version in $(seq 1 $max_version); do
        local padded_version=$(printf "%03d" $version)
        local sql=$(extract_migration_sql "$version" "${RUST_SESSION_MANAGER}")

        if [[ -z "$sql" ]]; then
            echo -e "${YELLOW}⚠ No SQL found for version $version, skipping...${NC}"
            skipped_count=$((skipped_count + 1))
            continue
        fi

        if ! validate_sql_syntax "$sql" "migration v$version"; then
            echo -e "${YELLOW}⚠ Validation warning for version $version, but continuing...${NC}"
        fi

        local migration_name
        if echo "$sql" | grep -q "CREATE TABLE.*schema_version"; then
            migration_name="add_schema_version"
        elif echo "$sql" | grep -q "ALTER TABLE.*ADD COLUMN"; then
            local column=$(echo "$sql" | sed -n 's/.*ADD COLUMN \([^ ]*\).*/\1/p' | head -1)
            migration_name="add_${column}"
        elif echo "$sql" | grep -q "RENAME COLUMN"; then
            local old_col=$(echo "$sql" | sed -n 's/.*RENAME COLUMN \([^ ]*\) TO.*/\1/p')
            local new_col=$(echo "$sql" | sed -n 's/.*TO \([^ ]*\).*/\1/p')
            migration_name="rename_${old_col}_to_${new_col}"
        else
            migration_name="migration_${version}"
        fi

        local migration_dir="${MIGRATIONS_DIR}/${padded_version}_${migration_name}"
        mkdir -p "$migration_dir"

        echo "BEGIN TRANSACTION;" > "${migration_dir}/up.sql"
        echo "" >> "${migration_dir}/up.sql"
        echo "$sql" >> "${migration_dir}/up.sql"
        echo "" >> "${migration_dir}/up.sql"
        echo "INSERT INTO schema_version (version) VALUES ($version);" >> "${migration_dir}/up.sql"
        echo "" >> "${migration_dir}/up.sql"
        echo "COMMIT;" >> "${migration_dir}/up.sql"

        generate_rollback_sql "$version" "$sql" > "${migration_dir}/down.sql"

        generate_metadata "$version" "$sql" > "${migration_dir}/metadata.txt"

        echo -e "${GREEN}✓ Generated migration $padded_version: ${migration_dir##*/}${NC}"
        generated_count=$((generated_count + 1))
    done

    echo ""
    echo -e "${GREEN}✓ Generation complete!${NC}"
    echo "Generated: $generated_count migrations"
    if [[ $skipped_count -gt 0 ]]; then
        echo "Skipped: $skipped_count migrations"
    fi
    echo ""
    echo -e "${YELLOW}Note:${NC} Please review generated rollback SQL (down.sql) files."
    echo "Some migrations may require manual rollback implementation."
}

show_help() {
    local latest_version=$(get_latest_version)

    echo -e "${BLUE}Goose Database Migration Helper${NC}"
    echo ""
    echo "This script is a developer utility for manually managing database schema"
    echo "versions when switching between branches with different schema requirements."
    echo "Migrations are stored in ${MIGRATIONS_DIR}."
    echo ""
    echo -e "${CYAN}Usage:${NC} $0 [flags] <command> [arguments] [flags]"
    echo ""
    echo -e "${CYAN}Global Flags (can be placed before or after the command):${NC}"
    echo -e "    ${GREEN}--dry-run${NC}"
    echo "        Preview changes without modifying the database"
    echo "        Works with: migrate-to, restore"
    echo ""
    echo -e "    ${GREEN}--yes, -y${NC}"
    echo "        Skip confirmation prompts (useful for automation)"
    echo "        Works with: migrate-to, restore, generate-migrations --clean"
    echo ""
    echo -e "    ${GREEN}--clean${NC}"
    echo "        Remove all existing migrations before regenerating"
    echo "        Works with: generate-migrations"
    echo "        Useful when switching between branches with different migrations"
    echo ""
    echo -e "${CYAN}Commands:${NC}"
    echo -e "    ${GREEN}status${NC}"
    echo "        Show current database schema version, table structure, and statistics"
    echo ""
    echo -e "    ${GREEN}migrate-to <version>${NC}"
    echo "        Migrate database to a specific schema version (0-${latest_version})"
    echo "        Automatically handles forward migrations and rollbacks"
    echo ""
    echo -e "    ${GREEN}history${NC}"
    echo "        Show all available migrations and their descriptions"
    echo ""
    echo -e "    ${GREEN}generate-migrations${NC}"
    echo "        Auto-generate migration files from Rust source code (session_manager.rs)"
    echo "        Creates up.sql, down.sql, and metadata.txt for each migration"
    echo ""
    echo -e "    ${GREEN}backup${NC}"
    echo "        Create a manual backup of the current database"
    echo ""
    echo -e "    ${GREEN}list-backups${NC}"
    echo "        Show all available backups with their versions and sizes"
    echo ""
    echo -e "    ${GREEN}restore <file>${NC}"
    echo "        Restore database from a backup file"
    echo ""
    echo -e "    ${GREEN}help${NC}"
    echo "        Show this help message"
    echo ""
    echo -e "${CYAN}Examples:${NC}"
    echo "    # Check current status"
    echo "    $0 status"
    echo ""
    echo "    # View all available migrations"
    echo "    $0 history"
    echo ""
    echo "    # Preview migration without making changes (dry-run before)"
    echo "    $0 --dry-run migrate-to 3"
    echo ""
    echo "    # Flags can also be placed after the command and arguments"
    echo "    $0 migrate-to 3 --dry-run"
    echo ""
    echo "    # Migrate to version 2"
    echo "    $0 migrate-to 2"
    echo ""
    echo "    # Rollback to version 1 without confirmation prompt"
    echo "    $0 migrate-to 1 --yes"
    echo ""
    echo "    # Create a backup"
    echo "    $0 backup"
    echo ""
    echo "    # Clean regenerate migrations (useful when switching branches)"
    echo "    $0 generate-migrations --clean"
    echo ""
    echo "    # Clean regenerate without confirmation"
    echo "    $0 generate-migrations --clean --yes"
    echo ""
    echo -e "${CYAN}Adding New Migrations:${NC}"
    echo "    After adding a migration to session_manager.rs, run:"
    echo ""
    echo "    $0 generate-migrations"
    echo ""
    echo "    This will automatically extract migrations from the Rust source"
    echo "    and create the necessary SQL files in ${MIGRATIONS_DIR}."
    echo ""
    echo -e "    ${YELLOW}Note:${NC} Review generated down.sql files, as some rollbacks"
    echo -e "    may require manual implementation."
    echo ""
    echo -e "${CYAN}Switching Branches:${NC}"
    echo "    When switching between branches with different migrations:"
    echo ""
    echo "    # Clean and regenerate to match current branch"
    echo "    git checkout main"
    echo "    $0 generate-migrations --clean"
    echo ""
    echo "    # Or manually remove specific migrations"
    echo "    rm -rf ~/.local/share/goose/migrations/004_*"
    echo "    $0 generate-migrations"
    echo ""
    echo -e "${CYAN}Configuration:${NC}"
    echo "    Database:   ${DB_PATH}"
    echo "    Backups:    ${BACKUP_DIR}"
    echo "    Migrations: ${MIGRATIONS_DIR}"
    echo "    Latest:     v${latest_version}"
    echo ""
    echo -e "${YELLOW}Note:${NC} All migrations automatically create backups before making changes."
}

main() {
    local non_flag_args=()

    while [[ $# -gt 0 ]]; do
        case "$1" in
            --dry-run)
                DRY_RUN=true
                shift
                ;;
            --yes|-y)
                SKIP_CONFIRM=true
                shift
                ;;
            --clean)
                CLEAN_GENERATE=true
                shift
                ;;
            --help|-h)
                show_help
                exit 0
                ;;
            -*)
                echo -e "${RED}ERROR: Unknown flag: $1${NC}" >&2
                echo ""
                show_help
                exit 1
                ;;
            *)
                non_flag_args+=("$1")
                shift
                ;;
        esac
    done

    local command=${non_flag_args[0]:-help}

    case "${command}" in
        status)
            show_status
            ;;
        migrate-to)
            migrate_to_version "${non_flag_args[1]}"
            ;;
        history)
            show_version_history
            ;;
        generate-migrations)
            generate_migrations
            ;;
        backup)
            create_backup
            ;;
        list-backups)
            list_backups
            ;;
        restore)
            restore_backup "${non_flag_args[1]}"
            ;;
        migrate)
            local latest_version=$(get_latest_version)
            echo -e "${YELLOW}Note: 'migrate' is deprecated. Use 'migrate-to ${latest_version}' instead.${NC}"
            echo ""
            migrate_to_version ${latest_version}
            ;;
        rollback)
            echo -e "${YELLOW}Note: 'rollback' is deprecated. Use 'migrate-to <version>' instead.${NC}"
            echo -e "${YELLOW}Use '$0 history' to see available versions.${NC}"
            echo ""
            show_version_history
            ;;
        compatible-with)
            echo -e "${RED}ERROR: 'compatible-with' command has been removed.${NC}" >&2
            echo ""
            echo "The script now uses a generic migration system."
            echo "To migrate your database, use: $0 migrate-to <version>"
            echo ""
            echo "Available migrations:"
            show_version_history
            exit 1
            ;;
        help)
            show_help
            ;;
        *)
            echo -e "${RED}ERROR: Unknown command: ${command}${NC}" >&2
            echo ""
            show_help
            exit 1
            ;;
    esac
}

main "$@"
