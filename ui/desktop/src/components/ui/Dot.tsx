export type LoadingStatus = 'loading' | 'success' | 'error';
export default function Dot({
  size,
  loadingStatus,
}: {
  size: number;
  loadingStatus: LoadingStatus;
}) {
  const backgroundColor =
    {
      loading: '#2693FF',
      success: 'var(--icon-extra-subtle)',
      error: '#CC0023',
    }[loadingStatus] ?? 'var(--icon-extra-subtle)';

  return (
    <div
      className={`w-${size} h-${size} rounded-full`}
      style={{
        backgroundColor: backgroundColor,
      }}
    />
  );
}
