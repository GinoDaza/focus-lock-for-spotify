import { useEffect, useState } from "react";
import { api, errorMessage } from "../api";
import { PlaylistList } from "../components/PlaylistList";
import { parsePlaylistId } from "../format";
import { useAction } from "../hooks";
import type { Allowlist, SessionInfo } from "../protocol";

export function PlaylistsView({ session }: { session: SessionInfo | null }) {
  const [saved, setSaved] = useState<Allowlist | null>(null);
  const [draft, setDraft] = useState<Allowlist>([]);
  const [input, setInput] = useState("");
  const [inputError, setInputError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const action = useAction();

  useEffect(() => {
    api
      .allowlist()
      .then((ids) => {
        setSaved(ids);
        setDraft(ids);
      })
      .catch((e: unknown) => setInputError(errorMessage(e)));
  }, []);

  const add = () => {
    const id = parsePlaylistId(input);
    if (!id) {
      setInputError("That isn't a playlist link. In Spotify use Share → Copy link to playlist.");
      return;
    }
    setInputError(null);
    setInput("");
    if (!draft.includes(id)) setDraft([...draft, id]);
  };

  const dirty = saved !== null && JSON.stringify(saved) !== JSON.stringify(draft);

  const save = async () => {
    setNotice(null);
    if (await action.run(() => api.setAllowlist(draft))) {
      setSaved(draft);
      setNotice("Saved.");
    }
  };

  return (
    <section className="stack">
      <p className="muted">
        During a session Spotify only shows and plays these playlists.
        {session && " Changes apply to the next session; the running one keeps its playlists."}
      </p>

      <div className="card stack">
        <label className="field">
          <span>Playlist link</span>
          <div className="row">
            <input
              value={input}
              placeholder="https://open.spotify.com/playlist/…"
              onChange={(e) => setInput(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") add();
              }}
            />
            <button className="secondary" disabled={!input.trim()} onClick={add}>
              Add
            </button>
          </div>
        </label>
        {inputError && <p className="error-text">{inputError}</p>}
      </div>

      <div className="card stack">
        <h2>Allowed playlists</h2>
        <PlaylistList ids={draft} onRemove={(id) => setDraft(draft.filter((x) => x !== id))} />
        <div className="row end">
          {notice && !dirty && <span className="muted">{notice}</span>}
          <button
            className="primary"
            disabled={!dirty || draft.length === 0 || action.busy}
            onClick={() => void save()}
          >
            Save
          </button>
        </div>
        {action.error && <p className="error-text">{action.error}</p>}
      </div>
    </section>
  );
}
