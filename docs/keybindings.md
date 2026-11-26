# Rating, favorite and discard shortcuts

The listener can decide about the track that is playing without leaving the player.

| Key | Decision | Effect |
| --- | --- | --- |
| `1` … `5` | rate | Saves 1–5 stars on the provider (Subsonic `setRating`) and appends a `rate` decision |
| `0` | clear rating | Removes the provider rating and appends a `rate` decision with `stars: 0` |
| `l` | favorite | Toggles the heart, through the same path as the player-bar button |
| `x` | discard | Appends a `discard` decision and plays the next track |

The shortcuts are registered in `crates/input/src/lib.rs` and handled in
`crates/sonora/src/actions.rs`. Discard is a decision, not a delete: it records that the track
should be left out and skips it, leaving the provider and the queue untouched so the AI side can
act on the log.

## Contexts and conflicts

The shortcuts use the `Workspace` context minus text input, menus and forms:

```
Workspace && !Input && !Menu && !Form
```

That keeps `1`–`5`, `l` and `x` typing into the search field, a rename field or any other
text input, while still working when a track table has focus (the table context is nested
under `Table`, which the shortcuts do not exclude). They do not use `space`, `ctrl-left`,
`ctrl-right` or `f`, so playback and fullscreen keep their keys. No existing binding used
`0`–`5`, `l` or `x` alone.

## The decisions log

Every shortcut appends one JSON object per line to:

```
$XDG_STATE_HOME/sonora/decisions.jsonl
```

falling back to the cache folder when the platform has no state folder. A line looks like:

```json
{"at":1764000000,"action":"rate","stars":4,"id":"42","title":"梧桐灯","artist":"许嵩","album":"不如吃茶去","provider":"subsonic"}
{"at":1764000012,"action":"favorite","saved":true,"id":"42","title":"梧桐灯","artist":"许嵩","album":"不如吃茶去","provider":"subsonic"}
{"at":1764000030,"action":"discard","id":"42","title":"梧桐灯","artist":"许嵩","album":"不如吃茶去","provider":"subsonic"}
```

The log records the listener's intent. A favorite or rating that the provider refuses is
reported by the player as usual, but the decision stays in the log.

## Reading it from the CLI

```bash
sonora decisions              # newest 50, oldest first, as a table
sonora decisions -n 0         # every decision
sonora decisions --json       # one JSON object per line, for a script
```

`sonora decisions` reads the file directly, so it works without the app running and does not
go through the socket protocol.

## Design note

Two shapes were weighed:

1. **Local log only** — a new state module, the two CLI commands and the keybindings, roughly
   260 lines. Ratings would not leave the machine.
2. **Log plus provider sync** — the same, plus a default `MusicApi::set_track_rating` and the
   Subsonic `setRating` call, roughly 350 lines. Stars then also show up in Navidrome and other
   Subsonic clients, and the existing `Library` path still owns favorites.

The second was chosen: it keeps the AI-facing log as the source of truth for decisions while
making ratings real for every other client. No other provider is touched; its default trait
method refuses the call, and the local decision is still written.
