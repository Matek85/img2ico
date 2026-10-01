# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/) for its
tags (`vMAJOR.MINOR.PATCH`) - note that the version tag and the `version`
field in `Cargo.toml` are maintained independently; keep them in sync by hand
when cutting a release.

## [Unreleased]

## [1.7.0] - 2026-10-01

SVG as a source, rendered anew at every icon size, and WebP, TIFF, TGA and
macOS `.icns` files as further source formats.

### Added

- SVG as a source (`.svg`, compressed `.svgz`, also from standard input): the
  drawing is rendered anew at every icon size instead of being scaled down
  from one picture, which gives the sharpest small icons. Works for `.ico` and
  `.icns` output, in folders and batches, with `--padding` and `--grayscale`;
  `--inspect` reports it as a vector image. A background color can be removed
  or replaced (`--chroma-key`, `--replace-color`, also `auto`) - applied to
  every rendered size; `--seed` and `--find` name pixel positions and are
  refused for an SVG with a message. Text in an SVG is drawn with the fonts
  installed on the computer, so it can look different elsewhere; links to
  other files or the web are never followed.
- More source formats: WebP, TIFF (`.tif`, `.tiff`), TGA and macOS `.icns`
  files can be converted like PNG, JPG, BMP and GIF - from a file, from a
  folder (which now takes these too) and, except for TGA, from standard
  input. An `.icns` source uses its largest icon, including the old
  RLE-compressed icon types; `--inspect` reports all of them as source images.

### Changed

- The release binary grows by about 2.9 MB (2.81 MB to 5.7 MB on Windows):
  about 0.9 MB for the new image decoders (WebP, TIFF, TGA) and the `.icns`
  reader, about 2 MB for the SVG renderer.
- The GitHub repository is now called `img2ico`, like the program (it was
  `pic2ico`); links to the old name are redirected.

### Fixed

- An icon is never written over the image it is made from: an output that is
  the input file itself - for example an `.icns` converted to `.icns` next to
  itself, or `-o` naming the input - is refused, even with `--force`; in a
  batch, an output that is one of the inputs refuses the whole run before
  anything is written. (Before, `-o` naming the input quietly replaced it.)
- Output name collisions in a batch are now also found when one path is
  written with `/` and the other with `\` on Windows.

## [1.6.0] - 2026-10-01

Pipeline support (standard input and output, `--quiet`), automatic
background detection with `--feather` and removal warnings, folder
selection in settings files, and `--what-if` in place of `--dry-run`.

### Added

- Pipelines: `-` as the input reads the image from standard input
  (`cat logo.png | img2ico - -o icon.ico`), and `-o -` writes the icon to
  standard output (`img2ico logo.png -o - > icon.ico`), for `.ico` and
  `.icns`. With `-o -` all text - the `Done` line, warnings, notices - goes
  to standard error, so standard output holds only the icon; img2ico
  refuses to write binary data to a terminal. Both are for converting one
  image: they cannot be combined with several inputs, a folder, `--merge`,
  `--inspect`, `--extract` or `--select`, and `--delete-source` is refused.
  Reading standard input needs `-o`, except for a `--find` preview.
- `--quiet` / `-q`: hides the normal output of a run (the `Done` line, batch
  progress and summary, notices about settings files and written files).
  Errors, warnings, and the reports of `--inspect`, `--find` and `--what-if`
  stay. Together with `--silent` only errors remain.
- `--chroma-key auto` (and `--find auto`): img2ico takes the most common
  color along the image border as the background color. It prints the color
  it found, so it can be passed explicitly next time, and stops with a clear
  message naming its best guess when the border has no dominant color (a
  gradient, a photo) or is already fully transparent. In a batch every file
  detects its own background. `chroma-key = "auto"` also works in a settings
  file.
- `--feather` (0-100, default 50): how much of the tolerance range is the
  soft transition at the edge of a removed or replaced background. `0` is a
  hard edge. The default is the behavior img2ico has always had. Also
  available as `feather` in a settings file.
- Warnings when a chroma key removed nothing, almost nothing (under 0.5% of
  the image) or almost everything (over 98%). They are counted in the batch
  summary and in `--report`.
- `examples/auto-background.toml`.
- `recursive`, `include` and `exclude` can now be set in a settings file, so
  a folder batch does not need its filters typed every time. They only
  matter when an input is a folder and are otherwise ignored. A list on the
  command line replaces the file's list, like `seeds`.
- `examples/folder-batch.toml`: a ready-made settings file for converting a
  folder tree, with a note that it needs a folder as the input.

### Changed

- The release binary grows by about 150 KB (2.66 MB to 2.81 MB on
  Windows) because of the pipeline support, the background detection and
  the additional help text.
- `-o -` is no longer an error: it writes the icon to standard output (see
  Pipelines above). `-` as an input file name now means standard input; a file
  really called `-` is written `./-`.
- `--dry-run` is now called `--what-if`, and its closing line starts with
  "What if:". The old name keeps working as an alias.

### Fixed

- A source image is recognized by its content, not by its file extension: a PNG
  without an extension, or saved as `.jpg`, is now read correctly instead of
  failing with "The image format could not be determined" (or being decoded
  with the wrong decoder).

### Documentation

- The README describes the order in which settings combine - built-in defaults,
  per-user file, project file (`img2ico.toml` or `--config`), command line -
  with a worked example, and says that the current directory, not the
  program's own folder, is searched.
- `--include` and `--exclude` are explained in the README and in `--help`:
  what a pattern is matched against, the wildcards (`*`, `?`, `[abc]`,
  `{a,b}`, `**`) and worked examples.

## [1.5.0] - 2026-10-01

The batch release: several files or whole folder trees in one run, with file
selection, naming, progress, a rehearsal mode and reports; plus settings
layering, `--verbose`, `--version` details and shell completions.

### Added

- Batch conversion: give several input files, or a folder, and they are
  all converted in one run with the same settings. `-o` then names an
  output folder (created if missing); without it each icon is written next
  to its input. A folder contributes the PNG, JPG, BMP and GIF files
  directly inside it, in name order (subfolders with `--recursive`, below). Inputs
  that would produce the same output file are refused before anything is
  written.
- `--recursive` / `-r`: search the subfolders of an input folder too, all
  the way down, in a reproducible order. A folder with images only in
  subfolders now says to add it.
- `--include` and `--exclude` (repeatable glob patterns): narrow down which
  files a folder contributes. A pattern without a `/` matches the file name,
  one with a `/` the path below the folder; matching ignores case. Files
  named directly on the command line are never filtered.
- `--keep-structure`: with `-o`, rebuild the input folder's subfolders below
  the output folder instead of putting every icon directly into it.
- `--name`: build the icons' file names from a pattern with `{stem}`,
  `{ext}` and `{format}`, for example `--name "{stem}-app"`.
- Output name collisions - now also between same-named files in different
  folders - are still refused before anything is written, and the message
  points at `--keep-structure` and `--name`.
- Every file of a batch gets a numbered progress line
  (`[3/20] assets/logo.png -> icons/logo.ico (42 KB)`); the summary also
  counts the warnings and gives the combined size and the time taken.
  `--silent` hides progress and the summary of a clean run, but not the
  summary of a run with failures.
- `--dry-run`: shows, for every input, the icon a run would write, the sizes
  and what an existing output would mean (skipped, replaced or refused),
  without writing anything - not even `--out-toml` - and without deleting
  sources. Options and name collisions are checked as in a real run.
- `--report FILE`: writes a record of the run as CSV or JSON (by file
  extension): per file the output, status, size, icon sizes, warning count,
  duration and message, plus totals. It is written after failures too.
- `--inspect --json`: the inspection as one JSON array, for scripts.
- The batch options are refused in `--merge`, `--extract`, `--select` and
  `--inspect`, and the folder options without a folder, instead of being
  silently ignored. None of them can be set in a settings file.
- New dependencies: `walkdir`, `globset` and `serde_json`.
- `--keep-going`: in a batch, carry on after a file fails. Without it the
  first failure stops the run and names the file. A batch with any failure
  exits with code 1 and a count; `--delete-source` and `--out-toml` then do
  nothing.
- `--skip-existing`: leave an input alone whose output already exists, so
  a batch can be repeated and only converts what is missing. Works for a
  single file too and contradicts `--force`.
- Both new options can also be set in a settings file (`keep-going`,
  `skip-existing`).
- In a batch, warnings and `--verbose` lines name the file they are about.
- `--version` prints the version together with the platform and the
  compiler the binary was built with - what a bug report needs. `-V`
  prints just the version.
- An optional per-user settings file (Windows:
  `%APPDATA%\img2ico\config.toml`, macOS:
  `~/Library/Application Support/img2ico/config.toml`, elsewhere:
  `$XDG_CONFIG_HOME/img2ico/config.toml` or
  `~/.config/img2ico/config.toml`). Settings now layer, setting by
  setting: command line, then the project file
  (`--config` or `img2ico.toml`), then the user file, then the built-in
  defaults. Every file used is named in a notice.
- `--no-config` ignores every settings file for one run. It cannot be
  combined with `--config`.
- `--verbose` / `-v` prints diagnostics to standard error: the settings
  actually in effect, the source image, the output, each generated size
  with its byte count, and timings. Standard output is unchanged; it
  cannot be combined with `--silent`.
- `--completions <shell>` prints a tab-completion script for bash, zsh,
  fish, PowerShell or elvish, generated from the program's own option
  definitions.
- README: a privacy statement (no network access, no telemetry) and a
  description of how the settings files layer.

### Changed

- Several input files without `--merge` are no longer an error: they are
  now converted as a batch (see above). `--merge`, `--extract` and
  `--select` keep their own rules for their inputs.
- The release binary grows by about 1.1 MB (1.57 MB to 2.66 MB on
  Windows) because of the completion generator, the batch mode with its
  glob pattern matching, the JSON output and the additional help text.

### Fixed

- `-o -` (the usual spelling for standard output, which is not supported)
  no longer creates a file literally named `-` and reports success; it is
  now an error for every mode. `-o ./-` still writes a file with that name.

## [1.4.0] - 2026-09-30

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