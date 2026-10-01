# Contributing to img2ico

This file is for people who want to build, change or release img2ico. If you only want to use it, the [README](README.md) is all you need.

## Building and testing

You need Rust **1.88 or newer** (the project uses the 2024 edition); [rustup](https://rustup.rs) installs it.

```
cargo build --release             # the binary: target/release/img2ico
cargo test                        # unit tests and end-to-end tests that run the real binary
cargo fmt --check                 # formatting
cargo clippy --all-targets -- -D warnings
```

Use a current stable toolchain for `clippy`: its rules grow with every release, and the automated checks use the newest stable. `Cargo.toml` names the oldest supported Rust version (`rust-version`); the checks run the tests with exactly that version too, so keep both in step.

On Windows the build embeds the program's icon (`assets/img2ico.ico`) in the `.exe` with the system's resource compiler (part of the Windows SDK that the Rust toolchain needs anyway); if it is missing the build still succeeds, with a warning, just without the icon.

The tests come in three kinds:

- **Unit tests** next to the code they test.
- **End-to-end tests** in `tests/cli.rs`, which start the real binary on temporary files and check what it prints, writes and exits with.
- **Property-based tests** that compare the optimized chroma-key and `--find` code against a deliberately naive reference implementation on random images, so speed-ups cannot silently change results.

## Where things are

| Module | What it does |
|---|---|
| `main.rs`, `cli.rs` | The program entry, the mode dispatch and the command-line definition (the `--help` texts live in `cli.rs`) |
| `config.rs`, `settings.rs` | Settings files, and how command line, files and defaults combine into the settings of a run |
| `convert.rs` | The conversion of one image or a batch: loading, background removal, layout, encoding, writing, the thread-per-file work |
| `plan.rs`, `select.rs`, `report.rs` | Which inputs become which outputs, folder expansion and filters, the batch report |
| `source.rs`, `vector.rs`, `gif.rs`, `icns.rs` | Reading source images (raster, SVG, animated GIF, `.icns`) and writing `.icns` |
| `layout.rs`, `resize.rs`, `chroma_key.rs` | Fitting, cropping, trimming and scaling; background detection and removal |
| `ico_ops.rs`, `ico_validate.rs` | `--inspect`, `--merge`, `--extract`, `--select`, and the `--validate` parser |
| `util.rs` | Shared helpers: atomic file writing, thread helpers, the file context for messages |

## How changes are made

Changes reach `main` through pull requests. Create a short-lived branch for one topic (for example `fix-gif-frame`), push it, and open a pull request. The automated checks run on the pull request, and `main` accepts it only when they have passed. Pull requests are squash-merged, so each one becomes a single commit on `main`. `main` therefore always holds what has been tested, and a release is a tag on a commit of `main`.

A change is finished when it has tests, the README and `--help` describe it, and `CHANGELOG.md` has an entry under `[Unreleased]`.

## The automated checks

The [CI workflow](.github/workflows/ci.yml) runs on every pull request and on every push to `main` that touches code (not for pushes that only change documentation), and it can be started by hand. It

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
