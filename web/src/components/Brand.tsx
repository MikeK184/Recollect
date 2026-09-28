export function Brand({ compact = false }: { compact?: boolean }) {
  return (
    <div className="brand" role="img" aria-label="Recollect">
      <img src="/brand/recollect-symbol.svg" width={36} height={36} alt="" />
      {!compact && (
        <span>
          Recollect<span className="brand-dot">.</span>
        </span>
      )}
    </div>
  );
}
