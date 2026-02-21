export type LoadingStatus = 'loading' | 'success' | 'error';
export default function Dot({
  size,
  loadingStatus,
}: {
  size: number;
  loadingStatus: LoadingStatus;
}) {
  const backgroundColorClasses: Record<LoadingStatus, string> = {
    loading: 'bg-background-accent',
    success: 'bg-background-success',
    error: 'bg-background-danger',
  };

  return (
    <div className="flex items-center justify-center">
      <div
        className={`rounded-full ${backgroundColorClasses[loadingStatus] || 'bg-icon-extra-subtle'}`}
        style={{
          width: `${size * 2}px`,
          height: `${size * 2}px`,
        }}
      />
    </div>
  );
}
