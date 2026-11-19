import type { ComponentProps } from "react";
import * as SwitchPrimitive from "@radix-ui/react-switch";
import { cva, type VariantProps } from "class-variance-authority";
// Project-local shadcn/ui primitives with ClippO's widget sizes and palette.
const button = cva(
  "button inline-flex shrink-0 items-center justify-center whitespace-nowrap disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg]:shrink-0",
  {
    variants: {
      variant: {
        default: "button-main",
        secondary: "button-secondary",
        ghost: "button-ghost",
        icon: "button-icon",
      },
    },
    defaultVariants: { variant: "default" },
  },
);
export function Button({
  variant = "default",
  className = "",
  ...props
}: ComponentProps<"button"> & VariantProps<typeof button>) {
  return (
    <button
      data-slot="button"
      data-variant={variant}
      className={button({ variant, className })}
      {...props}
    />
  );
}
export function Toggle({
  checked,
  onChange,
  label,
  disabled = false,
}: {
  checked: boolean;
  onChange: (v: boolean) => void;
  label: string;
  disabled?: boolean;
}) {
  return (
    <SwitchPrimitive.Root
      data-slot="switch"
      className="switch inline-flex shrink-0 items-center rounded-full disabled:cursor-not-allowed disabled:opacity-50"
      checked={checked}
      onCheckedChange={onChange}
      aria-label={label}
      disabled={disabled}
    >
      <SwitchPrimitive.Thumb
        data-slot="switch-thumb"
        className="switch-thumb pointer-events-none block rounded-full"
      />
    </SwitchPrimitive.Root>
  );
}
