# ADR 0008: Naming

- Status: superseded by ADR 0011
- Date: 2026-10-16

## Context

Tunelock only works with the Spotify desktop app, so people will look for it by searching for Spotify. Spotify's developer policy asks third-party apps not to use names that start with "Spot" or sound confusingly like Spotify, and its trademark license is narrow. Other open source tools that work on top of a product usually follow nominative use: they name the product to say what they work with, in a descriptive phrase, and never suggest affiliation.

## Decision

- The product and repository are named **Tunelock**. The name has no trademark of Spotify or Cold Turkey in it.
- "for Spotify" is a descriptor, not part of the name: the README title is "Tunelock — focus lock for Spotify", and the GitHub description and topics (`spotify`, `spicetify`, `focus`) carry the keywords for search.
- The README states that the project is not affiliated with or endorsed by Spotify AB, Spicetify or Cold Turkey.
- The app icon is our own (a lock with a note, in green on a dark background). It must not use or imitate Spotify's logo, including its three arcs. The green evokes the platform without copying the mark.

## Consequences

- Renaming later, for example to support another player, costs nothing in the name itself. GitHub redirects the old repository URL.
- Discovery depends on keeping the description, topics and README accurate.
- This is not legal advice. If the project grows, a lawyer should review the naming.
