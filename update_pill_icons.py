import re

print("Updating pill icons: blue diamond for documents, blue circle for actions...")

# First, update MentionPill to use blue diamond icon
with open('ui/desktop/src/components/MentionPill.tsx', 'r') as f:
    mention_content = f.read()

# Replace the File import and icon with a blue diamond
old_mention_imports = "import { X, File } from 'lucide-react';"
new_mention_imports = "import { X, Diamond } from 'lucide-react';"

mention_content = mention_content.replace(old_mention_imports, new_mention_imports)

# Replace the File icon with Diamond icon and add blue color
old_mention_icon = '''      <span className="flex items-center gap-1">
        <File size={12} />
        {fileName}
      </span>'''

new_mention_icon = '''      <span className="flex items-center gap-1">
        <Diamond size={12} className="text-blue-500 fill-blue-500" />
        {fileName}
      </span>'''

mention_content = mention_content.replace(old_mention_icon, new_mention_icon)

# Write the updated MentionPill
with open('ui/desktop/src/components/MentionPill.tsx', 'w') as f:
    f.write(mention_content)

print("✅ Updated MentionPill:")
print("   - Changed from File icon to Diamond icon")
print("   - Added blue color (text-blue-500 fill-blue-500)")

# Now update ActionPill to use blue circle icons
with open('ui/desktop/src/components/ActionPill.tsx', 'r') as f:
    action_content = f.read()

# We need to modify how the icon is rendered to add blue color
# Find the icon rendering section
old_action_icon = '''      <span className="flex items-center gap-1">
        {icon}
        {label}
      </span>'''

new_action_icon = '''      <span className="flex items-center gap-1">
        <span className="relative">
          <div className="w-3 h-3 bg-blue-500 rounded-full absolute inset-0" />
          <span className="relative text-white text-[8px] flex items-center justify-center w-3 h-3">
            {icon}
          </span>
        </span>
        {label}
      </span>'''

action_content = action_content.replace(old_action_icon, new_action_icon)

# Write the updated ActionPill
with open('ui/desktop/src/components/ActionPill.tsx', 'w') as f:
    f.write(action_content)

print("✅ Updated ActionPill:")
print("   - Added blue circle background for icons")
print("   - Icons now appear as white symbols on blue circles")

print("\n🎯 Result:")
print("   - Document pills: Blue diamond icon + filename")
print("   - Action pills: Blue circle with white action icon + label")
print("   - Both maintain white pill background with consistent styling")
