# The web version of img2ico: plan

img2ico also gets a version that runs in the browser. This file records what it is, what it is not, and how it is built, so that decisions are not made twice.

## Rules

- **No server.** Everything runs in the visitor's browser; nothing is uploaded. The result is a folder of static files that any web host can serve.
- **No advertising, no paywall, no tracking.**
- **User experience first, then speed, then features.** A feature that makes the page harder to understand does not get in, however useful it is on the command line.
- **Features that make no sense in a browser are left out** (for example `--delete-source`).
- **Hosted on github.io first**, with the option to move to another web space later. For that the page uses only relative paths and no server-side routing; moving it means copying one folder.
- **Multilingual.** The page is multilingual and the languages are fixed: English, German, Spanish, Brazilian Portuguese and French. English comes first; every text goes through a message catalogue from the start, so a language is a new catalogue and not a change to the code. The language is detected from the browser and can be changed by the visitor. The translations themselves come after the first big pull request.
- **Language switcher:** top right in the top bar, next to "Download CLI", showing the flags and names of the five languages.
- **Help in every language.** A help menu next to "Download CLI" in the top bar offers guidance. Each topic is a page of its own, per language, built statically (so it can be found and indexed); its texts are catalogue entries. Topics: getting started, the settings explained, file types, privacy, keyboard shortcuts (a feature still to design), and what the page cannot do and what the command line is for instead.
- **Light and dark mode:** a switch for the theme comes after the first big pull request (the colours are already tokens, so the switch sets a theme on the root element and remembers the choice).
- **Documentation stays English:** the README, the docs and the repository are for GitHub, the contributors and the maintainer, and are written in English only.
- **WYSIWYG**: what the preview shows is what the downloaded file contains.

## How it is built

```
Cargo.toml          the command line (package img2ico) and the workspace
src/                the command line
crates/core/        img2ico-core: bytes and pixels in, bytes out; no files, no threads
crates/wasm/        img2ico-wasm: the thin wasm-bindgen layer the page calls
web/                the page: Svelte, Vite, TypeScript
```

- **One implementation.** The command line and the page both use `img2ico-core`. Code moves from the command line into the core step by step, whenever it is free of file access and operating-system threads. The first piece is the `.ico` validator.
- **The engine runs in a Web Worker**, so the page never freezes while an image is processed.
- **The preview uses the same engine as the export**, not a CSS imitation. Small sizes render immediately (with a short delay while a slider is dragged), large ones in the background. The "saved file" preview decodes the bytes that would be downloaded.
- **No threads inside the engine** at first: shared memory in the browser needs special HTTP headers that github.io cannot set. Work is split per file across workers instead.
- **Messages carry a code and values**, not only an English sentence, so the page can write them in the visitor's language. Every warning and error of the engine is made with `msg!("code", "template {value}", value = ...)` (`crates/core/src/msg.rs`), which makes the same English sentence the command line prints. The engine layer hands the page `{ code, params, text }` (a value that is itself a message, such as the reason inside "could not read …", comes as a message again), and the page writes the sentence from the catalogue entry `msg.<code>`; a message without a code, or a code the catalogue does not know yet, is shown as its English `text` with the option names put into words (`web/src/lib/messages.ts`). `web/src/lib/codes.test.ts` fails when a code has no catalogue entry, an entry has no code, or a sentence uses a value the engine does not give. Sentences from the libraries the engine uses (an image that cannot be decoded, for instance) are still English reasons inside our own message.

### Why Svelte

The heavy work is in WebAssembly; the page itself is mostly state: many controls, a live preview, a crop overlay, undo/redo, translated texts. Svelte compiles to small plain JavaScript (no virtual DOM, a few kilobytes of runtime) and keeps that state code short. The output is a static folder.

## What carries over from the command line, and what does not

**Carries over:** conversion to `.ico` and `.icns`, size selection, padding, fit, crop, trim, corner radius, grayscale, background removal (chroma key, seed, find), SVG and GIF-frame sources, inspect, validate, extract, select.

**Left out:** `--merge` (the queue replaces it: an `.ico` holds one image per size, so complete icons cannot be combined; single images can be taken out of an icon), folders and `--recursive`, include/exclude filters, `--delete-source`, settings files and `--config`, `--jobs`, atomic writing, exit codes, `--json`, the batch report. `--max-pixels` stays as a fixed safety limit with a clear message. A later iteration replaces folder conversion with a ZIP upload and a ZIP download.

## Web-only features (from the feature list)

**First versions:** drag and drop, paste from the clipboard (also screenshots), a warning when the source is too small, a live preview with all sizes, before/after, light/dark/grey/custom backgrounds, Windows- and file-manager-style views at true size, zoom, pixel grid and pixel inspector (colour and alpha), a preview of the actual saved file, an interactive crop frame with aspect-ratio presets, keyboard operation and accessibility, touch, tooltips and help at the control, remembering the last settings, presets (such as Windows app, macOS, favicon), quick size switching, reset for each setting, version and changelog, an example image.

**Later:** a light/dark/automatic theme switch, keyboard shortcuts, undo/redo, rotate and mirror, comparing variants, crop per size, exporting and importing settings as a file, applying settings to several files, custom presets, copying the result to the clipboard.

**Not planned:** everything that belongs to a desktop program (target folders, opening folders, tray, sounds, shell integration, plugins, tabs, a separate preview window, recent files), a customizable interface, and image-editor features (pixel editor, shapes, lines, watermarks, palettes, guides).

**Ideas outside the list:** a favicon package as one ZIP (`favicon.ico`, PNGs, Apple touch icon, web manifest), and a background tile or shape behind the artwork. A **queue**: finish an icon with its own settings, add it, choose the next picture, and at the end download all icons as one ZIP.

## The page as it is built

- **Start page:** one drop zone (drop, choose, or paste). Several pictures or a ZIP go straight into the queue. An `.ico` is opened for inspection; "Edit as picture" turns its largest image into a picture to rework.
- **Editor, top to bottom:**
  - *Top bar:* back, the file name, the original picture with its size, "Reset to defaults" (asks first).
  - *Download bar* (stays in view while the page scrolls): the file type (`.ico`, `.icns`, website ZIP; the ZIP's settings are in a dialog, reopened by the gear), the PNG ZIP, "update in queue" / "add as a new icon", and the download.
  - *Preview:* the views Icon, Before and after, Pixels; a column of background swatches (hover shows, click keeps, up to seven own colours); all sizes below. "Crop" is in the Icon view only.
  - *Settings column:* "Quick start" (what it is for, style) and the "Advanced editor" (sizes, background removal, margin, corners, look). It folds to a slim rail of icons; the choice is remembered, and starting a crop folds it away and opens it again afterwards.
- **Crop:** the frame stays in the middle of the preview with the shape of the crop; the picture is dragged and zoomed beneath it (the wheel zooms anywhere on the preview). A lock switches to the other way: the picture stays and the frame is dragged by its handles. Aspect ratios and the four measures sit in a box under the picture. Crop is an engine setting like any other, so the saved file matches the preview.
- **Queue:** shown beside the editor on a wide window and below it on a narrow one. Items are live documents: changing the icon being edited is saved to the queue by itself (switchable), and downloading first waits for the preview to settle. Icons can be reordered, compared two at a time with a divider, and downloaded as one ZIP. Files can be dropped anywhere on the editor. The queue lives in the page only; leaving asks for confirmation while it is not empty.
- **Animated GIF:** a small player appears in the preview when the picture is an animated GIF: play/pause, previous and next frame, a slider, the frame shown. The icon is made from the frame you stop at (this replaces `--gif-frame`); while the animation plays the icon keeps its frame. The chosen frame is part of an icon's settings in the queue. The engine decodes all frames once (`GifFrames`, at most 64 million pixels in all; a GIF with more is used from its first frame).
- **Remembered in the browser** (localStorage, never sent anywhere): the last settings, whether the queue saves by itself, whether the settings column is folded.
- **Pixel view:** picks the colour to remove directly from the picture, then opens the background settings and flashes them once. The general rule: when an action changes settings elsewhere on the page, the page opens that area, scrolls to it and flashes it once.

## Search engines (SEO)

Being found counts as part of the page, so it is built in from the start and checked with every change to the page:

- **Text before any script runs.** The heading, the explanation (what it does, how it works, questions and answers) and the footer are plain HTML in `web/index.html`, filled from the message catalogue at build time by `web/seo.ts`. A crawler without JavaScript reads the same text as a visitor; the app fills only the converter. The explanation shows on the start page and hides once a picture is open.
- **Head tags:** a title and description with the words people search for, canonical link, Open Graph and Twitter tags, a theme color for both color schemes, and schema.org `WebApplication` data (free, runs in the browser).
- **robots.txt** always; **sitemap.xml, canonical link and the link-preview picture only when the address is known**, because they need absolute URLs and the page must stay movable: `SITE_URL=https://example.org/img2ico/ npm run build`. The deployment step sets it.
- **Speed:** the engine starts when the browser is idle, not before the page is shown.
- **Structure:** one `h1`, headings in order, landmarks (header, main, footer), `lang` follows the language in use.
- **Check:** `npx lighthouse <address> --only-categories=seo,accessibility,best-practices,performance` should give 100 for the first three. `web/seo.test.ts` tests the generated tags and text. Last measured on the start page: performance 96, accessibility 100, best practices 100, SEO 100 (what is left is the engine's 4 MB).
- **When languages are added:** one page per language (`/de/`, `/es/`, `/pt-br/`, `/fr/`) with `hreflang` links, not one page that switches by script; crawlers index what the address returns. The detection of the browser language only chooses where a first visit lands.
- **When the design is done:** the link-preview picture `web/public/og-image.png` is a plain placeholder and is redone then.

## Checks

Pull requests that only change `web/`, `docs/` or Markdown files skip the Rust checks (they still report as passed, which the protected `main` branch needs). Pull requests that touch Rust code, the workspace or the workflows run everything, and the web checks run too whenever the core or the bindings change. See `.github/workflows/ci.yml` and `web.yml`.

## Order of work

1. Workspace, shared core, bindings, CI paths. (Done.)
2. The page's foundation: build setup, worker, message catalogue, a first working screen. (Done: open an `.ico`, validate it.)
3. The conversion pipeline in the core, byte-identical to the command line. (Done: the command line itself runs on it, so there is one implementation. Errors are still English sentences; codes for translation come when the first translation does.)
4. Convert: drop or paste a picture, live preview of every size on four backgrounds, settings (sizes, margin, rounded corners, fit, black and white, trim, background removal), download as `.ico` or `.icns`. (Done in its first form.)
5. Features, until every one of them works and has been checked. (Done: crop frame, before/after, pixel inspector, presets and remembered settings, inspect/extract/select of `.ico` files, several pictures or a ZIP at once, PNG ZIPs, the website icon package.)
6. Design: only once all features work. The page is looked at and reworked until it looks the way its owner wants. (Done in this branch; the first big pull request carries it.)
7. Deployment to github.io, offline use: only once the design is approved. Until then the page is developed and tried locally (`npm run dev`) and nothing is published.

After the first big pull request, in this order: the translations (with the page per language and the language switcher), the help pages, the light/dark switch, keyboard shortcuts, codes instead of sentences for engine messages; the API last (see below).

**API (a fun feature, last).** A static page cannot receive HTTP requests, so a way to hand it a settings JSON and one encoded picture and get an encoded `.ico` back would be a `window.postMessage` protocol (with an allow-list of origins), a global function for scripts, and, for real programs, the engine as an npm package. Nothing is uploaded in any of them.

SVG text is an open question: the browser has no system fonts for the engine's SVG renderer. Either a font is bundled (bigger download) or SVGs are drawn by the browser's own renderer; this is decided after measuring.
