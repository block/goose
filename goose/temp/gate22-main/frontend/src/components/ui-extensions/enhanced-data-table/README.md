# EnhancedDataTable Component Documentation

The EnhancedDataTable is a data table component built on TanStack Table (React Table), providing advanced features such as sorting, filtering, searching, and row selection. This documentation will help you quickly understand and effectively use this component.

## Table of Contents

- [Basic Usage](#basic-usage)
- [Column Definitions](#column-definitions)
- [Search and Filtering](#search-and-filtering)
- [Sorting Functionality](#sorting-functionality)
- [Row Selection](#row-selection)
- [Custom Column Styling and Actions](#custom-column-styling-and-actions)
- [Practical Application Examples](#practical-application-examples)

## Basic Usage

To use the EnhancedDataTable, you need to provide two basic parameters: `columns` (column definitions) and `data` (data source).

```tsx
import { EnhancedDataTable } from "@/components/ui-extensions/enhanced-data-table/data-table";
import { useMyTableColumns } from "./useMyTableColumns";

export function MyTable({ data }) {
  // Use custom hook to get column definitions
  const columns = useMyTableColumns();

  return (
    <EnhancedDataTable
      columns={columns}
      data={data}
      searchBarProps={{ placeholder: "Search..." }}
    />
  );
}
```

## Column Definitions

It's recommended to create column definition hooks in separate files using `createColumnHelper` to get better type support.

### Creating Basic Column Definition Files

```tsx
// useMyTableColumns.tsx
import { useMemo } from "react";
import { createColumnHelper, type ColumnDef } from "@tanstack/react-table";
import { MyDataType } from "@/lib/types/myDataType";

const columnHelper = createColumnHelper<MyDataType>();

export const useMyTableColumns = (): ColumnDef<MyDataType>[] => {
  return useMemo(
    () => [
      columnHelper.accessor("name", {
        header: "Name",
        cell: (info) => info.getValue(),
        enableGlobalFilter: true,
      }),
      columnHelper.accessor("description", {
        header: "Description",
        cell: (info) => info.getValue(),
        enableGlobalFilter: true,
      }),
    ],
    [],
  );
};
```

### Column Properties Explained

- `header`: Column title, can be a string or React component
- `cell`: Cell rendering function
- `enableGlobalFilter`: Whether to include this column in global search
- `enableColumnFilter`: Whether to enable column filtering
- `filterFn`: Filter function type, e.g., "arrIncludes" for array fields
- `id`: Custom column ID, required when using accessor function
- `meta`: Used to set default sorting and other metadata

### Creating Sortable Columns

```tsx
columnHelper.accessor("name", {
  header: ({ column }) => (
    <Button
      variant="ghost"
      onClick={() => column.toggleSorting(column.getIsSorted() === "asc")}
      className="w-full justify-start px-0"
    >
      Name
      <ArrowUpDown className="h-4 w-4" />
    </Button>
  ),
  cell: (info) => info.getValue(),
  enableGlobalFilter: true,
  // Set as default sort column
  meta: {
    defaultSort: true,
    defaultSortDesc: false,
  },
});
```

## Search and Filtering

### Enabling Global Search

Provide the `searchBarProps` parameter and ensure relevant columns have `enableGlobalFilter: true`:

```tsx
<EnhancedDataTable columns={columns} data={data} searchBarProps={{ placeholder: "Search..." }} />
```

### Column Filtering Functionality

EnhancedDataTable provides single‑value dropdown filtering for array‑type columns. :

1. Set `enableColumnFilter: true`
2. Set `filterFn: "arrIncludes"`

#### Enabling Dropdown Filtering for Array Type Columns

```tsx
// Using dropdown filtering for tag arrays
columnHelper.accessor("tags", {
  header: "Tags",
  cell: (info) => (
    <div className="flex flex-wrap gap-2">
      {info.getValue().map((tag) => (
        <span key={tag} className="rounded-md bg-gray-100 px-2 py-1">
          {tag}
        </span>
      ))}
    </div>
  ),
  enableGlobalFilter: true,
  enableColumnFilter: true, // Enable column filtering
  filterFn: "arrIncludes", // Specify array filtering function
});
```

#### Handling Arrays from Non-Standard Data Sources

For array data not in default fields, you can use custom accessors:

```tsx
// Handling nested objects or transforming data
columnHelper.accessor(
  (row) => row.metadata?.categories || [], // Extract array from nested object
  {
    id: "categoriesDisplay", // Must provide unique ID
    header: "Categories",
    cell: (info) => (
      <div className="flex flex-wrap gap-2">
        {info.getValue().map((category) => (
          <span key={category} className="badge">
            {category}
          </span>
        ))}
      </div>
    ),
    enableColumnFilter: true,
    filterFn: "arrIncludes",
  },
);
```

#### Automatic Filter UI Generation

When the following conditions are met, the table will automatically generate dropdown filter menus for columns:

1. Column has `enableColumnFilter: true`
2. Column uses `filterFn: "arrIncludes"`

Filters will be displayed in the top toolbar of the table, allowing users to select multiple values for filtering.

## Sorting Functionality

### Setting Default Sorting

You can set default sorting in two ways:

1. Using the `meta` property in column definition:

```tsx
columnHelper.accessor("createdAt", {
  header: "Created At",
  cell: (info) => new Date(info.getValue()).toLocaleString(),
  id: "createdAt",
  meta: {
    defaultSort: true,
    defaultSortDesc: true, // Default descending order
  },
});
```

2. Through the component's `defaultSorting` property:

```tsx
<EnhancedDataTable
  columns={columns}
  data={data}
  defaultSorting={[
    {
      id: "createdAt",
      desc: true,
    },
  ]}
/>
```

## Row Selection

To enable row selection functionality, you need to provide `rowSelectionProps`:

```tsx
const [rowSelection, setRowSelection] = useState({});

<EnhancedDataTable
  columns={columns}
  data={data}
  rowSelectionProps={{
    rowSelection,
    onRowSelectionChange: setRowSelection,
    getRowId: (row) => row.id,
  }}
/>;
```

This will add a selection checkbox column to the left of the table, allowing users to select multiple rows of data.

## Custom Column Styling and Actions

### Image and Text Combination Column

```tsx
columnHelper.accessor("app_name", {
  header: "App Name",
  cell: (info) => {
    const appName = info.getValue();
    return (
      <div className="flex items-center gap-3">
        <div className="relative h-5 w-5 shrink-0 overflow-hidden">
          {appsMap[appName]?.logo && (
            <Image
              src={appsMap[appName]?.logo || ""}
              alt={`${appName} logo`}
              fill
              className="object-contain"
            />
          )}
        </div>
        <IdDisplay id={appName} dim={false} />
      </div>
    );
  },
  enableGlobalFilter: true,
});
```

### Tag Rendering

```tsx
columnHelper.accessor("tags", {
  header: "Tags",
  cell: (info) => (
    <div className="flex flex-wrap gap-2 overflow-hidden">
      {(info.getValue() || []).map((tag) => (
        <span
          key={tag}
          className="rounded-md border border-gray-200 bg-gray-100 px-3 py-1 text-sm font-medium text-gray-600"
        >
          {tag}
        </span>
      ))}
    </div>
  ),
  enableGlobalFilter: true,
  enableColumnFilter: true,
  filterFn: "arrIncludes",
});
```

### Custom Action Buttons Column

```tsx
columnHelper.accessor(
  (row) => row, // Get entire row data
  {
    id: "actions",
    header: "",
    cell: (info) => {
      const item = info.getValue();
      return (
        <div className="flex space-x-2">
          <Link href={`/items/${item.id}`}>
            <Button variant="outline" size="sm">
              Open
            </Button>
          </Link>
          <Button variant="ghost" size="sm" className="text-red-600">
            <GoTrash />
          </Button>
        </div>
      );
    },
    enableGlobalFilter: false,
  },
);
```

### Editable Cells

```tsx
columnHelper.accessor("name", {
  header: "Name",
  cell: (ctx) => {
    const item = ctx.row.original;
    return (
      <EditableCell
        initialValue={item.name}
        fieldName="name"
        itemId={item.id}
        onUpdate={handleUpdate}
      />
    );
  },
  enableGlobalFilter: true,
});
```

## Pagination

EnhancedDataTable supports client-side pagination using TanStack Table’s built-in `getPaginationRowModel()`.

To enable pagination, pass the `paginationOptions` prop:

```tsx
<EnhancedDataTable
  columns={columns}
  data={data}
  paginationOptions={{
    initialPageIndex: 0, // Default to first page (0-based)
    initialPageSize: 10, // Number of entries per page
  }}
/>
```

## Practical Application Examples

### MCP Configuration Table (with Icons, Category Filtering, and Toggle Switch)

```tsx
// In page component
import { EnhancedDataTable } from "@/components/ui-extensions/enhanced-data-table/data-table";
import { useAppConfigsTableColumns } from "./useAppConfigsTableColumns";

export function AppConfigsPage() {
  const [appConfigs, setAppConfigs] = useState<AppConfig[]>([]);
  // ...other states and logic

  const appConfigsColumns = useAppConfigsTableColumns({
    linkedAccountsCountMap,
    appsMap,
    updateAppConfigs: loadAllData,
  });

  return (
    <div>
      <EnhancedDataTable
        data={appConfigs}
        columns={appConfigsColumns}
        searchBarProps={{ placeholder: "Search apps..." }}
      />
    </div>
  );
}
```

### Agents Table (with Editable Cells and Custom Actions)

```tsx
// In page component
import { EnhancedDataTable } from "@/components/ui-extensions/enhanced-data-table/data-table";
import { useAgentsTableColumns } from "./useAgentsTableColumns";

export function AgentsPage() {
  const [agents, setAgents] = useState<Agent[]>([]);
  // ...other states and logic

  const columns = useAgentsTableColumns(
    projectId,
    handleDeleteAgent,
    reloadAgents,
    handleInstructionsSave,
  );

  return (
    <div>
      <EnhancedDataTable
        data={agents}
        columns={columns}
        searchBarProps={{ placeholder: "Search agents..." }}
      />
    </div>
  );
}
```
