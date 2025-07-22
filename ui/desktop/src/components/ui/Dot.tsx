export type LoadingStatus = 'loading' | 'success' | 'error';
export default function Dot({
  size,
  loadingStatus,
}: {
  size: number;
  loadingStatus: LoadingStatus;
}) {
  const backgroundColorClasses = {
    loading: 'bg-blue-500',
    success: 'bg-green-600',
    error: 'bg-red-600',
  };

  return (
    <div className={`${loadingStatus === 'loading' ? '' : ''} flex items-center justify-center`}>
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
