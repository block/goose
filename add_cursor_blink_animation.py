import re

# Read the main.css file
with open('ui/desktop/src/styles/main.css', 'r') as f:
    content = f.read()

print("Adding cursor blink animation to main.css...")

# Find the animations section and add the blink animation
animations_section = '''/* animations */
@keyframes appear {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}'''

new_animations_section = '''/* animations */
@keyframes appear {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}

@keyframes blink {
  0%, 50% {
    opacity: 1;
  }
  51%, 100% {
    opacity: 0;
  }
}'''

content = content.replace(animations_section, new_animations_section)

# Write the modified content back
with open('ui/desktop/src/styles/main.css', 'w') as f:
    f.write(content)

print("✅ Added cursor blink animation to main.css")
print("   - @keyframes blink: 0-50% visible, 51-100% hidden")
print("   - Creates a regular text cursor blinking effect")
print("   - Can be used with 'animate-blink' class")
