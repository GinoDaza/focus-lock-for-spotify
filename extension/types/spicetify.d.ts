// Minimal typings for the parts of the Spicetify API that Tunelock uses.
// Spotify-internal APIs change between client versions; keep this small and
// access them only through src/compat.ts.

declare namespace Spicetify {
  interface Location {
    pathname: string;
  }

  interface History {
    location: Location;
    listen(callback: (location: Location) => void): () => void;
    replace(path: string): void;
  }

  const Platform: {
    History?: History;
    version?: string;
  };

  namespace Player {
    const data:
      | {
          context?: { uri?: string };
          isPaused?: boolean;
        }
      | undefined;
    function addEventListener(type: "songchange" | "onplaypause", callback: () => void): void;
    function pause(): void;
  }

  function showNotification(message: string, isError?: boolean, msTimeout?: number): void;
}
