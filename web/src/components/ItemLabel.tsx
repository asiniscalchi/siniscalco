type ItemLabelProps = {
  primary: string;
  secondary: string | undefined;
};

export function ItemLabel({ primary, secondary }: ItemLabelProps) {
  return (
    <div className="flex flex-col">
      <span className="font-bold">{primary}</span>
      <span className="truncate text-[10px] text-muted-foreground">{secondary}</span>
    </div>
  );
}
