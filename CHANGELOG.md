# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/) for its
tags (`vMAJOR.MINOR.PATCH`) - note that the version tag and the `version`
field in `Cargo.toml` are maintained independently; keep them in sync by hand
when cutting a release.

## [Unreleased]

### Changed

- Release binaries are now built with link-time optimization, a single
  codegen unit, stripped symbols and `panic = "abort"` - about 30%
  smaller on Windows (1.93 MB to 1.33 MB). No behavior change.
- The GitHub Actions workflow now checks formatting (`cargo fmt`), lints
  with clippy (warnings are errors) and runs the full test suite on Windows,
  macOS and Linux before building; a failing test blocks the release.
- The project now uses the Rust 2024 edition and requires Rust 1.88 or
  newer to build from source (`rust-version` in `Cargo.toml`). Prebuilt
  binaries are unaffected.
- Dependencies are no longer pinned to old versions and were updated:
  `image` 0.25.10, `clap` 4.6, `toml` 1.1 and `ico` 0.5. `--out-toml`
  now writes standard double-quoted strings.
- `--chroma-key` and `--find` are considerably faster on large images
  (a 6000x6000 source: 0.75 s to 0.45 s and 0.48 s to 0.17 s, including
  loading). The results are identical - the colors are now compared once
  per pixel in whole numbers instead of repeatedly with square roots, and
  the background is filled row by row. No behavior change.
- Internal restructuring with no change in behavior: the conversion
  pipeline, settings resolution and mode handling are split into small,
  focused, separately tested modules, and needless copying of settings and
  pixel data was removed.
- The README was reorganized by task instead of by feature number, trimmed
  of duplicated and outdated passages (old dependency-pin note, an
  unverifiable audit claim) and extended with a contents line, exit codes,
  a scripting/CI section and a development section.

### Added

- An automated test suite: unit tests for every module plus end-to-end
  tests that run the real binary (254 tests in total).

## [1.3.0] - 2026-09-29

### Added

- A warning when a config file (`--config`/`img2ico.toml`) contains a
  setting name img2ico doesn't recognize - previously silently ignored
  (typically a typo, e.g. `toleranse` instead of `tolerance`), now
  flagged while the rest of the file's settings still apply normally.
- `--silent` to suppress advisory warnings and informational notices
  (upscaling/thin-content warnings, an out-of-range `--seed`, a skipped
  duplicate in `--merge`, the unknown-setting warning above, the "Using
  settings from ..." notice, etc.) - handy for scripted/CI use. Never
  suppresses errors or `--inspect`/`--find`'s own report output.
  Config-file-eligible like the other tuning settings.

## [1.2.0] - 2026-09-29

### Added

- `--gif-frame <n>` to pick which frame (1-indexed) of an animated GIF
  source becomes the icon - previously always the first frame, silently,
  with no way to choose or even know how many frames existed.
  Config-file-eligible like the other tuning settings. Has no effect on
  non-GIF input.
- `--inspect` on a GIF now reports its frame count.
- `--config <path>` to load default values for the "tuning" settings
  (sizes/preset, chroma-key/`--find` options, padding, grayscale,
  `--output-format`, `--force`, `--delete-source`, `--select`'s
  `--combine`/`--index`) from a TOML file - an explicit command-line flag
  still overrides the same setting from the file. If `--config` isn't
  given, an `img2ico.toml` in the current directory is picked up
  automatically if present (never combined with an explicit `--config` -
  one or the other, not both). Either way, a notice names the file that
  was used. Deliberately does not cover the mode
  (`--merge`/`--inspect`/`--extract`/`--select`), the input file(s), or
  `-o`/`--output`.
- `--out-toml <path>` to write the settings actually used for a run (CLI
  + config file + built-in defaults, fully resolved) back out as a TOML
  file - handy for turning a hand-tuned command line into a reusable
  `--config` file.
- An `examples/` folder with ready-to-use settings files (Windows app
  icon, favicon, macOS `.icns`, grayscale variant, automated/CI setup)
  plus a fully-commented reference covering every config-file field.

### Changed

- GitHub Releases now attach a single per-platform zip bundle
  (`img2ico-windows.zip` / `img2ico-macos.zip` / `img2ico-linux.zip`)
  instead of a bare binary - each contains the executable, `README.md`,
  `LICENSE`, and the `examples/` settings files, so a release download
  is self-contained.

## [1.1.0] - 2026-09-28

### Added

- `--find <hex color>` to automatically discover background regions the
  border-based `--chroma-key` flood fill can't reach on its own (the same
  situation `--seed` solves, but found by scanning the image instead of
  requiring pixel coordinates hunted down by hand in an image editor).
  Prints suggested `--seed` values and stops there by default; combined
  with the new `--auto-apply`, uses them automatically and runs the full
  conversion right away. Mutually exclusive with `--chroma-key`.
- A warning when a very elongated source image (e.g. an 8:1 banner) or a
  high `--padding` value (or both together) would leave only a thin,
  hard-to-recognize sliver of actual content at a given icon size, since
  img2ico never distorts a non-square source and instead pads the
  shorter side.
- `--replace-color <hex color>` to replace the detected background with
  a different solid color instead of making it transparent - same
  detection (border flood fill, `--tolerance`, `--seed`/`--find`) and the
  same soft edge blend, just blending towards the new color rather than
  towards transparent. Requires `--chroma-key` or `--find`.
- `--find-min-size <pixels>` to control the smallest region `--find`
  reports (default 9, matching the previous fixed behavior) - set to `1`
  to have `--find`/`--auto-apply` catch every matching pixel, including
  isolated single-pixel specks.
- `--grayscale` to remove all color from the icon (standard weighted
  luminance conversion), leaving transparency untouched. Works standalone
  or combined with anything else; runs after `--chroma-key`/
  `--replace-color`, so a replacement color ends up grayscaled too.

### Fixed

- A crash (panic, exit code 101) when this tool's output was piped into
  something that closes the pipe early, e.g. `img2ico --inspect big.ico
  | head`. Now exits cleanly instead - any other, genuine panic is
  unaffected and still shows normally.

## [1.0.0] - 2026-09-12

### Added

- README: a "Download a pre-built release" installation option alongside
  the existing build-from-source instructions, linking to the GitHub
  Releases page - recommended as the default path for anyone who doesn't
  need/want a Rust toolchain.

### Changed

- CI: `push`/`pull_request` runs now only trigger when something that
  actually affects the built program changes (`.rs` files, `Cargo.toml`,
  `Cargo.lock`, or the workflow file itself) - editing just the README,
  CHANGELOG, or LICENSE no longer spends CI minutes on a rebuild that
  would produce the same binaries. Tag pushes (`vX.Y.Z`, used for
  releases) are unaffected by this filter and always build regardless of
  which files changed.

## [0.1.0] - 2026-09-12

Initial release.

### Added

- Core conversion of PNG/JPG/BMP/GIF source images into Windows `.ico`
  files, always encoded as full 32-bit PNG (never the legacy, lower-quality
  BMP-with-reduced-palette format some tools fall back to).
- Alpha-aware ("premultiplied") resizing, so shrinking a transparent image
  never leaves a colored fringe around soft edges.
- `--chroma-key` background removal: flood-fill from the image border (so
  an identically-colored spot elsewhere in the artwork is left untouched),
  with a soft, anti-aliased transition at the edge instead of a hard cutout.
  `--tolerance` to tune the color match, `--seed` for background regions
  enclosed by a frame/ring that the border-based flood-fill can't reach on
  its own.
- `--padding` to leave a transparent margin around the artwork instead of
  filling the canvas edge-to-edge.
- `--sizes` for a custom size list, plus `--preset windows/favicon/minimal`
  for common size sets.
- `--output-format icns` for macOS icon export (Apple's full recommended
  size set, including 2x "Retina" variants), with an automatic
  platform-based default (`icns` on macOS, `ico` everywhere else) that can
  be overridden explicitly either way.
- A warning when a requested size exceeds the source image's resolution
  (upscaling can't add detail that isn't there).
- `--merge` to combine the icon entries of multiple existing `.ico` files
  into one.
- `--extract` to pull every size out of an existing `.ico` as separate
  PNGs.
- `--select` (with `--index` and `--combine`) to pull specific size(s) out
  of an existing `.ico` as standalone `.ico` file(s).
- `--inspect` to report on an existing `.ico`'s contents (sizes, color
  depth, PNG vs. legacy BMP encoding) or, given a plain source image
  instead, which standard sizes it can produce natively versus which would
  need upscaling.
- `--force` overwrite protection (refuses to clobber existing output
  unless explicitly told to) and `--delete-source` (removes the source
  file(s) after a successful run, with a safety net against deleting a
  source that turns out to also be the output path).
- Hardening: fixed a crash caused by slicing a `--chroma-key` value at an
  invalid UTF-8 byte boundary; verified graceful handling of malformed/
  corrupt `.ico` files and of oversized ("decompression bomb") source
  images; `cargo clippy` clean; `cargo audit` clean; property-based tests
  (`cargo test`, via `proptest`) covering the command-line input-parsing
  functions against arbitrary/adversarial input.
- Split into modules (`cli`, `chroma_key`, `resize`, `icns`, `ico_ops`,
  `util`) instead of one large file.
- MIT license.
- GitHub Actions workflow: native builds on Windows, macOS and Linux on
  every push/PR; an additional macOS job that generates a real `.icns`
  file and verifies it with Apple's own `iconutil`; and a release job
  that, on pushing a `vX.Y.Z` tag, publishes a GitHub Release with all
  three platform binaries attached.