export type HistoryEntry<T> = T;

export function createHistory<T>(initial: T) {
  let past: T[] = [];
  let present = initial;
  let future: T[] = [];

  return {
    get(): T {
      return present;
    },
    set(next: T, record = true) {
      if (record) {
        past = [...past, present].slice(-80);
        future = [];
      }
      present = next;
      return present;
    },
    replace(next: T) {
      present = next;
      return present;
    },
    undo(): T | null {
      if (!past.length) return null;
      future = [present, ...future].slice(0, 80);
      present = past[past.length - 1];
      past = past.slice(0, -1);
      return present;
    },
    redo(): T | null {
      if (!future.length) return null;
      past = [...past, present].slice(-80);
      present = future[0];
      future = future.slice(1);
      return present;
    },
    canUndo: () => past.length > 0,
    canRedo: () => future.length > 0,
    reset(next: T) {
      past = [];
      future = [];
      present = next;
    }
  };
}
