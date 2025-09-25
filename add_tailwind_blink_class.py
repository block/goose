import re

# Read the main.css file
with open('ui/desktop/src/styles/main.css', 'r') as f:
    content = f.read()

print("Adding Tailwind animate-blink utility class...")

# Find the @theme inline section and add the animation
theme_inline_section = '''@theme inline {
  --ease-g2: cubic-bezier(0.55, 0, 1, 0.45);'''

new_theme_inline_section = '''@theme inline {
  --ease-g2: cubic-bezier(0.55, 0, 1, 0.45);
  
  --animate-blink: blink 1s step-end infinite;'''

content = content.replace(theme_inline_section, new_theme_inline_section)

# Write the modified content back
with open('ui/desktop/src/styles/main.css', 'w') as f:
    f.write(content)

print("✅ Added animate-blink utility class to Tailwind theme")
print("   - Uses the blink keyframe with 1s duration")
print("   - step-end timing for sharp on/off transitions")
print("   - infinite loop for continuous blinking")
