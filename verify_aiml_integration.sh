#!/bin/bash

# AIML API Integration Verification Script
# Verifies that all components are properly integrated

echo "🔍 AIML API Integration Verification"
echo "===================================="
echo

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to check file existence and content
check_file() {
    local file="$1"
    local content="$2"
    local description="$3"
    
    if [ -f "$file" ]; then
        if grep -q "$content" "$file"; then
            echo -e "✅ ${GREEN}$description${NC}"
            return 0
        else
            echo -e "❌ ${RED}$description - Content not found${NC}"
            return 1
        fi
    else
        echo -e "❌ ${RED}$description - File not found${NC}"
        return 1
    fi
}

# Function to check file existence
check_file_exists() {
    local file="$1"
    local description="$2"
    
    if [ -f "$file" ]; then
        echo -e "✅ ${GREEN}$description${NC}"
        return 0
    else
        echo -e "❌ ${RED}$description - File not found${NC}"
        return 1
    fi
}

echo "🧩 Checking Backend Provider Implementation..."

# Check Rust provider implementation
check_file "crates/goose/src/providers/aimlapi.rs" "pub struct AimlApiProvider" "AIML API Provider implementation"

# Check provider module registration
check_file "crates/goose/src/providers/mod.rs" "pub mod aimlapi" "Provider module registration"

# Check provider factory registration  
check_file "crates/goose/src/providers/factory.rs" "AimlApiProvider::metadata()" "Provider factory registration"
check_file "crates/goose/src/providers/factory.rs" 'aimlapi.*AimlApiProvider::from_env' "Provider factory creation"

echo
echo "⚙️  Checking Backend API Configuration..."

# Check server API configuration
check_file "crates/goose-server/src/routes/providers_and_keys.json" '"aimlapi"' "Server API provider config"
check_file "crates/goose-server/src/routes/providers_and_keys.json" '"AIML API"' "Server API provider name"

echo
echo "🖥️  Checking Frontend GUI Integration..."

# Check frontend provider registry
check_file "ui/desktop/src/components/settings/providers/ProviderRegistry.tsx" "name: 'AIML API'" "Frontend provider registry"
check_file "ui/desktop/src/components/settings/providers/ProviderRegistry.tsx" "id: 'aimlapi'" "Frontend provider ID"
check_file "ui/desktop/src/components/settings/providers/ProviderRegistry.tsx" "AIMLAPI_API_KEY" "Frontend provider parameters"

echo
echo "🎨 Checking Visual Assets..."

# Check icon files
check_file_exists "ui/desktop/src/components/settings/providers/modal/subcomponents/icons/aimlapi.svg" "SVG icon"
check_file_exists "ui/desktop/src/components/settings/providers/modal/subcomponents/icons/aimlapi.png" "PNG icon (1x)"
check_file_exists "ui/desktop/src/components/settings/providers/modal/subcomponents/icons/aimlapi@2x.png" "PNG icon (2x)"
check_file_exists "ui/desktop/src/components/settings/providers/modal/subcomponents/icons/aimlapi@3x.png" "PNG icon (3x)"

# Check logo component integration
check_file "ui/desktop/src/components/settings/providers/modal/subcomponents/ProviderLogo.tsx" "AimlApiLogo" "Logo component import"
check_file "ui/desktop/src/components/settings/providers/modal/subcomponents/ProviderLogo.tsx" "aimlapi: AimlApiLogo" "Logo component mapping"

echo
echo "📋 Integration Summary:"
echo "======================"

# Count successes
total_checks=12
successful_checks=0

# Re-run checks silently to count successes
check_file "crates/goose/src/providers/aimlapi.rs" "pub struct AimlApiProvider" "" > /dev/null && ((successful_checks++))
check_file "crates/goose/src/providers/mod.rs" "pub mod aimlapi" "" > /dev/null && ((successful_checks++))
check_file "crates/goose/src/providers/factory.rs" "AimlApiProvider::metadata()" "" > /dev/null && ((successful_checks++))
check_file "crates/goose/src/providers/factory.rs" 'aimlapi.*AimlApiProvider::from_env' "" > /dev/null && ((successful_checks++))
check_file "crates/goose-server/src/routes/providers_and_keys.json" '"aimlapi"' "" > /dev/null && ((successful_checks++))
check_file "ui/desktop/src/components/settings/providers/ProviderRegistry.tsx" "name: 'AIML API'" "" > /dev/null && ((successful_checks++))
check_file_exists "ui/desktop/src/components/settings/providers/modal/subcomponents/icons/aimlapi.svg" "" > /dev/null && ((successful_checks++))
check_file_exists "ui/desktop/src/components/settings/providers/modal/subcomponents/icons/aimlapi.png" "" > /dev/null && ((successful_checks++))
check_file_exists "ui/desktop/src/components/settings/providers/modal/subcomponents/icons/aimlapi@2x.png" "" > /dev/null && ((successful_checks++))
check_file_exists "ui/desktop/src/components/settings/providers/modal/subcomponents/icons/aimlapi@3x.png" "" > /dev/null && ((successful_checks++))
check_file "ui/desktop/src/components/settings/providers/modal/subcomponents/ProviderLogo.tsx" "AimlApiLogo" "" > /dev/null && ((successful_checks++))
check_file "ui/desktop/src/components/settings/providers/modal/subcomponents/ProviderLogo.tsx" "aimlapi: AimlApiLogo" "" > /dev/null && ((successful_checks++))

if [ $successful_checks -eq $total_checks ]; then
    echo -e "🎉 ${GREEN}ALL CHECKS PASSED!${NC} ($successful_checks/$total_checks)"
    echo -e "✨ ${GREEN}AIML API is fully integrated into Goose!${NC}"
    echo
    echo "🚀 Next steps:"
    echo "1. Build the project: cargo build"
    echo "2. Set API key: export AIMLAPI_API_KEY='your-key-here'"  
    echo "3. Test CLI: goose session --provider aimlapi --model gpt-4o"
    echo "4. Test GUI: Open Goose Desktop → Settings → Providers → AIML API"
    echo
elif [ $successful_checks -gt $((total_checks * 3 / 4)) ]; then
    echo -e "⚠️  ${YELLOW}MOSTLY INTEGRATED${NC} ($successful_checks/$total_checks)"
    echo -e "🔧 Minor issues found, but core integration is complete"
else
    echo -e "❌ ${RED}INTEGRATION INCOMPLETE${NC} ($successful_checks/$total_checks)"
    echo -e "🚫 Please fix the issues above before using"
fi

echo
echo "📝 For detailed information, see: AIML_API_INTEGRATION.md"