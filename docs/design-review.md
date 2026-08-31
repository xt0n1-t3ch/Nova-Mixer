# Design review

The interface is reviewed by rendering it to PNG and judging the images, rather than by reading the
CSS. A token can be correct in isolation and still produce a page that is cramped, illegible in
light mode, or missing a control affordance.

## Run a capture

```bash
node scripts/capture-preview.mjs .preview
```

The script starts Vite against `frontend/preview.html`, walks the view and theme matrix, writes one
PNG per combination into the given directory, and fails if the page logged a console error. Output
is 2560x1600, a 1280x800 viewport at 2x device scale.

Screenshots are review artifacts, not source. `.preview/` is ignored by git.

## What the preview harness is

`frontend/src/preview/` contains a second entry point that stubs the Tauri transport and serves
seeded data. It is loaded only by `preview.html`; the shipped entry is `frontend/src/main.ts`, which
never imports from that directory.

The stub answers `get_settings`, `list_sessions`, and the plugin commands, so `App.svelte` runs its
real startup path: it installs event listeners, loads settings, applies the persisted theme and
locale, and fetches a snapshot. Only the transport is fake. That is what makes a screenshot evidence
about the real interface rather than about a staged copy of it.

An earlier version of the harness wrote to the stores directly and left the transport returning
`undefined`. The application then cleared those stores during its own startup, and the review caught
several views rendering empty. Serving the transport is the fix, and the reason the harness is
shaped this way.

## The matrix

| View | Themes | Locales |
|:---|:---|:---|
| Mixer | dark, light | en, es |
| Groups | dark, light | en |
| Settings | dark, light | en |
| About | dark | en |

The seeded session list deliberately covers every row state the design has to hold: audible,
grouped, muted, idle, locked, and system sounds. Meters run on deterministic per-session waveforms
so each row shows a genuinely different level instead of a row of identical bars.

Query parameters drive the matrix: `preview.html?view=groups&theme=light&lang=es`.

## Judging

Review every page in both themes. The recurring failures worth checking first:

- **Column alignment.** Every mixer row's meter, slider, percentage, and mute button must sit in the
  same columns. The row is a fixed grid rather than a flex layout for exactly this reason; names
  vary in length and flex would produce ragged sliders.
- **Control affordance.** A slider must not read as a progress bar. The thumb stays visible at rest
  and the empty track keeps an inset border, so a row at 100% is still recognisably adjustable and
  is not confused with the level meter beside it.
- **Light mode as a design, not an inversion.** A glow that reads as depth on black reads as dirt on
  white. Light mode collapses each luminous cue into a crisp ring and a downward shadow.
- **State legibility.** Muted, idle, and locked rows should be distinguishable before reading their
  labels.
- **Spanish overflow.** Spanish strings run longer than English. Check truncation, and check that a
  translated label has not collided with an unrelated one elsewhere on the same screen.
