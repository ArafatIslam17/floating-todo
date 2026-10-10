// Purposeful motion only: row enter/leave (needs height-to-auto) and the alert arrival.
import { gsap } from 'gsap';

export function prefersReducedMotion(): boolean {
  return typeof window !== 'undefined' && !!window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
}

const dur = (s: number) => (prefersReducedMotion() ? 0 : s);

export function rowEnter(el: HTMLElement): void {
  gsap.fromTo(
    el,
    { height: 0, opacity: 0 },
    { height: 'auto', opacity: 1, duration: dur(0.18), ease: 'power2.out', clearProps: 'height,opacity' },
  );
}

/** Collapse a row before the action that removes it; resolves when done. */
export function rowLeave(el: HTMLElement): Promise<void> {
  return new Promise((resolve) => {
    gsap.to(el, { height: 0, opacity: 0, duration: dur(0.18), ease: 'power2.in', onComplete: resolve });
  });
}

/** Undo rowLeave if the action failed. */
export function rowRestore(el: HTMLElement): void {
  gsap.set(el, { clearProps: 'height,opacity' });
}

export function alertArrive(el: HTMLElement): void {
  if (prefersReducedMotion()) return;
  gsap.fromTo(el, { x: -6 }, { x: 0, duration: 0.32, ease: 'elastic.out(1, 0.45)' });
}
