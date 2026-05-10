export function Stat({
  label,
  value,
  hint,
}: {
  label: string;
  value: string;
  hint?: string;
}) {
  return (
    <div className="rounded-md border border-navy-700/60 bg-navy-700/30 px-5 py-4">
      <div className="text-xs uppercase tracking-widest text-navy-200">{label}</div>
      <div className="mt-1 font-serif text-3xl tabular-nums">{value}</div>
      {hint && <div className="mt-1 text-xs text-navy-200/60">{hint}</div>}
    </div>
  );
}
