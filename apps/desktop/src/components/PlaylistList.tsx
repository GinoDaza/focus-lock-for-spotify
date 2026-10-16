/** Playlist IDs, optionally removable. Names would need the Spotify Web API. */
export function PlaylistList({ ids, onRemove }: { ids: string[]; onRemove?: (id: string) => void }) {
  if (ids.length === 0) return <p className="muted">No playlists yet.</p>;
  return (
    <ul className="playlists">
      {ids.map((id) => (
        <li key={id}>
          <code>{id}</code>
          {onRemove && (
            <button className="link" aria-label={`Remove ${id}`} onClick={() => onRemove(id)}>
              Remove
            </button>
          )}
        </li>
      ))}
    </ul>
  );
}
