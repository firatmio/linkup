import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

/** Koşullu sınıf birleştirme + Tailwind çakışma çözümü. */
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
