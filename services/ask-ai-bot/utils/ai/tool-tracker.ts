export class ToolTracker {
  private searchCalls: number = 0;
  private searchResults: Set<string> = new Set();
  private viewedPaths: Set<string> = new Set();

  recordSearchCall(results: string[]): void {
    this.searchCalls++;
    results.forEach((result) => this.searchResults.add(result));
  }

  recordViewCall(filePaths: string | string[]): void {
    const paths = Array.isArray(filePaths) ? filePaths : [filePaths];
    paths.forEach((path) => this.viewedPaths.add(path));
  }

  getSummary(): string {
    const parts: string[] = [];

    if (this.searchCalls > 0) {
      const resultCount = this.searchResults.size;
      const timesText = this.searchCalls === 1 ? "time" : "times";
      const resultsText = resultCount === 1 ? "result" : "results";
      parts.push(
        `searched ${this.searchCalls} ${timesText} with ${resultCount} ${resultsText}`,
      );
    }

    if (this.viewedPaths.size > 0) {
      const pageCount = this.viewedPaths.size;
      const pagesText = pageCount === 1 ? "page" : "pages";
      parts.push(`viewed ${pageCount} ${pagesText}`);
    }

    if (parts.length === 0) return "";

    const firstPart = parts[0].charAt(0).toUpperCase() + parts[0].slice(1);
    return parts.length === 1 ? firstPart : firstPart + ", " + parts[1];
  }
}
