#!/usr/bin/env python3
"""
Decode base64 training data for the recipe scanner
This script will be used inside the Docker container to decode GitHub secrets
"""

import json
import base64
import os
import tempfile
from pathlib import Path

def decode_training_data():
    """
    Decode all available training data from environment variables
    Returns a dictionary with risk levels and their decoded recipes
    """
    training_data = {}
    
    # Check for each risk level
    for risk_level in ["LOW", "MEDIUM", "HIGH", "EXTREME"]:
        env_var = f"TRAINING_DATA_{risk_level}"
        encoded_data = os.environ.get(env_var)
        
        if encoded_data:
            try:
                # Decode the base64 outer layer
                json_data = base64.b64decode(encoded_data).decode('utf-8')
                
                # Parse the JSON
                parsed_data = json.loads(json_data)
                
                # Decode each recipe's content
                for recipe in parsed_data.get('recipes', []):
                    recipe_content = base64.b64decode(recipe['content_base64']).decode('utf-8')
                    recipe['content'] = recipe_content
                    # Keep the base64 version for reference but don't need it for analysis
                
                training_data[risk_level.lower()] = parsed_data
                print(f"✅ Decoded {len(parsed_data['recipes'])} {risk_level.lower()} risk recipes")
                
            except Exception as e:
                print(f"❌ Error decoding {env_var}: {e}")
    
    return training_data

def write_training_files(training_data, output_dir="/tmp/training"):
    """
    Write decoded training files to disk for Goose to analyze
    """
    output_path = Path(output_dir)
    output_path.mkdir(exist_ok=True)
    
    # Write a summary file for Goose
    summary = {
        "training_summary": "Recipe security training data",
        "risk_levels": {},
        "total_recipes": 0
    }
    
    for risk_level, data in training_data.items():
        risk_dir = output_path / risk_level
        risk_dir.mkdir(exist_ok=True)
        
        recipes_info = []
        
        for recipe in data.get('recipes', []):
            # Write the recipe file
            recipe_file = risk_dir / recipe['filename']
            with open(recipe_file, 'w') as f:
                f.write(recipe['content'])
            
            # Write the training notes
            notes_file = risk_dir / f"{recipe['filename']}.notes.txt"
            with open(notes_file, 'w') as f:
                f.write(f"Risk Level: {risk_level.upper()}\n")
                f.write(f"Filename: {recipe['filename']}\n")
                f.write(f"Size: {recipe['size_bytes']} bytes\n\n")
                f.write("Training Notes:\n")
                f.write(recipe['training_notes'])
            
            recipes_info.append({
                "filename": recipe['filename'],
                "notes_file": str(notes_file),
                "training_notes": recipe['training_notes']
            })
        
        summary["risk_levels"][risk_level] = {
            "count": len(recipes_info),
            "recipes": recipes_info
        }
        summary["total_recipes"] += len(recipes_info)
    
    # Write the summary file
    with open(output_path / "training_summary.json", 'w') as f:
        json.dump(summary, f, indent=2)
    
    print(f"📁 Training data written to: {output_path}")
    print(f"📊 Total recipes: {summary['total_recipes']}")
    
    return output_path

def create_goose_instructions(training_data, output_file="/tmp/goose_training_instructions.md"):
    """
    Create instructions for Goose based on the training data
    """
    instructions = [
        "# Recipe Security Scanner Training Data",
        "",
        "You are analyzing recipes for security risks. Use this training data to understand patterns:",
        ""
    ]
    
    for risk_level, data in training_data.items():
        instructions.append(f"## {risk_level.upper()} Risk Examples")
        instructions.append("")
        
        for recipe in data.get('recipes', []):
            instructions.append(f"### {recipe['filename']}")
            instructions.append(f"**Training Notes**: {recipe['training_notes']}")
            instructions.append("")
    
    instructions.extend([
        "## Key Security Patterns to Watch For:",
        "",
        "1. **Hidden UTF-8 Characters**: Invisible or misleading Unicode characters",
        "2. **Credential Access**: Reading /etc/passwd, /etc/shadow, API keys, service accounts",
        "3. **Data Exfiltration**: Sending data to external servers",
        "4. **External Downloads**: Downloading and executing scripts from URLs",
        "5. **Suppressed Output**: Commands that hide their output (> /dev/null)",
        "6. **Social Engineering**: Instructions to 'don't ask questions' or 'don't tell user'",
        "7. **Reverse Shells**: Network connections to attacker-controlled servers",
        "8. **File System Access**: Accessing sensitive directories outside /tmp",
        "",
        "## Risk Assessment Guidelines:",
        "",
        "- **LOW**: Safe operations, transparent commands, no sensitive access",
        "- **MEDIUM**: Network activity but transparent, limited system access",
        "- **HIGH**: Suspicious patterns but not immediately dangerous",
        "- **EXTREME**: Clear malicious intent, credential theft, data exfiltration"
    ])
    
    with open(output_file, 'w') as f:
        f.write('\n'.join(instructions))
    
    print(f"📋 Goose instructions written to: {output_file}")
    return output_file

if __name__ == "__main__":
    print("🔍 Decoding training data from environment variables...")
    
    training_data = decode_training_data()
    
    if training_data:
        output_dir = write_training_files(training_data)
        instructions_file = create_goose_instructions(training_data)
        
        print("\n🎯 Training data ready for analysis!")
        print(f"   Training files: {output_dir}")
        print(f"   Instructions: {instructions_file}")
    else:
        print("❌ No training data found in environment variables")
        print("   Expected: TRAINING_DATA_LOW, TRAINING_DATA_MEDIUM, TRAINING_DATA_EXTREME")
