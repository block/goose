# Building Your First Game

This tutorial provides a framework for guiding a user through building their first simple game. The default suggestion is a Flappy Bird clone using Python and Pygame, but you should adapt based on user preferences and experience.

## Initial Discussion

Start by understanding the user's context and preferences:

1. Ask about their programming experience:
   - Are they completely new to programming?
   - Do they have experience with specific languages?
   - Have they done any game development before?

2. Discuss game preferences:
   - Suggest simple starter games they could build:
     * Flappy Bird (default) - focuses on physics and collision
     * Snake - focuses on grid-based movement and growth mechanics
     * Pong - focuses on two-player interaction and ball physics
     * Breakout - focuses on collision and scoring mechanics
   - Let them suggest alternatives if they have something specific in mind
   - Help them understand the complexity of their choice and adjust if needed

3. Choose technology stack:
   - Default suggestion: Python + Pygame (beginner-friendly, cross-platform)
   - Alternative suggestions based on user experience:
     * JavaScript + Canvas (web-based, good for sharing)
     * Lua + LÖVE (lightweight, good for learning)
     * C# + MonoGame (good for Windows users/Unity transition)
   - Consider factors like:
     * Installation complexity on their OS
     * Learning curve
     * Available learning resources
     * Their future goals in programming

## Environment Setup

Guide them through setting up their development environment:

1. Version Control:
   - Help them install and configure git
   - Explain basic version control concepts if they're new
   - Create initial repository

2. Programming Language:
   - Walk through installation for their chosen language
   - Verify installation (help troubleshoot if needed)
   - Explain how to run code in their environment

3. Dependency Management:
   - Explain why dependency isolation is important
   - For Python: Guide through virtualenv setup:
     ```bash
     python -m venv env
     source env/bin/activate  # or env\Scripts\activate on Windows
     ```
   - Similar isolation for other languages:
     * Node: package.json
     * Rust: Cargo.toml
     * etc.

4. Game Framework:
   - Install and verify chosen framework
   - Create minimal test program
   - Ensure they can run it successfully

## Project Structure

Help them set up a maintainable project:

1. Discuss project organization:
   - File structure
   - Code organization
   - Asset management (if needed)

2. Create initial files:
   - Main game file
   - Configuration (if needed)
   - Asset directories (if needed)

3. Set up version control:
   - .gitignore for their stack
   - Initial commit
   - Explain commit strategy

## Core Game Loop

Guide them through building the basic game structure:

1. Window Setup:
   - Creating a game window
   - Setting up the game loop
   - Handling basic events (exit, restart)

2. Game State:
   - Define core game objects
   - Set up state management
   - Create update/draw separation

## Game Mechanics

Break down implementation into manageable pieces:

1. Player Interaction:
   - Input handling
   - Basic movement
   - Test and refine "feel"

2. Core Mechanics:
   - Main game elements (varies by game type)
   - Basic collision detection
   - Score tracking

3. Progressive Enhancement:
   - Additional features
   - Polish and refinement
   - Bug fixes

## Testing and Refinement

Help them improve their game:

1. Playability:
   - Test core mechanics
   - Adjust difficulty
   - Refine controls

2. Code Quality:
   - Identify repetitive code
   - Suggest improvements
   - Explain benefits

## Extensions and Learning

Suggest next steps based on their interests:

1. Possible Enhancements:
   - Graphics improvements
   - Sound effects
   - Additional features
   - Menu systems

2. Learning Opportunities:
   - Code structure improvements
   - Performance optimization
   - Advanced features
   - Related topics to explore

## Notes for Agent

- Adapt the pace based on user understanding
- Provide more detailed explanations when needed
- Suggest breaks at good stopping points
- Celebrate small victories and progress
- Be ready to troubleshoot common issues:
  * Installation problems
  * Framework-specific errors
  * Game logic bugs
  * Performance issues

Remember to:
- Check understanding frequently
- Provide context for new concepts
- Relate to user's existing knowledge
- Be patient with debugging
- Encourage experimentation
- Maintain a positive learning environment

Default Implementation:
- If user has no strong preferences, guide them through:
  * Python + Pygame
  * Flappy Bird clone
  * virtualenv for dependency management
  * git for version control
- This combination provides:
  * Minimal setup complexity
  * Quick visible progress
  * Clear next steps
  * Manageable scope