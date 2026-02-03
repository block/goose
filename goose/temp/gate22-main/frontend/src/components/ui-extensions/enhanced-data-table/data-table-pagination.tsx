"use client";
import { Table } from "@tanstack/react-table";
import {
  Pagination,
  PaginationContent,
  PaginationItem,
  PaginationLink,
} from "@/components/ui/pagination";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { ChevronLeft, ChevronRight } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

interface DataTablePaginationProps<TData> {
  table: Table<TData>;
  totalCount?: number;
}
type PageItem = number | "ellipsis";

// `current0` indicates a 0-based page index (internal state), converted to `currentPage` (1-based) for user-facing pagination.
function getPageItems(current0: number, total: number): PageItem[] {
  if (total <= 0) return [];
  const current = current0 + 1;
  // If the total number of pages is less than or equal to 7, display all page numbers
  if (total <= 7) return Array.from({ length: total }, (_, i) => i + 1);

  // Case 2: At the first 3 pages
  if (current <= 3) return [1, 2, 3, 4, 5, "ellipsis", total];

  // Case 3: At the last 3 pages
  if (current >= total - 2) {
    // let the tail interval start dynamically: total - 4, but not less than 2
    const tailStart = Math.max(total - 4, 2);
    const tail = Array.from({ length: total - tailStart }, (_, i) => tailStart + i);
    return [1, "ellipsis", ...tail, total];
  }

  // Case 4: In the middle position
  return [1, "ellipsis", current - 1, current, current + 1, "ellipsis", total];
}

export function DataTablePagination<TData>({ table, totalCount }: DataTablePaginationProps<TData>) {
  const { pageIndex, pageSize } = table.getState().pagination;
  const pageCount = table.getPageCount();
  const rowsThisPage = table.getPaginationRowModel().rows.length;

  /* Row number range 41 – 50 / 2167 */
  const startRow = pageIndex * pageSize + 1;
  const endRow = startRow + rowsThisPage - 1;

  const [pageInput, setPageInput] = useState(pageIndex + 1);
  useEffect(() => setPageInput(pageIndex + 1), [pageIndex]);

  const jumpToPage = () => {
    // Handle empty or non-numeric input
    if (isNaN(pageInput) || pageInput === null) {
      setPageInput(pageIndex + 1);
      return;
    }

    table.setPageIndex(Math.min(Math.max(pageInput, 1), pageCount) - 1);
  };

  /* Page number button array */
  const pageItems = useMemo(() => getPageItems(pageIndex, pageCount), [pageIndex, pageCount]);

  // Handle empty table
  if (table.getRowCount() === 0) {
    return <div className="ml-4 py-2 text-sm text-muted-foreground">No results to display</div>;
  }

  return (
    <div className="flex flex-nowrap items-center justify-between px-3 py-1">
      {/* Left: Total row number information */}
      <div className="flex items-center gap-4">
        <div className="text-sm font-medium">
          {startRow} – {endRow} / {totalCount ?? table.getRowCount()}
        </div>
        <div className="flex items-center gap-2">
          <span className="text-sm text-muted-foreground">Rows per page</span>
          <Select
            value={pageSize.toString()}
            onValueChange={(value) => {
              table.setPageSize(Number(value));
            }}
          >
            <SelectTrigger className="h-8 w-[70px]">
              <SelectValue placeholder={pageSize} />
            </SelectTrigger>
            <SelectContent side="top">
              {[10, 15, 20, 30, 50].map((size) => (
                <SelectItem key={size} value={size.toString()}>
                  {size}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      </div>

      {/* Right: Pagination buttons + input box */}
      <div className="ml-auto w-max cursor-pointer">
        <Pagination>
          <PaginationContent className="flex items-center">
            {/* Prev */}
            <PaginationItem>
              <PaginationLink
                aria-label="Previous page"
                className={
                  !table.getCanPreviousPage() ? "pointer-events-none opacity-50" : undefined
                }
                onClick={() => table.previousPage()}
              >
                <ChevronLeft className="h-4 w-4" />
              </PaginationLink>
            </PaginationItem>

            {/* Dynamic page number group */}
            {pageItems.map((item, i) => (
              <PaginationItem key={i}>
                {item === "ellipsis" ? (
                  <span
                    className="px-2 text-sm text-muted-foreground select-none"
                    aria-hidden="true"
                  >
                    …
                  </span>
                ) : (
                  <PaginationLink
                    isActive={item - 1 === pageIndex}
                    aria-label={`Page ${item}`}
                    aria-current={item - 1 === pageIndex ? "page" : undefined}
                    onClick={() => table.setPageIndex(item - 1)}
                  >
                    {item}
                  </PaginationLink>
                )}
              </PaginationItem>
            ))}

            {/* Next */}
            <PaginationItem>
              <PaginationLink
                aria-label="Next page"
                className={!table.getCanNextPage() ? "pointer-events-none opacity-50" : undefined}
                onClick={() => table.nextPage()}
              >
                <ChevronRight className="h-4 w-4" />
              </PaginationLink>
            </PaginationItem>

            {/* Jump page input box */}
            <PaginationItem className="ml-2 flex items-center gap-1">
              <Input
                type="number"
                inputMode="numeric"
                pattern="[0-9]*"
                min={1}
                max={pageCount}
                value={pageInput}
                onChange={(e) => setPageInput(Number(e.target.value))}
                onKeyDown={(e) => e.key === "Enter" && jumpToPage()}
                // Hide the arrow of the input box  [appearance:textfield] [&::-webkit-outer-spin-button]:appearance-none [&::-webkit-inner-spin-button]:appearance-none
                className="h-7 w-14 [appearance:textfield] text-center [&::-webkit-inner-spin-button]:appearance-none [&::-webkit-outer-spin-button]:appearance-none"
                aria-label="To Page"
              />
              <Button
                size="sm"
                className="h-7"
                disabled={isNaN(pageInput) || pageInput < 1 || pageInput > pageCount}
                onClick={jumpToPage}
                aria-label="Go"
              >
                Go
              </Button>
            </PaginationItem>
          </PaginationContent>
        </Pagination>
      </div>
    </div>
  );
}
