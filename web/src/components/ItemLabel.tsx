type ItemLabelProps = {
  primary: string;
  secondary: string | undefined;
};

export function ItemLabel({ primary, secondary }: ItemLabelProps) {
  return (
    <div className="flex min-w-0 max-w-full flex-col">
      <span className="font-bold">{primary}</span>
      <span
        className="block max-w-full truncate text-[10px] text-muted-foreground"
        title={secondary}
      >
        {secondary}
      </span>
    </div>
  );
}
