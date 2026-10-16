import { useEffect, useState } from "react";
import { api, errorMessage } from "../api";
import { Icon } from "../components/Icon";
import { Card, InfoBar, Page, Section } from "../components/Layout";
import { PlaylistList } from "../components/PlaylistList";
import { parsePlaylistId } from "../format";
import { useAction } from "../hooks";
import type { Allowlist, SessionInfo } from "../protocol";

export function PlaylistsView({ session }: { session: SessionInfo | null }) {
  const [saved, setSaved] = useState<Allowlist | null>(null);
  const [draft, setDraft] = useState<Allowlist>([]);
  const [input, setInput] = useState("");
  const [inputError, setInputError] = useState<string | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const action = useAction();

  useEffect(() => {
    api
      .allowlist()
      .then((ids) => {
        setSaved(ids);
        setDraft(ids);
      })
      .catch((e: unknown) => setLoadError(errorMessage(e)));
  }, []);

  const add = () => {
    const id = parsePlaylistId(input);
    if (!id) {
      setInputError("That isn't a playlist link. Copy it from Spotify as described above.");
      return;
    }
    setInputError(null);
    setInput("");
    if (!draft.includes(id)) setDraft([...draft, id]);
  };

  const dirty = saved !== null && JSON.stringify(saved) !== JSON.stringify(draft);

  return (
    <Page
      title="Playlists"
      description="During a session, Spotify only shows and plays these playlists."
    >
      {loadError && <InfoBar severity="critical" title="Couldn't load playlists.">{loadError}</InfoBar>}
      {session && (
        <InfoBar severity="info" title="A session is running.">
          Changes apply to your next session.
        </InfoBar>
      )}

      <Section title="Add a playlist">
        <Card
          stacked
          icon="add"
          header="Playlist link"
          description="In Spotify, right-click a playlist, then Share › Copy link to playlist."
        >
          <div className="input-row">
            <input
              value={input}
              placeholder="https://open.spotify.com/playlist/…"
              aria-invalid={inputError !== null}
              onChange={(e) => {
                setInput(e.target.value);
                setInputError(null);
              }}
              onKeyDown={(e) => {
                if (e.key === "Enter") add();
              }}
            />
            <button disabled={!input.trim()} onClick={add}>
              Add
            </button>
          </div>
          {inputError && <p className="error-text">{inputError}</p>}
        </Card>
      </Section>

      <Section title={`Allowed (${draft.length})`}>
        <PlaylistList
          ids={draft}
          onRemove={(id) => setDraft(draft.filter((x) => x !== id))}
          empty="Paste a link above to allow your first playlist."
        />
      </Section>

      {dirty && (
        <div className="savebar" role="region" aria-label="Unsaved changes">
          <span>
            <Icon name="info" /> Unsaved changes
          </span>
          {action.error && <span className="error-text">{action.error}</span>}
          {draft.length === 0 && <span className="error-text">Keep at least one playlist.</span>}
          <div className="savebar-buttons">
            <button onClick={() => saved && setDraft(saved)}>Discard</button>
            <button
              className="accent"
              disabled={draft.length === 0 || action.busy}
              onClick={() =>
                void action.run(() => api.setAllowlist(draft)).then((ok) => ok && setSaved(draft))
              }
            >
              Save
            </button>
          </div>
        </div>
      )}
    </Page>
  );
}
