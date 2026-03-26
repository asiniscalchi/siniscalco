import { SLICE_COLORS } from "@/lib/colors";

export { SLICE_COLORS };

function polarToCartesian(r: number, angle: number, cx: number, cy: number) {
  return {
    x: cx + r * Math.cos(angle),
    y: cy + r * Math.sin(angle),
  };
}

function arcPath(
  startAngle: number,
  endAngle: number,
  cx: number,
  cy: number,
  innerRadius: number,
  outerRadius: number,
): string {
  const os = polarToCartesian(outerRadius, startAngle, cx, cy);
  const oe = polarToCartesian(outerRadius, endAngle, cx, cy);
  const is_ = polarToCartesian(innerRadius, startAngle, cx, cy);
  const ie = polarToCartesian(innerRadius, endAngle, cx, cy);
  const large = endAngle - startAngle > Math.PI ? 1 : 0;

  return [
    `M ${os.x} ${os.y}`,
    `A ${outerRadius} ${outerRadius} 0 ${large} 1 ${oe.x} ${oe.y}`,
    `L ${ie.x} ${ie.y}`,
    `A ${innerRadius} ${innerRadius} 0 ${large} 0 ${is_.x} ${is_.y}`,
    `Z`,
  ].join(" ");
}

export function DonutChart({
  slices,
  size = 180,
  innerRadius = 52,
  outerRadius = 82,
}: {
  slices: { value: number; color: string }[];
  size?: number;
  innerRadius?: number;
  outerRadius?: number;
}) {
  const cx = size / 2;
  const cy = size / 2;
  const total = slices.reduce((s, x) => s + x.value, 0);

  if (total === 0) return null;

  const hasGap = slices.length > 1;
  const GAP = 0.03;
  const startAngle = -Math.PI / 2;

  const slicesWithAngles = slices.reduce<
    { slice: { value: number; color: string }; start: number; end: number }[]
  >((acc, slice) => {
    const span = Math.min(
      (slice.value / total) * 2 * Math.PI,
      2 * Math.PI - 0.0001,
    );
    const prevEnd = acc.length > 0 ? acc[acc.length - 1].end : startAngle;
    const currentStart = prevEnd + (hasGap ? GAP / 2 : 0);
    const currentEnd = currentStart + span - (hasGap ? GAP / 2 : 0);
    return [...acc, { slice, start: currentStart, end: currentEnd }];
  }, []);

  return (
    <svg width={size} height={size} aria-hidden="true">
      {slicesWithAngles.map(({ slice, start, end }, i) => (
        <path
          key={i}
          d={arcPath(start, end, cx, cy, innerRadius, outerRadius)}
          fill={slice.color}
        />
      ))}
    </svg>
  );
}
