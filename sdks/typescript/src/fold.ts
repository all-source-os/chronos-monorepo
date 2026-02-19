import type { Event } from "./types";

/** Interface for folding events into a state. */
export interface EventFolder<S> {
  /** Apply an event to the folder's internal state. Returns true if the event was relevant. */
  apply(event: Event): boolean;
  /** Finalize and return the accumulated state. Returns undefined if no relevant events were applied. */
  finalize(): S | undefined;
}

/** Fold a sequence of events using the given folder. */
export function foldEvents<S>(folder: EventFolder<S>, events: Event[]): S | undefined {
  for (const event of events) {
    folder.apply(event);
  }
  return folder.finalize();
}
