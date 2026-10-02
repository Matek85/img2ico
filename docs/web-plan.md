# The web version of img2ico: plan

img2ico also gets a version that runs in the browser. This file records what it is, what it is not, and how it is built, so that decisions are not made twice.

## Rules

- **No server.** Everything runs in the visitor's browser; nothing is uploaded. The result is a folder of static files that any web host can serve.
- **No advertising, no paywall, no tracking.**
- **User experience first, then speed, then features.** A feature that makes the page harder to understand does not get in, however useful it is on the command line.
- **Features that make no sense in a browser are left out** (for example `--delete-source`).
- **Hosted on github.io first**, with the option to move to another web space later. For that the page uses only relative paths and no server-side routing; moving it means copying one folder.
- **English first**, but every text goes through a message catalogue from the start, so more languages can be added without touching the code.
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
- **Errors carry a code and parameters**, not a finished English sentence, so the page can translate them.

### Why Svelte

The heavy work is in WebAssembly; the page itself is mostly state: many controls, a live preview, a crop overlay, undo/redo, translated texts. Svelte compiles to small plain JavaScript (no virtual DOM, a few kilobytes of runtime) and keeps that state code short. The output is a static folder.

## What carries over from the command line, and what does not

**Carries over:** conversion to `.ico` and `.icns`, size selection, padding, fit, crop, trim, corner radius, grayscale, background removal (chroma key, seed, find), SVG and GIF-frame sources, inspect, validate, extract, merge, select.

**Left out:** folders and `--recursive`, include/exclude filters, `--delete-source`, settings files and `--config`, `--jobs`, atomic writing, exit codes, `--json`, the batch report. `--max-pixels` stays as a fixed safety limit with a clear message. A later iteration replaces folder conversion with a ZIP upload and a ZIP download.

## Web-only features (from the feature list)

**First versions:** drag and drop, paste from the clipboard (also screenshots), a warning when the source is too small, a live preview with all sizes, before/after, light/dark/grey/custom backgrounds, Windows- and file-manager-style views at true size, zoom, pixel grid and pixel inspector (colour and alpha), a preview of the actual saved file, an interactive crop frame with aspect-ratio presets, dark/light/automatic theme, keyboard operation and accessibility, touch, tooltips and help at the control, remembering the last settings, presets (such as Windows app, macOS, favicon), quick size switching, reset for each setting, version and changelog, an example image.

**Later:** undo/redo, rotate and mirror, comparing variants, crop per size, exporting and importing settings as a file, applying settings to several files, custom presets, copying the result to the clipboard.

**Not planned:** everything that belongs to a desktop program (target folders, opening folders, tray, sounds, shell integration, plugins, tabs, a separate preview window, recent files), a customizable interface, and image-editor features (pixel editor, shapes, lines, watermarks, palettes, guides).

**Ideas outside the list:** a favicon package as one ZIP (`favicon.ico`, PNGs, Apple touch icon, web manifest), and a background tile or shape behind the artwork. A **queue**: finish an icon with its own settings, add it, choose the next picture, and at the end download all icons as one ZIP. (An `.ico` file holds one image per size, so several complete icons cannot go into one `.ico`; the queue therefore has no "combine" button. Taking single images out of an icon with several is possible.)

## Search engines (SEO)

Being found counts as part of the page, so it is built in from the start and checked with every change to the page:

- **Text before any script runs.** The heading, the explanation (what it does, how it works, questions and answers) and the footer are plain HTML in `web/index.html`, filled from the message catalogue at build time by `web/seo.ts`. A crawler without JavaScript reads the same text as a visitor; the app fills only the converter. The explanation shows on the start page and hides once a picture is open.
- **Head tags:** a title and description with the words people search for, canonical link, Open Graph and Twitter tags, a theme color for both color schemes, and schema.org `WebApplication` data (free, runs in the browser).
- **robots.txt** always; **sitemap.xml, canonical link and the link-preview picture only when the address is known**, because they need absolute URLs and the page must stay movable: `SITE_URL=https://example.org/img2ico/ npm run build`. The deployment step sets it.
- **Speed:** the engine starts when the browser is idle, not before the page is shown.
- **Structure:** one `h1`, headings in order, landmarks (header, main, footer), `lang` follows the language in use.
- **Check:** `npx lighthouse <address> --only-categories=seo,accessibility,best-practices,performance` should give 100 for the first three. `web/seo.test.ts` tests the generated tags and text.
- **When languages are added:** one page per language (`/de/`) with `hreflang` links, not one page that switches by script; crawlers index what the address returns.
- **When the design is done:** the link-preview picture `web/public/og-image.png` is a plain placeholder and is redone then.

## Checks

Pull requests that only change `web/`, `docs/` or Markdown files skip the Rust checks (they still report as passed, which the protected `main` branch needs). Pull requests that touch Rust code, the workspace or the workflows run everything, and the web checks run too whenever the core or the bindings change. See `.github/workflows/ci.yml` and `web.yml`.

## Order of work

1. Workspace, shared core, bindings, CI paths. (Done.)
2. The page's foundation: build setup, worker, message catalogue, a first working screen. (Done: open an `.ico`, validate it.)
3. The conversion pipeline in the core, byte-identical to the command line. (Done: the command line itself runs on it, so there is one implementation. Errors are still English sentences; codes for translation come when the first translation does.)
4. Convert: drop or paste a picture, live preview of every size on four backgrounds, settings (sizes, margin, rounded corners, fit, black and white, trim, background removal), download as `.ico` or `.icns`. (Done in its first form.)
5. Features, until every one of them works and has been checked. (Done: crop frame, before/after, pixel inspector, presets and remembered settings, inspect/extract/select/merge of `.ico` files, several pictures or a ZIP at once, PNG ZIPs, the website icon package.)
6. Design: only once all features work. The page is looked at and reworked until it looks the way its owner wants.
7. Deployment to github.io, offline use: only once the design is approved. Until then the page is developed and tried locally (`npm run dev`) and nothing is published.

SVG text is an open question: the browser has no system fonts for the engine's SVG renderer. Either a font is bundled (bigger download) or SVGs are drawn by the browser's own renderer; this is decided after measuring.
