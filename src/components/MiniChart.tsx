interface MiniChartProps {
  values: number[];
  color: string;
  min?: number;
  max?: number;
}

export function MiniChart({ values, color, min = 0, max = 100 }: MiniChartProps) {
  const chartWidth = 270;
  const chartHeight = 38;
  const range = Math.max(max - min, 1);
  const source = values.length ? values : [min];
  const points = source
    .map((value, index) => {
      const x = source.length === 1 ? chartWidth / 2 : (index / (source.length - 1)) * chartWidth;
      const normalized = Math.min(1, Math.max(0, (value - min) / range));
      const y = chartHeight - 3 - normalized * (chartHeight - 6);
      return `${x.toFixed(2)},${y.toFixed(2)}`;
    })
    .join(" ");

  return (
    <svg
      className="mini-chart"
      aria-hidden="true"
      viewBox={`0 0 ${chartWidth} ${chartHeight}`}
      preserveAspectRatio="none"
    >
      <polyline
        points={points}
        fill="none"
        stroke={color}
        strokeWidth="1.7"
        strokeLinecap="round"
        strokeLinejoin="round"
        vectorEffect="non-scaling-stroke"
      />
    </svg>
  );
}
