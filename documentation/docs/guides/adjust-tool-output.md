---
sidebar_position: 11
title: Adjusting Tool Output Verbosity
sidebar_label: Adjust Tool Output
---

When working with the Goose CLI, you can control the verbosity of tool output.

To adjust the tool output, run:

```sh
goose configure
```

Then choose `Adjust Tool Output`

```sh
┌   goose-configure 
│
◆  What would you like to configure?
│  ○ Configure Providers 
│  ○ Add Extension 
│  ○ Toggle Extensions 
│  ○ Remove Extension
// highlight-next-line
│  ● Adjust Tool Output (Show more or less tool output)
└  
```

Next, choose one of the available modes:

```sh
┌   goose-configure 
│
◇  What would you like to configure?
│  Adjust Tool Output 
│
// highlight-start
◆  Which tool output would you like to show?
│  ○ High Importance 
│  ○ Medium Importance 
│  ○ All 
// highlight-end
└  
```

- **High Importance**
    - Shows only the most important tool outputs
    - Most minimal output level

- **Medium Importance**
    - Shows medium and high importance outputs
    - Example: Results of file-write operations

- **All**
    - Shows all tool outputs
    - Example: Shell command outputs
    - Most verbose level
