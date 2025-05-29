export function LoadingPlaceholder() {
  return (
    <div className="space-y-2">
      {[...Array(2)].map((_, index) => (
        <div
          key={index}
          className="h-4 bg-gradient-to-r from-[#ffffff]/30 via-[#7A7EFB]/40 to-[#ffffff]/30
                   animate-shimmer-pulse bg-[length:200%_100%] rounded-full"
          style={{
            width: `${Math.floor(Math.random() * (100 - 70 + 1) + 70)}%`,
          }}
        />
      ))}
    </div>
  );
}
