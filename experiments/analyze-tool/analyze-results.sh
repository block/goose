#!/bin/bash
# Analyze experiment results
#
# Usage: ./analyze-results.sh [timestamp]
#   If timestamp not provided, analyzes most recent run

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RESULTS_DIR="$SCRIPT_DIR/results"

if [ ! -d "$RESULTS_DIR" ]; then
    echo "No results directory found. Run experiments first."
    exit 1
fi

# Find timestamp to analyze
if [ -n "$1" ]; then
    TIMESTAMP="$1"
else
    # Get most recent timestamp
    TIMESTAMP=$(ls "$RESULTS_DIR"/*.json 2>/dev/null | head -1 | sed 's/.*\/\([0-9_]*\)_.*/\1/' || echo "")
fi

if [ -z "$TIMESTAMP" ]; then
    echo "No results found in $RESULTS_DIR"
    exit 1
fi

echo "Analyze Tool Experiment Results"
echo "================================"
echo "Timestamp: $TIMESTAMP"
echo ""

# Summary table header
printf "%-10s %-18s %12s %12s %12s\n" "Question" "Condition" "Total" "Input" "Output"
printf "%-10s %-18s %12s %12s %12s\n" "--------" "---------" "-----" "-----" "------"

# Process each result file
for q in 1 2 3 4; do
    for condition in "with-analyze" "with-map" "without-analyze"; do
        file="$RESULTS_DIR/${TIMESTAMP}_q${q}_${condition}.json"
        if [ -f "$file" ]; then
            # Extract JSON portion (skip loading messages)
            json_content=$(sed -n '/^{/,$p' "$file")
            total=$(echo "$json_content" | jq -r '.metadata.accumulated_total_tokens // .metadata.total_tokens // "N/A"' 2>/dev/null)
            input=$(echo "$json_content" | jq -r '.metadata.accumulated_input_tokens // .metadata.input_tokens // "N/A"' 2>/dev/null)
            output=$(echo "$json_content" | jq -r '.metadata.accumulated_output_tokens // .metadata.output_tokens // "N/A"' 2>/dev/null)
            printf "%-10s %-18s %12s %12s %12s\n" "Q$q" "$condition" "$total" "$input" "$output"
        fi
    done
done

echo ""
echo "Token Comparison vs Baseline"
echo "============================"

for q in 1 2 3 4; do
    analyze_file="$RESULTS_DIR/${TIMESTAMP}_q${q}_with-analyze.json"
    map_file="$RESULTS_DIR/${TIMESTAMP}_q${q}_with-map.json"
    baseline_file="$RESULTS_DIR/${TIMESTAMP}_q${q}_without-analyze.json"
    
    if [ -f "$baseline_file" ]; then
        baseline_json=$(sed -n '/^{/,$p' "$baseline_file")
        baseline_total=$(echo "$baseline_json" | jq -r '.metadata.accumulated_total_tokens // .metadata.total_tokens // 0' 2>/dev/null)
        
        echo ""
        echo "Q$q (baseline=$baseline_total):"
        
        if [ -f "$analyze_file" ] && [ "$baseline_total" != "0" ] && [ "$baseline_total" != "null" ]; then
            analyze_json=$(sed -n '/^{/,$p' "$analyze_file")
            analyze_total=$(echo "$analyze_json" | jq -r '.metadata.accumulated_total_tokens // .metadata.total_tokens // 0' 2>/dev/null)
            savings=$(echo "scale=1; (($baseline_total - $analyze_total) * 100) / $baseline_total" | bc 2>/dev/null || echo "N/A")
            echo "  with-analyze: $analyze_total (${savings}% savings)"
        fi
        
        if [ -f "$map_file" ] && [ "$baseline_total" != "0" ] && [ "$baseline_total" != "null" ]; then
            map_json=$(sed -n '/^{/,$p' "$map_file")
            map_total=$(echo "$map_json" | jq -r '.metadata.accumulated_total_tokens // .metadata.total_tokens // 0' 2>/dev/null)
            savings=$(echo "scale=1; (($baseline_total - $map_total) * 100) / $baseline_total" | bc 2>/dev/null || echo "N/A")
            echo "  with-map:     $map_total (${savings}% savings)"
        fi
    fi
done

echo ""
echo "Tool Call Counts"
echo "================"

for q in 1 2 3 4; do
    for condition in "with-analyze" "with-map" "without-analyze"; do
        file="$RESULTS_DIR/${TIMESTAMP}_q${q}_${condition}.json"
        if [ -f "$file" ]; then
            echo ""
            echo "Q$q - $condition:"
            # Count tool calls by name
            jq -r '.messages[]? | select(.role == "assistant") | .content[]? | select(.type == "toolUse") | .name' "$file" 2>/dev/null | sort | uniq -c | sort -rn || echo "  (no tool calls found)"
        fi
    done
done

echo ""
echo "Files analyzed: $RESULTS_DIR/${TIMESTAMP}_*.json"
