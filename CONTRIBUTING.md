# Contributing to img2ico

This file is for people who want to build, change or release img2ico. If you only want to use it, the [README](README.md) is all you need.

## Building and testing

You need Rust **1.88 or newer** (the project uses the 2024 edition); [rustup](https://rustup.rs) installs it.

```
cargo build --release             # the binary: target/release/img2ico
cargo test --workspace            # unit tests and end-to-end tests that run the real binary
cargo fmt --all --check           # formatting
cargo clippy --workspace --all-targets -- -D warnings
```

The repository is a Cargo workspace. The command line is the package at the root; `crates/core` holds the code it shares with the web version, and `crates/wasm` the WebAssembly bindings for the page in `web/`. The plan for the web version is in [docs/web-plan.md](docs/web-plan.md).

Use a current stable toolchain for `clippy`: its rules grow with every release, and the automated checks use the newest stable. `Cargo.toml` names the oldest supported Rust version (`rust-version`); the checks run the tests with exactly that version too, so keep both in step.

On Windows the build embeds the program's icon (`assets/img2ico.ico`) in the `.exe` with the system's resource compiler (part of the Windows SDK that the Rust toolchain needs anyway); if it is missing the build still succeeds on your machine, with a warning, just without the icon - but fails in the automated builds (where the `CI` variable is set), so a release cannot lose its icon.

The tests come in three kinds:

- **Unit tests** next to the code they test.
- **End-to-end tests** in `tests/cli.rs`, which start the real binary on temporary files and check what it prints, writes and exits with.
- **Property-based tests** that compare the optimized chroma-key and `--find` code against a deliberately naive reference implementation on random images, so speed-ups cannot silently change results.

## The web page

The page in `web/` needs [Node.js](https://nodejs.org) (the current LTS) and the `wasm32-unknown-unknown` Rust target (`rustup target add wasm32-unknown-unknown`). It also needs the `wasm-bindgen` tool in exactly the version `Cargo.lock` names; if yours does not match, the build tells you the command to install the right one.

```
cd web
npm install
npm run dev                       # builds the engine, then serves the page with live reload
npm run check                     # type-check
npm test                          # the page's tests
npm run build                     # the finished static page in web/dist
```

`SITE_URL=https://example.org/img2ico/ npm run build` also writes the canonical link, the link-preview address and `sitemap.xml` (without it they are left out, so the build works from any address). See the search engine section of `docs/web-plan.md`.

Rules for changes to the page:

- **Every text goes through the message catalogue** (`web/src/i18n/en.ts`, used with `t('key', { params })`); nothing is written into the markup. The page will be translated, so the same placeholders must be usable in every language, and plurals are keys with `_one` / `_other`. Texts of the engine are shown through `web/src/lib/messages.ts`.
- **Buttons** are sized by the classes at the end of `web/src/app.css` (`primary`, `outline`, `quiet`, `icon-button`, `chip`, `small`); a button that does something has a line icon (`web/src/lib/icons.ts`) in front of its text, and an icon-only button has a tooltip and an `aria-label`.
- **A new warning or error of the engine is made with `msg!`** (`crates/core/src/msg.rs`: `msg!("area.what", "English sentence with {values}", value = ...)`), never with a bare `format!`, and gets a sentence in `web/src/i18n/en.ts` under `msg.area.what` that uses the same values. `npm test` checks that every code has one.
- **The page and the command line must give the same bytes.** The page only calls the engine; nothing in `web/` converts pixels itself.
- **Relative paths only**, so the built folder works from any address.
- Before a pull request: `npm run check`, `npm test`, `npm run build`, and, for the Rust side, the commands of the Rust CI jobs. A Lighthouse run (see `docs/web-plan.md`) catches problems with accessibility and search engines.

`npm run build:wasm` rebuilds only the engine (`crates/wasm`) into `web/src/wasm/pkg/`, which is generated and not committed. The `web/dist` folder works from any address, so it can be copied to any web space.

## Where things are

| Module | What it does |
|---|---|
| `main.rs`, `cli.rs` | The program entry, the mode dispatch and the command-line definition (the `--help` texts live in `cli.rs`) |
| `config.rs`, `settings.rs` | Settings files, and how command line, files and defaults combine into the settings of a run |
| `convert.rs` | Running a conversion or a batch: loading the file, the `--find` preview, output checks, writing, progress, the thread-per-file work. The conversion itself is `crates/core`'s `convert` |
| `plan.rs`, `select.rs`, `report.rs` | Which inputs become which outputs, folder expansion and filters, the batch report |
| `source.rs`, `gif.rs` | Opening source files from disk (raster, animated GIF); decoding from memory is in the core |
| `ico_ops.rs`, `ico_validate.rs` | `--inspect`, `--merge`, `--extract`, `--select`, and the `--validate` parser |
| `util.rs` | Shared helpers: atomic file writing, thread helpers, the file context for messages |
| `crates/core` | Code that needs neither files nor threads, shared with the web version: the conversion itself (`convert`: background removal, crop, trim, fitting, scaling, encoding, for any source), decoding sources from memory (`source`), the `.ico` validator's parser, fitting, cropping, trimming and scaling (`layout`, `resize`), background detection and removal (`chroma_key`), SVG rendering (`vector`), `.icns` reading and writing (`icns`), warnings and the file context (`diag`), thread helpers (`par`) |
| `crates/wasm` | The WebAssembly bindings the web page calls |
| `web/` | The web page |

## How changes are made

Changes reach `main` through pull requests. Create a short-lived branch for one topic (for example `fix-gif-frame`), push it, and open a pull request. The automated checks run on the pull request, and `main` accepts it only when they have passed. Pull requests are squash-merged, so each one becomes a single commit on `main`. `main` therefore always holds what has been tested, and a release is a tag on a commit of `main`.

A change is finished when it has tests, the README and `--help` describe it, and `CHANGELOG.md` has an entry under `[Unreleased]`.

## The automated checks

The [CI workflow](.github/workflows/ci.yml) runs on every pull request and on every push to `main` that touches code (not for pushes that only change documentation or the web page), and it can be started by hand. A pull request that only changes files under `web/` or `docs/`, or Markdown files, skips the Rust checks below: they report as passed without doing any work (the protected `main` needs them to report). Any other file in a pull request makes them run. Once a pull request has passed the Rust checks, a later push that only changes web or documentation files does not run them again (the check looks at what that push changed, and only trusts it if the previous commit's Rust checks passed). The workflow

- checks the formatting,
- runs `clippy` and the full test suite on Windows, macOS and Linux,
- runs the tests with the minimum supported Rust version, and
- on a real macOS runner, checks both directions with Apple's own tools: a generated `.icns` is unpacked with `iconutil`, and files that Apple's encoders wrote (an `.icns` from `iconutil`; TIFF, JPEG, BMP, GIF and TGA from `sips`) are read back as source images.

## Releasing

1. In a pull request, set the new version in `Cargo.toml` (and `Cargo.lock`), turn the `[Unreleased]` section of `CHANGELOG.md` into `## [x.y.z] - date`, and update the version example in the README.
2. After it is merged, push a tag `vx.y.z` on the merged commit.

The [release workflow](.github/workflows/release.yml) then checks that the tagged commit is on `main` and that the tag, `Cargo.toml` and the changelog agree, runs the whole CI workflow as a gate, builds and smoke-tests the bundles for Windows, macOS and Linux, and publishes a GitHub Release whose description is that section of the changelog. A failing check blocks the release. Started by hand, it does everything except publishing — a rehearsal that produces the bundles as downloadable build artifacts.

## About this project

Developed in an extended pair-programming session with [Claude Sonnet 5](https://www.anthropic.com/claude) (Anthropic) — every feature, fix and piece of documentation in this repository went through iterative review and testing during that process.
