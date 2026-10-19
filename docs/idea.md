# Spotify Focus Lock (original idea)

> This is the note that started Focus Lock, kept for context. The current design is in [`plan.md`](plan.md).

## Idea

A study mode for the Spotify desktop app that leaves **only my focus playlists** available. It cannot be turned off until the time I chose has passed.

## Problem

- Fun music (phonk, etc.) distracts me when I read or code.
- I don't want to delete Spotify or use another account.
- I have Premium on my account, but not a second Premium account just for studying.
- I don't have Cold Turkey for apps, so the lock has to hold on its own.

## What it does

1. I choose how many hours the focus mode lasts.
2. While it lasts, Spotify only shows:
   - Binaural Beats: Focus
   - Instrumental Studying (soft/no lyrics)
3. No search, no library, no home: the interface offers no way to reach other music.
4. There is no button to turn it off until the time is up.
5. When it ends, Spotify goes back to normal.

## Technical approach (rough)

- **Spicetify:** an open-source tool that modifies the desktop client with CSS and JavaScript. Chosen because it lives inside Spotify and does not need a separate process.
- **Alternative:** a script using the official Spotify API (possible with Premium) that pauses or redirects playback when something outside the playlists plays. Set aside for now because it runs as a separate process.
- A **custom extension** that hides the interface and stores when the lock ends.
- An **install script** for Windows.

## To do

- Links to the two playlists (Share → Copy link) to get their IDs.
- Install Spicetify on Windows.
- Decide what the hours picker looks like when starting the mode.
- Test that it survives Spotify updates.

## Things to keep in mind

- Modifying the client is against Spotify's terms, although it is a very common practice.
- It is a friction tool meant for me, not a lock that holds against everything.
