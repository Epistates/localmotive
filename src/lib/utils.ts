import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";
import type { Snippet } from "svelte";
import type { HTMLAttributes } from "svelte/elements";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

// shadcn-svelte component utility types

/** Removes the `children` prop from a type. */
export type WithoutChildren<T> = T extends { children?: unknown }
  ? Omit<T, "children">
  : T;

/** Removes the `child` prop from a type. */
export type WithoutChild<T> = T extends { child?: unknown }
  ? Omit<T, "child">
  : T;

/** Removes both `children` and `child` props. */
export type WithoutChildrenOrChild<T> = WithoutChildren<WithoutChild<T>>;

/** Adds an `el` bindable element ref to a type. */
export type WithElementRef<
  T,
  El extends HTMLElement = HTMLElement,
> = T & {
  ref?: El | null;
};
