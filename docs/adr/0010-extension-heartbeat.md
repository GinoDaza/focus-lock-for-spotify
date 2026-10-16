# ADR 0010: The extension heartbeat is not authenticated

- Status: accepted
- Date: 2026-10-16

## Context

The extension reports to the service with `POST /v1/heartbeat` over loopback HTTP (ADR 0003). The watchdog uses it to know that Spotify runs with the extension loaded (ADR 0004). The endpoint doesn't check who sends the request. We considered checking the origin of the request or sharing a token between the service and the extension.

## Decision

Keep the heartbeat unauthenticated. It carries no data and cannot change a session: its only effect is the timestamp of the last heartbeat. A check or token would be readable or reproducible by any process running with the user's permissions, so it would add complexity and a dependency on Spotify internals without a real guarantee.

Tunelock adds friction against casual removal. It doesn't try to be impossible to remove for the owner of the machine. The README says so.

## Consequences

- Nothing about the extension channel is treated as proof of integrity. What protects a locked session is the service, its ACLs and the protected Spicetify folders (ADR 0005).
- If we later need a stronger guarantee, it must come from verifying the client from outside the user's session, not from a secret in the extension.
