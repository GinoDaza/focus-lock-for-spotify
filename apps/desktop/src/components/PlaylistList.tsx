import { useEffect, useState } from "react";
import { cachedMeta, fetchMeta, type PlaylistMeta } from "../spotify";
import { Icon } from "./Icon";

function PlaylistRow({ id, onRemove }: { id: string; onRemove?: (id: string) => void }) {
  const [meta, setMeta] = useState<PlaylistMeta | null>(() => cachedMeta(id));

  useEffect(() => {
    let live = true;
    void fetchMeta(id).then((fresh) => {
      if (live && fresh) setMeta(fresh);
    });
    return () => {
      live = false;
    };
  }, [id]);

  return (
    <li className="card playlist">
      <div className="card-main">
        {meta?.thumbnail ? (
          <img className="cover" src={meta.thumbnail} alt="" />
        ) : (
          <span className="cover placeholder">
            <Icon name="music" />
          </span>
        )}
        <div className="card-text">
          <div className="card-header ellipsis">{meta?.title ?? "Playlist"}</div>
          <div className="caption tertiary-text selectable">{id}</div>
        </div>
      </div>
      {onRemove && (
        <button
          className="subtle icon-button"
          title="Remove"
          aria-label={`Remove ${meta?.title ?? id}`}
          onClick={() => onRemove(id)}
        >
          <Icon name="delete" />
        </button>
      )}
    </li>
  );
}

export function PlaylistList({
  ids,
  onRemove,
  empty,
}: {
  ids: string[];
  onRemove?: (id: string) => void;
  empty?: React.ReactNode;
}) {
  if (ids.length === 0) {
    return <div className="card empty secondary-text">{empty ?? "No playlists yet."}</div>;
  }
  return (
    <ul className="cards list">
      {ids.map((id) => (
        <PlaylistRow key={id} id={id} onRemove={onRemove} />
      ))}
    </ul>
  );
}
