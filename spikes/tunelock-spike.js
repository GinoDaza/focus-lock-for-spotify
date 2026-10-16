// Tunelock spike: can an extension inside the Spotify client reach the
// local service, and which Spicetify APIs does this client expose?
(async function tunelockSpike() {
  if (!globalThis.Spicetify?.showNotification || !Spicetify.Player || !Spicetify.Platform) {
    setTimeout(tunelockSpike, 300);
    return;
  }
  const base = "http://127.0.0.1:39817";
  const apis = {
    historyListen: typeof Spicetify.Platform.History?.listen === "function",
    historyPush: typeof Spicetify.Platform.History?.push === "function",
    playerContext: Spicetify.Player.data?.context?.uri ?? null,
    playUri: typeof Spicetify.Player.playUri === "function",
    spotifyVersion: Spicetify.Platform.version ?? null,
  };
  console.log("[tunelock-spike] apis", apis);
  try {
    const state = await (await fetch(`${base}/v1/state`)).json();
    await fetch(`${base}/v1/heartbeat`, { method: "POST" });
    console.log("[tunelock-spike] OK", state);
    Spicetify.showNotification(
      `Tunelock spike OK · active=${state.active} · history=${apis.historyListen} · v${apis.spotifyVersion}`,
    );
  } catch (error) {
    console.error("[tunelock-spike] FAILED", error);
    Spicetify.showNotification(`Tunelock spike FAILED: ${error}`, true);
  }
})();
