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

The seeded list uses real applications and their real icons, and covers every row state the design
has to hold: pinned, several streams, grouped, muted, idle, locked, saved with no session, and system
sounds. Do not invent an application or give one another application's icon.

The preview has no audio device, so every meter stays at zero. Meters are never animated with
invented data. To see real levels, run the app, or read them from the console with
`cargo run -p novamixer-cli -- doctor --watch --peaks`, which prints the same
`IAudioMeterInformation::GetPeakValue` batch the meters draw.

Query parameters drive the matrix: `preview.html?view=groups&theme=light&lang=es`.

## Judging

Review every page in both themes. The recurring failures worth checking first:

- **Colour means signal.** The chassis is monochrome: neutral greys with an ink accent, near-white on
  dark and near-black on light. Saturated colour is reserved for meters, the live count, mute and
  destructive actions. A tinted fader, badge, avatar, brand mark or background is a defect. An
  earlier build tinted the whole chassis indigo, which left signal nothing to contrast against.
- **One frame for every view.** Each view opens with `PageHeader.svelte`: one row with the title on
  the leading edge and the view's actions on the trailing edge. There are no subtitles. The
  secondary views share one grid, a `--side-panel-width` column beside a fluid main column. A view
  that invents its own top or its own column widths is a defect.
- **One top bar, no sidebar.** `TopBar.svelte` carries the logo, the four views as underlined
  tabs, the command palette, theme, language, shortcuts, and the window controls. Below 1060px the
  tab labels hide and the icons stay. A second navigation surface is a defect.
- **Channel rows on one grid.** Every application is one row on `--grid-channel`: identity, the
  fader with its meter rail directly beneath it on the same scale, the readout, mute, and row
  actions. All faders, readouts and mute buttons stand in the same columns. At the default
  1080x720 window every seeded application is visible without scrolling.
- **The output leads.** The output row sits above the channel list and never scrolls with it. It
  has one meter, because the endpoint reports one peak; two meters would invent a stereo reading.
- **Everything stays visible.** Nothing clips, overflows the frame, or scrolls sideways. Only the
  channel list scrolls, with the native scrollbar (`scrollbar-width` and `scrollbar-color`, no
  hand-painted `::-webkit-scrollbar`).
- **Application icons.** Icons are the application's own 128px shell artwork, drawn whole with no
  radius or frame. A blurry or clipped logo means the extraction path fell back to 32px.
- **Control affordance.** A slider must not read as a progress bar or as a meter. The fader is a
  rounded capsule with a visible thumb; the meter is a row of square segments. That shape contrast
  is what separates the control from the signal beside it.
- **Light mode as a design, not an inversion.** A glow that reads as depth on black reads as dirt on
  white. Light mode collapses each luminous cue into a hairline and a downward shadow, and every
  text token is re-derived rather than reused: inheriting dark mode's muted greys onto white is what
  once made secondary rows and column legends unreadable.
- **Ink on a filled control.** A glyph on a solid `--danger` fill uses `--danger-on`, never
  `--accent-fg`. The accent foreground inverts per theme and once put near-black on dark mode's
  light red at 2.77:1, turning the mute button into a blank red square.
- **Panels hug their content.** Empty page below the last card is fine. Empty space trapped *inside*
  a card's border is not: it reads as a rendering fault or as content that failed to load.
- **State legibility.** Muted, idle and locked rows should be distinguishable before reading their
  labels.
- **Spanish overflow.** Spanish strings run longer than English. Check truncation, and check that a
  translated label has not collided with an unrelated one elsewhere on the same screen. A badge must
  never win width against the label it annotates.
