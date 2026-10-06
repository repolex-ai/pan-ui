# pan-ui

Look through a [Pan](https://github.com/repolex-ai/pan) store fast, and rate it from the keyboard.

Pan keeps images and what is known about them in a graph. `pan-ui` shows one store at a time, by
set or in time order, and lets you mark each image the way Lightroom does: 0 to 5 stars, and a
white flag (pick), a black flag (reject) or no flag. Marks are saved back to Pan, on the image.

It is the Pan counterpart of [git-lex-ui](https://github.com/repolex-ai/git-lex-ui) and
[ravel-ui](https://github.com/repolex-ai/ravel-ui): one daemon (`pand`) holds every store, and one
page shows any of them.

## Run it

`pand` must be running (it listens on `127.0.0.1:7401`). Then:

```sh
cd web && npm install && npm run build && cd ..
cargo run --release -- --port 8890
```

The server binds `127.0.0.1:8890` and opens your browser.

| Flag | Default | Description |
|---|---|---|
| `--port <n>` | `8890` | Port for pan-ui. Fails loudly if taken. |
| `--daemon-port <n>` | `7401` | Where `pand` listens. |
| `--data <dir>` | `~/.pan-ui` | Where the store indexes and the save queue are kept. |
| `--no-open` | `false` | Do not open a browser tab on startup. |
| `--web <dir>` | `./web/dist` | Frontend build directory, read on every request. |

For frontend work, run `npm run dev` in `web/` as well. Vite serves the page on
`http://localhost:5175` and forwards `/api` to the server on 8890.

## Keys

| Key | Does |
|---|---|
| `←` `→` | previous / next image |
| `↑` `↓` | up / down a row in the grid; previous / next in the full-size view |
| `Home` `End` `PgUp` `PgDn` | first, last, four rows up or down |
| `Enter` `Space` | full size, and back |
| `E` | full size |
| `G` `Esc` | back to the grid |
| `1`–`5` | stars |
| `0` | no stars |
| `P` | pick (white flag) |
| `X` | reject (black flag) |
| `U` | no flag |
| `Shift` + any mark | mark, then move to the next image |
| `=` `-` | bigger / smaller thumbnails |

## What it draws

- **Left:** the stores `pand` holds, then the sets of the open store, newest first, with a filter.
  "Everything, in time order" is the whole store.
- **Middle:** the grid of thumbnails, or one image at full size. A rejected image is dimmed in the
  grid; stars and flags sit in its corner.
- **Right:** what Pan knows about the image under the cursor: date, size, model, seed, caption,
  prompt, and every fact on request. The keys are listed at the bottom.

The address bar carries the store, the set and the image (`#700c5b/<set>/<image>`), so a reload
comes back to the same place and a view can be sent to someone.

## How it stays fast

- **The store is indexed once.** Listing every image of a 200,000-image store takes `pand` about
  ten seconds, so `pan-ui` asks once, keeps the answer (image, date, sets, stars, flag) in memory,
  and keeps a copy in `~/.pan-ui/index/` so the next start shows the store at once. The copy is
  disposable. `pan-ui` rebuilds it when `pand` reports a different number of images.
- **Only the visible rows of the grid exist.** Scrolling through 200,000 thumbnails costs what a
  screenful costs.
- **The next images are decoded before you reach them.** The full-size view keeps a small pool of
  decoded images around the current one, mostly ahead in the direction you are moving, so moving
  to the next image swaps in pixels that are already there. If you outrun it, the thumbnail stands
  in until the full image is ready.
- **A mark does not wait for Pan.** Saving a rating rewrites the image's own metadata, which is
  right (the rating travels with the file) and too slow to wait for between key presses. A mark
  updates the page at once and goes into a queue on disk (`~/.pan-ui/queue.json`), which a
  background task hands to `pand`. If `pand` is down, marks wait in the queue and go when it is
  back, even after a restart. The top right of the page says whether everything is saved.

## What it writes

`pan-ui` writes three properties on images, through `pand`'s `/media/{id}/set` and `/unset`:
`pan:rating`, `pan:isPicked` and `pan:isRejected`. No stars and no flag are removals, not zeros or
falses. It reaches no other route that changes anything: not delivery, not deletion, not set
membership.
