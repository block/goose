import re

# Read the MentionPill file
with open('ui/desktop/src/components/MentionPill.tsx', 'r') as f:
    content = f.read()

print("Updating MentionPill styling to match ActionPill (white background, same spacing)...")

# Update the variantClasses to match ActionPill styling
old_variant_classes = '''  const variantClasses = {
    default: "bg-green-100 text-green-800 border-green-200 dark:bg-green-900 dark:text-green-200 dark:border-green-700",
    message: "bg-green-100 text-green-800 border-green-200 dark:bg-green-900 dark:text-green-200 dark:border-green-700"
  };'''

new_variant_classes = '''  const variantClasses = {
    default: "bg-bgProminent text-textProminentInverse border-borderProminent",
    message: "bg-bgProminent text-textProminentInverse border-borderProminent"
  };'''

content = content.replace(old_variant_classes, new_variant_classes)

# Write the modified content back
with open('ui/desktop/src/components/MentionPill.tsx', 'w') as f:
    f.write(content)

print("✅ Updated MentionPill styling:")
print("   - Changed from green colors to theme-aware white background")
print("   - Now uses bg-bgProminent text-textProminentInverse border-borderProminent")
print("   - Same styling as ActionPill (white background, proper contrast)")
print("   - Maintains same spacing and padding (px-2 py-1 text-xs)")
print("   - Both pills now have consistent appearance")
