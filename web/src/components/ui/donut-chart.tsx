import { Cell, Pie, PieChart } from "recharts";

import { SLICE_COLORS } from "@/lib/colors";

export { SLICE_COLORS };

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
  const total = slices.reduce((s, x) => s + x.value, 0);

  if (total === 0) return null;

  return (
    <PieChart width={size} height={size} margin={{ top: 0, right: 0, bottom: 0, left: 0 }}>
      <Pie
        data={slices}
        cx={size / 2}
        cy={size / 2}
        innerRadius={innerRadius}
        outerRadius={outerRadius}
        dataKey="value"
        paddingAngle={slices.length > 1 ? 2 : 0}
        startAngle={90}
        endAngle={-270}
        isAnimationActive={false}
        stroke="none"
      >
        {slices.map((slice, i) => (
          <Cell key={i} fill={slice.color} />
        ))}
      </Pie>
    </PieChart>
  );
}
