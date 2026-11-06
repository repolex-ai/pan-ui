# pan-ui

Look through a [Pan](https://github.com/repolex-ai/pan) store fast, and rate it from the keyboard.

Pan keeps images and what is known about them in a graph. `pan-ui` shows one store at a time, by
set or all of it, newest image first, and lets you mark each image the way Lightroom does: 0 to 5 stars, and a
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
| `R` | a random image from the set, among those the filter shows; for finding older work that browsing newest-first never reaches |
| `Shift` + `R` | back to the image you were on before the last `R`; press again to keep going back |
| `S` | more like this: the image and the 300 that look most like it, most alike first |
| `Backspace` | out of "more like this", back to the set |
| `1`–`5` | stars |
| `0` | no stars |
| `P` | pick (white flag) |
| `X` | reject (black flag) |
| `U` | no flag |
| `,` (`<`) | flag one step toward pick: reject becomes no flag, no flag becomes pick |
| `.` (`>`) | flag one step toward reject: pick becomes no flag, no flag becomes reject |
| `Shift` + `↑` | one star more; an unrated image gets 4, as 3 is average. Stops at 5 |
| `Shift` + `↓` | one star less; an unrated image gets 3, average. Stops at 1; `0` clears |
| `Shift` + any mark | mark, then move to the next image |
| `=` `-` | bigger / smaller thumbnails |
| `Tab` | hide / show both side panels |
| `Shift` + `Tab` | hide / show every panel, filmstrip included |

## What it draws

The top bar switches between three modules, each taking the whole screen: **Library** (the
viewer, below), **Lab** (development views that do not fit the Library layout, listed in
`web/src/lib/labs/index.ts`) and **Config** (pand's own files, below). The module is in the
address as `?m=lab` or `?m=config`.

The Library is laid out like Lightroom's:

- **Left:** the stores `pand` holds, then the sets of the open store, folded into months and
  days by the date of each set's first image. Typing in the box above them lists every set whose
  name matches, unfolded. "Everything, newest first" is the whole store. Every set shows its
  newest image first.
- **Middle:** the grid of thumbnails, or one image at full size. A rejected image is dimmed;
  stars and flags sit in its corner. The bar above it filters the set: any mix of picked,
  unflagged and rejected (none chosen shows all), and a star rating that the image's must be at
  least, at most, or exactly (○ is no stars; click a chosen star again to drop it). The filter
  stays as you change sets. Mark an image out of the filter and it leaves, and the cursor is on
  the next one.
- **Bottom, at full size:** the filmstrip, the whole set as one row, the current image in the
  middle.
- **Right:** what Pan knows about the image under the cursor, in sections that fold: file, rating,
  the sets that hold it, and every fact. Description and Generation (prompt, model, seed) appear
  only for images that have such facts, so a store of screenshots shows neither.

Each panel puts itself away with the triangle on its edge of the screen, and stays put away
after a reload. So does each folded section.

Switching sets keeps your place. If the new set holds the image under the cursor ("everything"
always does), it stays on that image; if not, it goes to the set's image nearest to it in time.

The address bar carries the store, the set and the image (`#700c5b/<set>/<image>`), so a reload
comes back to the same place and a view can be sent to someone.

## The Lab

The Lab's views read data that `tools/lab_build.py` works out once per store from what `pand`
already holds: the embeddings, from `GET /stores/<id>/embeddings?format=f16` (half precision,
about 290 MB for 70,000 images; `--format f32` for full), and the body poses, by a SPARQL query. It talks to `pand` over HTTP only (`--pand` gives its address), so Pan can be on
another machine, and it never writes to the store. Build it, then gather the captions for the
word search:

```bash
uv venv tools/.venv && uv pip install --python tools/.venv/bin/python numpy umap-learn scikit-learn
tools/.venv/bin/python tools/lab_build.py <store-id>    # under two minutes for 70,000 images
tools/.venv/bin/python tools/lab_words.py <store-id>    # about two minutes for 200,000 images
```

Both write to `~/.pan-ui/lab/<store>/`, which can be deleted at any time. The data is a
snapshot: Pan keeps embedding new images, and the Lab bar says when the snapshot was taken and
how many newer embeddings `pand` holds. Run the two commands again to catch up. The build keeps
the full embeddings it fetched (`raw.f32`) and next time asks `pand` only for those produced
since (`?since=`, with `pand`'s own snapshot time, so two machines' clocks never disagree), so catching up downloads hundreds of rows, not the whole store; `--fresh`
fetches everything again.

The embeddings are kept in memory reduced to 256 dimensions (by PCA), so "what looks like this?"
is answered in about 50 ms without asking `pand`. Scores in the Lab are in that reduced space,
close to but not the same as `pand`'s own search.

- **Atlas.** Every image with an embedding as a dot, placed so that look-alikes sit together
  (UMAP), coloured by place in time order. Zoom in and the dots become the pictures. Play runs the
  store's history. Click a dot for its nearest images, and the nearest from outside its own sets.
- **Joystick.** One image and a step in each of eight directions: the two ways the images around
  it differ most, worked out afresh at each step. `Q W E / A D / Z X C` or the arrows to steer,
  `Space` to jump anywhere, `Backspace` to go back.
- **Words & Looks.** Caption search, and beside it the images that look like the matches but
  whose captions never use the words: what a word search alone cannot find.
- **Mixer.** More like these, less like those, weighted. Scope keeps to the first image's own
  sets, leaves them, or shows both side by side; images cluster hard by the set they were made in,
  and leaving it shows what an image resembles anywhere else.
- **Echoes.** Pairs that look alike but were made at least three weeks apart, then and now.
- **Pose.** Drag a stick figure; the bodies that stand that way come up as you drag. Joints you
  move count most.
- **Sets.** Every set as a circle where its images sit on the Atlas, joined to the sets it looks
  most like.
- **Diary.** For a soul's store: the soul's memory notes from ravel (`raveld` on port 7881, same
  id as the store), each beside the images made while it was the latest note.
- **Facets.** Every property the images carry, its commonest values as bars, and the images
  behind each value. Asks `pand` directly; needs no Lab data.
- **Daydream.** The joystick with nobody holding it: a slow drift through the store, full size.

Double-click any image in the Lab to open it in the Library.

## pand's config and prompts

The Config module opens pand's own files as plain text:
`~/.config/pan/config.yml` (the stores, the port, and every model pass: which model, its endpoint,
its prompt, and the settings sent with each request) and the caption prompts in
`~/.config/pan/prompts/`. Plain text on purpose: the comments in config.yml carry the reasons
behind each setting, and a form would lose them.

Saving is careful, because a broken config.yml stops pand from starting:

- A new config.yml is checked with `pand check-config`, pand's own start-time loader, before it
  is saved. A mistyped key, a wrong value or a missing prompt is refused with pand's message, and
  the file on disk is not touched.
- The old file is copied to `~/.config/pan/backups/` first, then the new text is written to a
  temporary file and renamed into place, so pand only ever finds the old file or the new one.
- If the file changed on disk after you opened it, the save is refused rather than overwriting it.
- `auth:` values are hidden in the editor and put back on save.
- A prompt whose name ends `.default.md` ships with Pan and is rewritten by pand, so it cannot be
  saved; save your version as a new prompt and point the pass's `prompt:` line at it.

pand reads these files once, when it starts. A saved change takes effect at the next pand restart;
pan-ui does not restart it.

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
membership. The Lab only reads, from `pand` and from `raveld`, and never from a store's files.
