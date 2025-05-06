---
title: Smart Extension Recommendation
sidebar_position: 21
sidebar_label: Smart Extension Recommendation
description: Learn how Goose dynamically discovers and manages extensions
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

The Smart Extension Recommendation system in Goose automatically identifies and suggests relevant extensions based on your tasks and needs. This guide explains how to use this feature effectively and understand its capabilities and limitations.

When you request a task, Goose checks its enabled extensions and their tools to determine if it can fulfill the request. If not, it suggests or enables additional extensions as needed. You can also request specific extensions by name.


:::warning
Any extensions enabled dynamically are only enabled for the current session. To keep extensions enabled between sessions, follow the [Using Extensions](/docs/getting-started/using-extensions) guide.
:::

## Automatic Detection

Goose automatically detects when an extension is needed based on your task requirements. Here's an example of how Goose identifies and enables a needed extension during a conversation:

<Tabs groupId="interface">
<TabItem value="ui" label="Goose Desktop" default>

#### Goose Prompt
```plaintext
Find all orders with pending status from our production database
```

#### Goose Output

```plaintext
I'll help you search for available extensions that might help us interact with PostgreSQL databases.

🔍 Search Available Extensions
└─ Output ▼

 I see there's a PostgreSQL extension available. Let me enable it so we can query your database.

🔧 Manage Extensions
└─ action           enable
   extension_name   postgresql

The extension 'postgresql' has been installed successfully

Great! Now I can help you query the database...
```

</TabItem>
<TabItem value="cli" label="Goose CLI">

#### Goose Prompt
```plaintext
Find all orders with pending status from our production database
```

#### Goose Output

```sh
I apologize, but I notice that I don't currently have access to your database. Let me search if there are any database-related extensions available.
─── search_available_extensions | platform ──────────────────────────

I see that there is a "postgresql" extension available. Let me enable it so I can help you query your database.
─── enable_extension | platform ──────────────────────────
extension_name: postgresql


■  Goose would like to enable the following extension, do you approve?
// highlight-start
| ● Yes, for this session 
// highlight-end
| ○ No
```

</TabItem>
</Tabs>

## Direct Request

Goose responds to explicit requests for extensions, allowing users to manually enable specific tools they need. Here's an example of how Goose handles a direct request to enable an extension:

<Tabs groupId="interface">
<TabItem value="ui" label="Goose Desktop" default>

#### Goose Prompt

```plaintext
Use PostgreSQL extension
```

#### Goose Output

```plaintext
I'll help enable the PostgreSQL extension for you.

🔧 Manage Extensions
└─ action           enable
   extension_name   postgresql

The extension 'postgresql' has been installed successfully

The PostgreSQL extension is now ready to use. What would you like to do with it?
```

</TabItem>
<TabItem value="cli" label="Goose CLI">

#### Goose Prompt

```sh
Use the PostgreSQL extension
```

#### Goose Output

```sh
I'll help enable the PostgreSQL extension for you.
─── enable_extension | platform ──────────────────────────
extension_name: postgresql


■  Goose would like to enable the following extension, do you approve?
// highlight-start
| ● Yes, for this session 
// highlight-end
| ○ No
```

</TabItem>
</Tabs>