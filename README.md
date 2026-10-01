# img2ico

A command-line tool that converts any image into a Windows `.ico` file (or a macOS `.icns` file), with proper transparency support, background removal, and a few extra tools for working with existing icon files.

Built in Rust: a single, dependency-free binary — no runtime to install, nothing to configure.

**Contents:** [Features](#features) · [Installation](#installation) · [Quick start](#quick-start) · [Command reference](#command-reference) · [Converting an image](#converting-an-image) · [Several files at once](#converting-several-files-at-once) · [Background color](#removing-or-replacing-a-background-color) · [macOS icons](#macos-icons) · [Existing .ico files](#working-with-existing-ico-files) · [Overwrite protection](#overwrite-protection-and-cleanup) · [Settings files](#settings-files) · [Scripting and CI](#scripting-and-ci) · [Recipes](#recipes) · [Quality and safety](#quality-and-safety) · [Development](#development)

## Features

- **Convert** PNG, JPG, BMP and GIF to `.ico` with full 32-bit color and a clean alpha channel — never a reduced-color legacy format. Size presets, padding, grayscale, and frame selection for animated GIFs.
- **Batch conversion:** several files, or a whole folder tree, in one run with the same settings — with file filters, name patterns, a progress line per file, a rehearsal mode (`--dry-run`), a CSV or JSON report, and a choice of stopping at the first failure or carrying on.
- **Remove or replace a background color** ("chroma key") with a soft, anti-aliased edge instead of a hard cutout, including background areas enclosed by the artwork — found automatically with `--find`.
- **macOS `.icns`** from the same source image (chosen automatically as the default when running on macOS).
- **Work with existing `.ico` files:** inspect them, merge several into one, extract every size as PNG, or pull out specific sizes as standalone `.ico` files.
- **Safe and scriptable:** overwrite protection and source-file cleanup (both opt-in), reusable settings files, quiet mode, and clear exit codes.
- **Helpful warnings** when a source image is too small for the requested sizes or would leave only a sliver of visible artwork.

## Installation

### Download a pre-built release (recommended)

Every [GitHub Release](https://github.com/Matek85/pic2ico/releases) has a ready-to-run bundle for Windows, macOS and Linux — no Rust toolchain, no compiling:

1. Open the latest release and download `img2ico-windows.zip`, `img2ico-macos.zip` or `img2ico-linux.zip`.
2. Extract it. Each bundle contains the binary (`img2ico.exe` / `img2ico`), this README, the license, and the [`examples/`](examples/) settings files.
3. On macOS/Linux, mark the binary executable once: `chmod +x img2ico`.

The bundles are built automatically by this project's GitHub Actions workflow from the tagged source (see [Development](#development)).

**First-run warnings.** The binaries are not code-signed, so your operating system may object the first time:

- **Windows:** SmartScreen shows "Windows protected your PC". Click "More info" → "Run anyway".
- **macOS:** Gatekeeper may refuse a binary downloaded from the internet. Allow it once in System Settings → Privacy & Security — or build it yourself (below); a binary you compile locally isn't subject to this check.

### Build from source

You need Rust **1.88 or newer** (the project uses the 2024 edition). Install it via [rustup](https://rustup.rs) if you don't have it.

```
git clone https://github.com/Matek85/pic2ico.git
cd pic2ico
cargo build --release
```

The binary is at `target/release/img2ico` (`img2ico.exe` on Windows). The source is platform-neutral — the same command builds it on Windows, macOS and Linux.

### Check the installation

```
img2ico --version
```

```
img2ico 1.5.0
target:   x86_64-pc-windows-msvc
compiler: rustc 1.97.1 (8bab26f4f 2026-07-14)
```

`-V` prints just the first line. The platform and compiler lines are what a bug report needs, so please include them.

### Tab completion (optional)

`--completions <shell>` prints a completion script for `bash`, `zsh`, `fish`, `powershell` or `elvish`, generated from the program's own option definitions so it always matches your version. Save it where your shell looks for completions, for example:

```
# bash
img2ico --completions bash > ~/.local/share/bash-completion/completions/img2ico

# fish
img2ico --completions fish > ~/.config/fish/completions/img2ico.fish

# PowerShell: add this line to your profile ($PROFILE)
img2ico --completions powershell | Out-String | Invoke-Expression
```

For zsh, put the output of `img2ico --completions zsh` in a file named `_img2ico` in a directory on your `fpath`.

## Quick start

```
img2ico logo.png
```

That's all. This creates `logo.ico` next to `logo.png`, with the default sizes 16, 32, 48, 64, 128 and 256. (On macOS the default output is `logo.icns` instead — see [macOS icons](#macos-icons).)

> **About the examples:** this document writes the command as `img2ico`. On Windows that is `img2ico.exe` (or just `img2ico` from the folder containing it); on macOS/Linux it is `./img2ico` when running the binary from where you built or extracted it, or plain `img2ico` once it's on your `PATH`. The flags are identical on all platforms.

## Command reference

```
Usage: img2ico [OPTIONS] <INPUT>...
```

| Flag | Short | Value | Purpose |
|---|---|---|---|
| `--output` | `-o` | path | Where to write the result: a file, or a folder for a batch, `--extract` and `--select` into several files; `-o -` (standard output) is not supported |
| `--sizes` | `-s` | comma list | Icon sizes to generate (default `16,32,48,64,128,256`) |
| `--preset` | | `windows` / `favicon` / `minimal` | A predefined size set, instead of `--sizes` |
| `--padding` | | 0–100 (default 0) | Transparent margin around the artwork, in percent |
| `--grayscale` | | | Remove all color, keep only brightness |
| `--gif-frame` | | number (default 1) | Which frame of an animated GIF to use |
| `--chroma-key` | `-c` | hex color | Remove this background color |
| `--tolerance` | `-t` | 0–100 (default 20) | How strictly `--chroma-key`/`--find` match colors |
| `--seed` | | `x,y` (repeatable) | Extra starting point(s) for background removal |
| `--find` | | hex color | Discover seed points for that color automatically |
| `--find-min-size` | | pixels (default 9) | Smallest region `--find` reports; `1` catches every matching pixel |
| `--auto-apply` | | | With `--find`: use the discovered points and convert right away |
| `--replace-color` | | hex color | Replace the background with this color instead of making it transparent |
| `--output-format` | | `ico` / `icns` | Force an icon container format |
| `--merge` | | | Combine several `.ico` files into one |
| `--extract` | | | Save every size of an `.ico` as a PNG |
| `--select` | | | Pull specific size(s) out of an `.ico` as `.ico` file(s) |
| `--index` | | comma list | Which size(s) `--select` pulls out (default `0`) |
| `--combine` | | | With `--select` and several indices: one file instead of several |
| `--inspect` | | | Print a report about `.ico` file(s) or source image(s) |
| `--force` | `-f` | | Allow overwriting existing output |
| `--skip-existing` | | | Leave an input alone whose output already exists (contradicts `--force`) |
| `--keep-going` | | | In a batch, carry on after a file fails instead of stopping |
| `--recursive` | `-r` | | Also search the subfolders of an input folder |
| `--include` | | glob (repeatable) | Only take folder files whose name (or path, with a `/`) matches |
| `--exclude` | | glob (repeatable) | Leave out folder files whose name (or path) matches |
| `--keep-structure` | | | With `-o`: rebuild the input folder's subfolders below it |
| `--name` | | pattern | Name the icons by a pattern with `{stem}`, `{ext}`, `{format}` |
| `--dry-run` | | | Show what would happen; write nothing (contradicts `--report`) |
| `--report` | | `.csv` / `.json` file | Write a record of the run: one line per file, plus totals |
| `--json` | | | With `--inspect`: print the report as JSON |
| `--delete-source` | | | Delete the input file(s) after a successful run |
| `--config` | | path | Load default settings from a TOML file |
| `--no-config` | | | Ignore every settings file (cannot be combined with `--config`) |
| `--out-toml` | | path | Save the settings used for this run as a TOML file |
| `--silent` | | | Suppress warnings and notices |
| `--verbose` | `-v` | | Print diagnostic details to stderr (cannot be combined with `--silent`) |
| `--completions` | | shell | Print a tab-completion script and exit |
| `--version` | `-V` | | Print the version, platform and compiler (`-V`: just the version) |
| `--help` | `-h` | | Full built-in help |

**Modes.** `--merge`, `--inspect`, `--extract`, `--select` and `--output-format` are mutually exclusive: each changes what the tool does with its input, so only one can be active per run. Everything else can be combined freely. The exclusivity check looks at what you actually typed, not at the automatic macOS default — so `--merge` works normally on a Mac without ever mentioning `--output-format`.

## Converting an image

### Output path and sizes

```
img2ico logo.png -o build/icons/app.ico
img2ico logo.png -s 16,32,256
```

`-o` sets where the result goes (the target folder must already exist). `-s` generates exactly the sizes you list — handy for a smaller file. Valid `.ico` sizes are 1–256; anything else is skipped with a notice.

### Presets

```
img2ico logo.png --preset windows
```

| Preset | Sizes |
|---|---|
| `windows` | 16, 20, 24, 32, 40, 48, 64, 96, 128, 256 — Microsoft's recommended set, so Windows never has to stretch a nearby size at any DPI scaling level |
| `favicon` | 16, 32, 48 |
| `minimal` | 16, 32 |

`--preset` completely replaces `--sizes`.

### Padding

```
img2ico logo.png --preset favicon --padding 10
```

Leaves roughly a 10% transparent margin on every side instead of filling the canvas edge to edge — useful when the artwork already touches its own edges and looks cramped as a small icon. Padding is applied after any background removal.

### Grayscale

```
img2ico logo.png --grayscale
```

Removes all color using the standard weighted luminance formula (green counts most, blue least, matching how the eye perceives brightness). Transparency is untouched. It always runs last, so a `--replace-color` color is grayscaled too rather than being an exception in an otherwise gray icon.

### Animated GIFs

```
img2ico --inspect animation.gif
img2ico animation.gif --gif-frame 2 -o icon.ico
```

An icon can't be animated, so exactly one frame is converted: frame 1 by default, or the one you pick with `--gif-frame` (numbered from 1). `--inspect` tells you how many frames a GIF has. For any other format, `--gif-frame` prints a warning and is ignored.

### Non-square images and warnings

A non-square source is never distorted: it is fitted into the square canvas proportionally, with transparent margins on the shorter side. Two situations trigger a warning on stderr. Neither is an error, and the icon is still created.

**Upscaling** — the source is smaller than a requested size, so that size will look soft:

```
Warning: the source image is 64x64 pixels, smaller than [96, 128, 256] - those sizes will be upscaled and may look soft or blurry rather than sharp. For crisp results at every size, use a higher-resolution source image.
```

**A thin sliver of content** — an elongated source (an 800×100 banner leaves only about 2 pixels of height at 16×16) or a high `--padding` leaves very little visible artwork:

```
Warning: at these sizes, only a thin sliver of the actual artwork will be visible: 16x16 (~2px of actual content), 20x20 (~3px of actual content) - this can be caused by an elongated source image, a high --padding value, or both. Consider a less elongated source image and/or less padding if that looks too thin.
```

To check a source image before converting it, use `--inspect` (see [Inspect](#inspect)).

## Converting several files at once

Give more than one input — or a folder — and img2ico converts them all in one run, with the same settings for every file:

```
img2ico a.png b.png c.png                 # icons next to each input
img2ico a.png b.png c.png -o icons/       # icons collected in a folder
img2ico assets/ -o icons/                 # every image in a folder
img2ico --preset windows --padding 8 assets/ -o icons/
img2ico assets/ -r -o icons/ --keep-structure   # the whole tree, folders preserved
```

**What gets converted.** A file stands for itself. A folder stands for the PNG, JPG, BMP and GIF files directly inside it, in name order; other files are ignored. Add `--recursive` (`-r`) to take the subfolders too, all the way down — folder by folder, each in name order, so a run is reproducible. A folder with no images is an error rather than a silent no-op (and if the images are in subfolders, the error says to add `--recursive`). Files and folders can be mixed; files are converted in the order you give them.

**Choosing files.** `--include` and `--exclude` narrow down what a *folder* contributes; both can be repeated, use the usual glob patterns (`*`, `?`, `[abc]`, and `**` for any number of folders) and ignore upper/lower case:

```
img2ico assets/ -r --include "*.png" --exclude "*_old*"
img2ico assets/ -r --exclude "backup/**"        # a pattern with a / matches the path below the folder
```

A pattern without a `/` is matched against the file name wherever the file sits; a pattern with a `/` against its path below the input folder (always written with forward slashes, on Windows too). A file is taken if it matches at least one `--include` (when there are any) and no `--exclude`. The filters only ever narrow down the supported image types, they cannot add others. A file you name directly on the command line is never filtered away — you asked for it by name. If the filters leave nothing, that is an error.

**Where the icons go.** In a batch, `-o` names a **folder** (created if it doesn't exist), and each icon is called after its input: `logo.png` becomes `icons/logo.ico` (or `.icns` with `--output-format icns`). Without `-o`, each icon is written next to its input. If `-o` names an existing file, or looks like a file name such as `out.ico`, img2ico says so instead of quietly creating a folder with that name. To convert one file to a specific file name, give just that one input as before.

**Keeping the folder structure.** By default every icon goes straight into the output folder. `--keep-structure` rebuilds the subfolders instead: with `img2ico assets/ -r -o icons/ --keep-structure`, `assets/ui/save.png` becomes `icons/ui/save.ico`, and the subfolders are created as needed. It needs `-o` (without it, icons already sit next to their inputs, in whatever folder that is).

**Naming the icons.** `--name` sets the icons' file names by a pattern. The variables are `{stem}` (the input's name without its extension), `{ext}` (its extension, in lowercase) and `{format}` (`ico` or `icns`); `{{` and `}}` give literal braces, and the icon's own extension is added after the pattern:

```
img2ico assets/ -o icons/ --name "{stem}-app"        # logo.png  ->  icons/logo-app.ico
img2ico logo.png logo.jpg --name "{stem}-{ext}"      # logo-png.ico and logo-jpg.ico
```

The pattern is a file name, not a path (use `-o` and `--keep-structure` for folders), an unknown variable is an error, and it cannot be combined with `-o` naming a single output *file*. It also works for one input file.

**Name clashes are caught first.** If two inputs would produce the same output file — say `logo.png` and `logo.jpg`, or `one/logo.png` and `two/logo.png` collected into one folder — the whole run is refused before anything is written, naming both and pointing at `--keep-structure` and `--name`, which are the two ways to tell them apart.

**When a file fails.** By default the run stops at the first failure, names the file and exits with code 1. With `--keep-going` the other files are still converted, each failure is reported as it happens, and the run ends with exit code 1 and a count of what failed:

```
img2ico assets/ -o icons/ --keep-going
```

```
[1/3] assets/app.png -> icons/app.ico (3 KB)
[2/3] Error: assets/broken.png: Could not read input file: Format error decoding Png: Invalid PNG signature.
[3/3] assets/logo.png -> icons/logo.ico (3 KB)
Batch finished: 2 converted, 0 skipped, 1 failed (3 file(s) in total).
Wrote 2 icon file(s), 6 KB in all, in 41 ms.
1 of 3 file(s) failed.
```

**Progress and summary.** With more than one file every line starts with a counter — `[3/20]` — and a converted file shows where its icon went and how large it is. At the end comes a summary: how many files were converted, skipped and failed, how many warnings there were (when there were any), the combined size of the icons and how long it took. (A single file just prints its `Done: …` line.) `--silent` hides the progress lines and the summary of a clean run; if anything failed, the summary and the errors are still printed.

**Repeating a run.** `--skip-existing` leaves an input alone whose output already exists, so a batch can be run again and only converts what is missing. Existing outputs are never touched, and the skip is reported (silenced by `--silent`). It also works for a single file, and it contradicts `--force`, which overwrites instead — using both is an error, wherever each comes from.

```
img2ico assets/ -o icons/ --skip-existing
```

**Rehearsal.** `--dry-run` shows what a run *would* do and writes nothing — no icons, no output folder, no `--report`, no `--out-toml`, and `--delete-source` deletes nothing. Every input gets one line with the icon it would produce, the sizes, and what an already existing output would mean:

```
img2ico assets/ -r -o icons/ --keep-structure --dry-run --skip-existing
```

```
[1/3] assets/logo.png -> icons/logo.ico (would convert, sizes [16, 32, 48, 64, 128, 256])
[2/3] assets/ui/old.png -> icons/ui/old.ico (would skip: the output already exists)
[3/3] assets/ui/save.png -> icons/ui/save.ico (would convert, sizes [16, 32, 48, 64, 128, 256])
Dry run: 2 would be converted, 1 skipped, 0 would fail. Nothing was written.
```

The options are checked and name clashes are looked for exactly as in a real run, so a rehearsal that passes is a good sign. It does not open the images, so it cannot know about a damaged file; a missing input file, and an existing output that would be refused (without `--force` or `--skip-existing`), are reported as `would fail`, and the exit code is then 1. A `--find` preview is not part of a rehearsal. It also works for a single file.

**Reports.** `--report FILE` writes a record of the run, in the format the file name's extension names — `.csv` or `.json` (anything else is refused, so a typo can never overwrite an image):

```
img2ico assets/ -r -o icons/ --keep-going --report report.csv
img2ico assets/ -r -o icons/ --keep-going --report report.json
```

Each input has a line with its output, `status` (`converted`, `skipped` or `failed`), `size_bytes` of the icon, the `sizes` inside it, the number of `warnings` it raised, `duration_ms` and, for a failure or a skip, a `message`. The CSV has a header line, with the sizes separated by spaces; the JSON is `{"summary": {…totals…}, "files": [ … ]}`. The report is written even when files failed — that is when it is most useful — and an existing report file is replaced. With the default stop at the first failure the report holds the files up to and including the failed one. Warnings are counted in the report even with `--silent`.

**Good to know**

- **Settings apply to every file.** That includes `--seed`, which holds pixel coordinates of one particular image and so only fits a batch of images that share a layout.
- **Warnings say which file they are about** (`Warning: logo.png: the source image is …`). For a single file the messages are unchanged.
- **`--delete-source` and `--out-toml` act only after a fully successful batch.** If any file failed, no source is deleted — even those that were converted — and no settings snapshot is written. A skipped input is never deleted.
- **`--find` without `--auto-apply`** prints a report for one image and is refused for a batch; add `--auto-apply` (each file then gets its own discovery), or run it per file.
- **`--merge`, `--extract`, `--select` and `--inspect`** keep their own rules for their inputs; the batch options of this section (`--recursive`, `--include`, `--exclude`, `--keep-structure`, `--name`, `--dry-run`, `--report`) are refused there rather than quietly ignored. The same goes for the folder options given without a folder, and for `--keep-structure` without `-o`.
- **These options are command-line only.** Which files, what they are called and what gets reported changes with every run, so none of `--recursive`, `--include`, `--exclude`, `--keep-structure`, `--name`, `--dry-run`, `--report` and `--json` can be set in a settings file.

## Removing or replacing a background color

### Basic removal

```
img2ico logo.png -c 00C800 -o icon.ico
img2ico logo.png -c 00C800 -t 30
```

Starting from the image border, img2ico finds the connected region of the given color (`#00C800`; with or without `#`) and makes it transparent, with a soft edge instead of a jagged cutout. Pixels well inside the tolerance become fully transparent; pixels near its limit become partially transparent, so the edge blends smoothly.

`--tolerance` (default 20) controls how similar a shade must be to count as background. A higher value copes with gradients, JPEG artifacts and anti-aliasing, but too high a value eats into the artwork.

Only pixels *connected to the border* are affected, so the same color appearing inside the artwork stays untouched.

### Areas enclosed by the artwork

```
img2ico logo.png -c FFFFFF --seed 128,64
img2ico logo.png -c FFFFFF --seed 128,64 --seed 200,300
```

If the background color also appears *inside* the artwork (say, inside a ring), the border search can't reach it. `--seed x,y` adds a starting point there. Use pixel coordinates from your **original** image, for example by hovering in an image editor. It can be given several times.

### Finding seed points automatically

```
img2ico --find FFFFFF logo.png
```

```
Found 2 additional region(s) matching FFFFFF that the border-based flood fill can't reach on its own:
  ~450 pixel(s) near (128, 64) -> --seed 128,64
  ~120 pixel(s) near (300, 310) -> --seed 300,310
Re-run with these as --seed values, or add --auto-apply to use them automatically.
```

`--find` scans for exactly the regions `--seed` is for and prints them as ready-to-use values. On its own it only prints this report: nothing is converted or written, so no `-o` or `--sizes` is needed. It uses the same `--tolerance` and is mutually exclusive with `--chroma-key` (its color takes over that role).

To skip the copy-paste step, add `--auto-apply`: the discovered regions are used as seeds and the conversion runs right away. Manual `--seed` values can still be added on top.

```
img2ico --find FFFFFF --auto-apply logo.png -o icon.ico
```

Regions smaller than 9 pixels are ignored by default, since a few stray pixels are almost always anti-aliasing or JPEG noise. Change that with `--find-min-size`; with `1`, nothing matching is left out, however small:

```
img2ico logo.png --find FFFFFF --find-min-size 1 --auto-apply
```

### Replacing instead of removing

```
img2ico logo.png -c FFFFFF --replace-color 000000
img2ico logo.png --find FFFFFF --auto-apply --replace-color FF0000
```

Swaps the detected background for a solid color (for example white to black, to match a dark theme) instead of making it transparent. Detection, tolerance, seeds and the soft edge work exactly as above, blending towards the new color instead of towards transparency. Alpha is left untouched. Requires `--chroma-key` or `--find`.

### Combining options

All of these options combine freely with each other and with sizes, presets, padding and `--output-format`:

```
img2ico logo.png -c 00C800 -t 25 --seed 128,64 --preset windows --padding 10 -o icon.ico --force
```

## macOS icons

`.icns` output is selected with `--output-format`:

```
img2ico logo.png --output-format icns
img2ico logo.png --output-format icns -o build/AppIcon.icns
```

It contains Apple's full recommended set — 16, 32, 64, 128, 256, 512 and 1024 pixels, including the 2× "Retina" variants — regardless of `--sizes` and `--preset`, which are ignored for `.icns`. Everything that affects the image itself (`--chroma-key`, `--tolerance`, `--seed`, `--padding`, `--grayscale`, …) applies as usual.

**Automatic default.** Without `--output-format`, img2ico picks the format of the platform the binary was *built for*: `icns` on macOS, `ico` everywhere else. The same command therefore does the right thing on every platform. To get a Windows `.ico` from a Mac build, pass `--output-format ico`.

> The `.icns` container was verified byte by byte against the public specification, cross-checked with an independent parser, and — in this project's GitHub Actions workflow — unpacked with Apple's own `iconutil` on a real macOS runner. A test on your own Mac is still worthwhile before relying on it for anything important.

## Working with existing .ico files

### Inspect

```
img2ico --inspect icon.ico
img2ico --inspect one.ico two.ico logo.png
```

For an `.ico`, the report lists every size with its index, color depth, whether it is stored as PNG or the legacy BMP format, and warnings about common problems (reduced color depth, missing standard Windows sizes). The `[index]` is what `--select` expects.

```
icon.ico:
  [0]   16x16    32bpp  PNG  1008 bytes
  [1]   20x20    32bpp  PNG  1422 bytes
  ...
```

For a regular source image, it shows the resolution and which standard sizes the image covers **natively** versus which would need **upscaling** — a good check before you commit to a large icon:

```
logo.png (source image, 64x64):
  Windows sizes this image covers natively: [16, 20, 24, 32, 40, 48, 64]
  Windows sizes that would need upscaling (may look soft/blurry): [96, 128, 256]
  macOS (.icns) sizes this image covers natively: [16, 32, 64]
  macOS (.icns) sizes that would need upscaling (may look soft/blurry): [128, 256, 512, 1024]
  tip: for consistently sharp icons at every common size, a source of at least 256x256 (1024x1024 if you also need .icns) is recommended.
```

For a GIF it also reports the frame count. Each file is inspected with whichever report fits it, and nothing is written or changed.

**For scripts: `--json`.** `img2ico --inspect icon.ico logo.png --json` prints the same information as JSON — always one array with an entry per file, even for a single file. Standard output then holds nothing but the JSON; if a file cannot be read, nothing is printed and the error goes to standard error.

```json
[
  {
    "path": "icon.ico",
    "kind": "ico",
    "entries": [
      { "index": 0, "width": 16, "height": 16, "bits_per_pixel": 32, "format": "png", "bytes": 1008 }
    ],
    "missing_windows_sizes": [20, 24, 40, 96, 128, 256],
    "warnings": []
  },
  {
    "path": "logo.png",
    "kind": "image",
    "width": 64,
    "height": 64,
    "frames": null,
    "windows": { "native": [16, 20, 24, 32, 40, 48, 64], "upscaled": [96, 128, 256] },
    "macos": { "native": [16, 32, 64], "upscaled": [128, 256, 512, 1024] }
  }
]
```

`frames` is the frame count for a GIF and `null` for every other image; `warnings` lists the same sanity warnings as the text report.

### Merge

```
img2ico --merge small.ico large.ico -o combined.ico
```

Combines the sizes of two or more `.ico` files into one. `-o` is required, since there's no single obvious name to derive. If a size appears in several inputs, the first occurrence wins and later ones are skipped with a warning. Every entry is re-encoded as PNG, so the result has consistent full quality even if a source used an older format.

### Extract

```
img2ico --extract icon.ico
img2ico --extract icon.ico -o pngs/
```

Saves every size as a PNG (`icon_16x16.png`, `icon_32x32.png`, …) into `icon_extracted/` next to the input, or into the directory given with `-o` (created if needed — in this mode `-o` names a directory, not a file).

### Select

```
img2ico --select icon.ico                          # the first icon (index 0)
img2ico --select icon.ico --index 3                # one specific size
img2ico --select icon.ico --index 0,3,5            # separate files in icon_selected/
img2ico --select icon.ico --index 0,3,5 --combine  # one multi-size icon_selected.ico
```

The ICO-to-ICO counterpart of `--extract`: re-exports chosen sizes as standalone `.ico` files. Run `--inspect` first to see which index is which size. `-o` is a file path whenever exactly one file is written (a single index, or `--combine`) and a directory when several separate files are written:

```
img2ico --select icon.ico --index 3 -o app-icon.ico
img2ico --select icon.ico --index 0,3,5 -o chosen/
```

## Overwrite protection and cleanup

Opt-in flags that work the same way in every mode.

```
img2ico logo.png -o icon.ico            # first run: fine
img2ico logo.png -o icon.ico            # second run: refused
img2ico logo.png -o icon.ico --force    # explicitly allowed
```

The refusal is an error (`'icon.ico' already exists. Use --force to overwrite it.`), so the first result is never silently destroyed. For `--extract` and `--select` into a directory, *every* target file is checked before anything is written, so a conflict never leaves a half-written folder behind. If you would rather leave an existing result alone than fail, use `--skip-existing` instead (see [Several files at once](#converting-several-files-at-once)).

```
img2ico logo.png -o icon.ico --delete-source
img2ico --merge small.ico large.ico -o combined.ico --delete-source
```

`--delete-source` removes the input file(s) only **after** the output has been written completely — if anything fails first, nothing is deleted (in a batch: if any file fails). If a source resolves to the same file as the output, that file is kept (with a warning), since deleting it would destroy the very result just created.

## Settings files

With more than twenty flags, typing the same combination each time gets old. A TOML settings file sets defaults for the "tuning" options: sizes or preset, the chroma-key/`--find` options, padding, grayscale, GIF frame, `--output-format`, `--force`, `--skip-existing`, `--keep-going`, `--delete-source`, `--silent`, and `--select`'s `--combine`/`--index`. **Not** covered: the input file(s), `-o`, the mode (`--merge`/`--inspect`/`--extract`/`--select`), and the options that choose files, names and reporting for a batch (`--recursive`, `--include`, `--exclude`, `--keep-structure`, `--name`, `--dry-run`, `--report`, `--json`) — those change with every run. An explicit command-line flag always wins over a file; a file only fills in what you didn't type. Where the files sit, and how they layer, is described under [Which settings files apply](#which-settings-files-apply).

### TOML in brief

A settings file is just `key = value` lines:

```toml
# Lines starting with # are comments.

tolerance = 20                  # a number: no quotes
grayscale = true                # true or false: no quotes
chroma-key = "FFFFFF"           # text needs double quotes
seeds = ["128,64", "200,300"]   # a list: square brackets, comma-separated
```

Keys match the long flag names without the leading `--` (so `chroma-key` for `--chroma-key`). The full language is described at [toml.io](https://toml.io/), but the four lines above cover everything img2ico uses.

### Saving, reusing and overriding settings

```
img2ico logo.png -c FFFFFF --preset windows --padding 10 -o icon.ico --out-toml my-settings.toml
```

Converts as usual and also writes the settings that were actually used — what you typed plus anything a config file contributed, with defaults filled in:

```toml
preset = "windows"
chroma-key = "FFFFFF"
tolerance = 20
seeds = []
find-min-size = 9
auto-apply = false
grayscale = false
padding = 10
gif-frame = 1
delete-source = false
force = false
combine = false
silent = false
```

Settings with no value (`find`, `replace-color`, `output-format`, `index`, `sizes` here) are left out. On/off settings are always written, since "off" can't otherwise be told apart from "not set".

```
img2ico other-logo.png --config my-settings.toml -o other-icon.ico
img2ico logo.png --config my-settings.toml --padding 0 -o icon.ico
```

The first command loads the file as defaults. The second shows that a flag overrides just that one setting (`--padding 0` here) while everything else still comes from the file.

A notice names the file that was used, so behavior never changes silently:

```
Using settings from 'my-settings.toml'.
```

### Which settings files apply

Up to two files can contribute, plus the built-in defaults. From highest to lowest priority:

1. **The command line.**
2. **The project file:** the one named with `--config`, or else a file called exactly `img2ico.toml` in the current directory — handy for a project folder where you always want the same settings. An explicit `--config` replaces this lookup entirely; the two are never merged.
3. **The per-user file**, for your own defaults everywhere:

   | Platform | Location |
   |---|---|
   | Windows | `%APPDATA%\img2ico\config.toml` |
   | macOS | `~/Library/Application Support/img2ico/config.toml` |
   | Linux and others | `$XDG_CONFIG_HOME/img2ico/config.toml`, or `~/.config/img2ico/config.toml` |

4. **The built-in defaults.**

The layers combine setting by setting, not all or nothing: a project file that only sets `sizes` still gets `padding` from your user file. A setting that takes a list (`seeds`) is replaced by the higher layer's list, not added to, and on/off settings (`grayscale`, `force`, …) are on if any layer turns them on. Every file that was used is named in a notice, so nothing changes silently.

`--no-config` ignores all settings files for one run — handy to reproduce a result or when an automatically found file gets in the way. To see what the layers add up to, use `--verbose` (below): it prints the settings actually in effect.

### Typos are reported

```
Warning: unknown setting 'toleranse' in 'my-settings.toml' - ignored. Check for a typo, or see `img2ico --help` for the exact setting names.
```

An unrecognized key is almost always a typo, so it is flagged. The conversion still proceeds with all valid settings, as if the line weren't there.

### Ready-made examples

The [`examples/`](examples/) folder has ready-to-use settings files for a Windows app icon, a favicon, a macOS `.icns`, a grayscale variant, and a hands-off setup for scripted/CI use. `reference-all-settings.toml` documents every possible setting in one place; it isn't meant to be used as-is, because several of its settings deliberately contradict each other — copy individual lines from it instead.

## Scripting and CI

**Quiet mode.** `--silent` suppresses every advisory warning and notice: upscaling and thin-content warnings, an out-of-range `--seed`, a skipped duplicate in `--merge`, the unknown-setting warning, the "Using settings from …" notice, and so on. It does **not** suppress:

- **Errors** — something going wrong always reaches you, and the exit code.
- **`--inspect` and `--find` reports** — they are the requested output of those modes.
- **The final "Done: … created" line** of a single conversion.
- **A batch's failure summary** — in a batch, `--silent` hides the per-file progress lines and the summary of a clean run, but if anything failed, the summary and the errors are printed.

`--silent` can also be set in a settings file, with one nuance: it only takes effect once that file has been loaded, so it can never hide a warning about the file itself.

**Machine-readable output.** `--report report.json` (or `.csv`) records a conversion run file by file, and `--inspect --json` prints an inspection as JSON; see [Several files at once](#converting-several-files-at-once) and [Inspect](#inspect). Both leave standard output free of anything else you did not ask for. `--dry-run` lets a script check what a run would do before doing it.

**Diagnostics.** `--verbose` (`-v`) prints details to standard error while it works: the settings actually in effect after all layers are combined (in the same form `--out-toml` writes), the source image, the output, background removal, each generated size with its byte count, and how long the steps took. Standard output is unchanged, so scripts keep working. It is command-line only and cannot be combined with `--silent`.

**Exit codes.**

| Code | Meaning |
|---|---|
| `0` | Success |
| `1` | The run failed (unreadable input, refused overwrite, invalid option value, …); the reason is printed to stderr |
| `2` | The command line itself is invalid (missing input, unknown flag, options that can't be combined) |

`--help`, `--version` and `--completions` exit with `0`.

**Pipes.** Piping output into a program that closes early (for example `img2ico --inspect big.ico | head`) exits cleanly instead of showing a panic. The reports of `--inspect` and `--find` go to standard output and warnings and errors to standard error, so `grep` and friends see only the report.

img2ico reads and writes files only: it does not read an image from standard input, and `-o -` (write to standard output) is rejected with an error rather than creating a file called `-`. To really write a file named `-`, use `-o ./-`.

`examples/automated-pipeline.toml` is a ready-made starting point for build scripts.

## Recipes

**A logo with a plain background → a ready-to-ship Windows icon, replacing any earlier attempt:**
```
img2ico logo.png -c FFFFFF -t 20 --preset windows --padding 8 -o dist/app.ico --force
```

**Windows and macOS icons from one source (two runs, since each run writes one format):**
```
img2ico logo.png -c FFFFFF --preset windows -o dist/app.ico
img2ico logo.png -c FFFFFF --output-format icns -o dist/app.icns
```

**Pull the two largest sizes out of a vendor-supplied `.ico` and clean up:**
```
img2ico --inspect vendor-icon.ico
img2ico --select vendor-icon.ico --index 8,9 --combine -o our-icon.ico --delete-source
```

**Convert a whole folder of logos, keeping what is already done and carrying on past a broken file:**
```
img2ico assets/ -o icons/ --preset windows --skip-existing --keep-going
```

**Convert a whole asset tree, keep its folders, and keep a record:**
```
img2ico assets/ -r -o icons/ --keep-structure --exclude "backup/**" --keep-going --report report.json
```

**Look before you leap — what would a run do?**
```
img2ico assets/ -r -o icons/ --keep-structure --dry-run
```

**Combine two teams' icons into one shared file:**
```
img2ico --merge team-a.ico team-b.ico -o shared.ico --force --delete-source
```

**Check a source before committing to a large icon**, then proceed anyway if a slightly soft large size is acceptable:
```
img2ico --inspect logo.png
img2ico logo.png --output-format icns --force
```

## Quality and safety

- Every icon size is PNG-encoded at full color depth — never the older, lower-quality BMP-with-reduced-palette format. Whenever the image has any transparency, the PNG carries a full alpha channel; a fully opaque icon is stored without one, since there is nothing to preserve.
- Resizing is alpha-aware (premultiplied), so shrinking a transparent image doesn't leave a colored fringe around soft edges.
- Destructive operations are opt-in and fail safe: nothing is overwritten without `--force`, and nothing is deleted unless the run succeeded.
- **Privacy:** img2ico works entirely on your machine. It never opens a network connection, has no telemetry, no update check and uploads nothing; it only reads the files you name and writes the files you ask for.
- Input parsing is hardened against malformed and adversarial input and covered by property-based tests.
- Every change is checked by the automated test suite, `rustfmt` and `clippy` (warnings are errors) on Windows, macOS and Linux — see [Development](#development).

## Development

```
cargo test                        # unit tests and end-to-end tests that run the real binary
cargo fmt --check                 # formatting
cargo clippy --all-targets -- -D warnings
```

The test suite includes property-based tests that compare the optimized chroma-key and `--find` code against a deliberately naive reference implementation on random images, so speed-ups can't silently change results.

**Continuous integration.** The [GitHub Actions workflow](.github/workflows/main.yml) runs the formatting check, clippy and the full test suite on Windows, macOS and Linux. It runs only when a version tag is pushed or when started manually ("Run workflow"), not on every push.

**Releasing.** Pushing a tag like `v1.5.0` runs the workflow, builds the bundles for all three platforms, verifies the `.icns` output with Apple's `iconutil`, and publishes a GitHub Release. A failing check blocks the release.

> Developed in an extended pair-programming session with [Claude Sonnet 5](https://www.anthropic.com/claude) (Anthropic) — every feature, fix, and piece of documentation in this repo went through iterative review and testing during that process.

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for a version-by-version history of what changed.

## License

MIT — see [LICENSE](LICENSE) for the full text.
