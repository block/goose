export function extractCommand(link: string): string {
  const url = new URL(link);
  const cmd = url.searchParams.get('cmd') || 'Unknown Command';
  const args = url.searchParams.getAll('arg').map(decodeURIComponent);

  // Combine the command and its arguments into a reviewable format
  return `${cmd} ${args.join(' ')}`.trim();
}

export function extractExtensionName(link: string): string {
  const url = new URL(link);
  const name = url.searchParams.get('name');
  return name ? decodeURIComponent(name) : 'Unknown Extension';
}
