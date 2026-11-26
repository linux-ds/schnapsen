# Schnapsen

Schnapsen is a native GNOME implementation of the Austrian two-player card game. It uses Rust, GTK 4, and libadwaita.

**Status:** Playable prototype in active development. The game includes soft and sharp Schnapsen with [documented house rules](RULE_AUDIT.md), three computer difficulties, automatic save and resume, and German and English localization. It supports mouse, keyboard, and touch input. Card faces and backs can be selected independently.

## Build and run

The supported build environment is **GNOME SDK 51** with the stable Rust extension and the matching GNOME Platform runtime.

- **GNOME Builder:** Open the project folder, select `io.github.schnapsen.Schnapsen.json` as the build configuration, then select **Build** and **Run**. Allow Builder to fetch the required Flatpak runtimes and Cargo sources on the first build.
- **Flatpak CLI:** With the SDK, runtime, Rust extension, and `flatpak-builder` available, run the following commands from the repository root.

  ```sh
  flatpak-builder .flatpak-build io.github.schnapsen.Schnapsen.json
  flatpak-builder --run .flatpak-build io.github.schnapsen.Schnapsen.json /app/bin/schnapsen
  ```

The Flatpak build runs the Cargo test suite through Meson. See the [development plan](PLAN.md) for the Builder-based terminal workflow and the roadmap.

## Tests

```sh
cargo fmt --check
cargo test --locked --no-default-features
cargo clippy --locked --no-default-features --all-targets -- -D warnings
```

The headless tests require Rust and Cargo. Run `cargo test --locked` in the GNOME SDK to include the graphical application's tests. The [specification](SPEC.md) documents the simulation and replay tools.

## License status

A project license has yet to be selected. The AppStream file declares CC0-1.0 for the metadata. The project icon has no separate license declaration. See [CONTRIBUTING.md](CONTRIBUTING.md) for development guidelines.
