import { cn } from "@/lib/utils";

type ItemLabelProps = {
  primary: string;
  secondary: string | undefined;
  href?: string;
  target?: string;
  className?: string;
};

export function ItemLabel({ primary, secondary, href, target, className }: ItemLabelProps) {
  const content = (
    <div className={cn("flex min-w-0 max-w-full flex-col", className)}>
      <span className="font-bold">{primary}</span>
      <span
        className="block max-w-full truncate text-[10px] text-muted-foreground"
        title={secondary}
      >
        {secondary}
      </span>
    </div>
  );

  if (href) {
    return (
      <a
        className="group/item-label"
        href={href}
        onClick={(e) => e.stopPropagation()}
        rel={target === "_blank" ? "noopener noreferrer" : undefined}
        target={target}
      >
        <div className="flex min-w-0 max-w-full flex-col">
          <span className="font-bold group-hover/item-label:underline">{primary}</span>
          <span
            className="block max-w-full truncate text-[10px] text-muted-foreground"
            title={secondary}
          >
            {secondary}
          </span>
        </div>
      </a>
    );
  }

  return content;
}
