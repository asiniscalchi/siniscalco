type AssetLabelProps = {
  symbol: string;
  name: string | undefined;
};

export function AssetLabel({ symbol, name }: AssetLabelProps) {
  return (
    <div className="flex flex-col">
      <span className="font-bold">{symbol}</span>
      <span className="truncate text-[10px] text-muted-foreground">{name}</span>
    </div>
  );
}
