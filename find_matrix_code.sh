#!/bin/bash

# Script to find Matrix-related code in the Goose Desktop codebase
# This will help locate where Matrix rooms are being opened

echo "=== Searching for Matrix Integration Code ==="
echo ""

echo "1. Searching for Matrix room IDs (format: !xxx:server.com)..."
find ui/desktop/src -type f \( -name "*.ts" -o -name "*.tsx" \) -exec grep -l "!.*:.*\." {} \; 2>/dev/null | grep -v node_modules

echo ""
echo "2. Searching for 'room' with 'open', 'click', or 'select'..."
grep -rn "room" ui/desktop/src --include="*.tsx" --include="*.ts" | grep -i "open\|click\|select" | grep -v node_modules | head -20

echo ""
echo "3. Searching for session ID patterns (YYYYMMDD_XX)..."
grep -rn "20[0-9][0-9][0-9][0-9][0-9][0-9]_[0-9]" ui/desktop/src --include="*.tsx" --include="*.ts" | grep -v node_modules | head -10

echo ""
echo "4. Searching for 'Space' or 'Channel' components..."
find ui/desktop/src/components -type f -name "*Space*.tsx" -o -name "*Channel*.tsx" -o -name "*Room*.tsx" 2>/dev/null

echo ""
echo "5. Searching for navigation to '/pair'..."
grep -rn "navigate.*pair\|'/pair'" ui/desktop/src --include="*.tsx" --include="*.ts" | grep -v node_modules | head -20

echo ""
echo "6. Searching for tab creation/management..."
grep -rn "addTab\|createTab\|openTab\|newTab" ui/desktop/src --include="*.tsx" --include="*.ts" | grep -v node_modules | head -20

echo ""
echo "7. Searching for chat object creation..."
grep -rn "id:.*session\|ChatType.*{" ui/desktop/src --include="*.tsx" --include="*.ts" | grep -v node_modules | head -20

echo ""
echo "8. Searching for Matrix-related imports or services..."
grep -rn "import.*matrix\|from.*matrix" ui/desktop/src --include="*.tsx" --include="*.ts" -i | grep -v node_modules | head -20

echo ""
echo "9. Checking for external Matrix packages..."
grep -i "matrix" ui/desktop/package.json 2>/dev/null

echo ""
echo "10. Searching for collaborative/shared session code..."
grep -rn "collaborative\|Collaborative\|shared.*session" ui/desktop/src --include="*.tsx" --include="*.ts" -i | grep -v node_modules | head -20

echo ""
echo "=== Search Complete ==="
echo ""
echo "If you found relevant files, use createMatrixRoomChat() from utils/matrixRoomHelper.ts"
echo "See MATRIX_SESSION_FIX_IMPLEMENTATION.md for details"
