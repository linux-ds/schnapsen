# Schnapsen

Schnapsen is a native GNOME implementation of the Austrian two-player card game. It uses Rust, GTK 4, and libadwaita. It is maintained by the [Linux DS Team](https://linux-ds.org/). Source code and issues are hosted on [GitHub](https://github.com/linux-ds/schnapsen).

**Status (0.5.1):** Playable prototype in active development. The game includes soft and sharp Schnapsen with [documented house rules](RULE_AUDIT.md), three computer difficulties, automatic save and resume, and German and English localization. It supports mouse, keyboard, and touch input. Double German is the default card face. French faces and card backs can be selected independently.

## Release notes

Each version has a features section and a known limitations section in both languages. [English changelog](CHANGELOG.md) · [Deutsches Änderungsprotokoll](CHANGELOG.de.md)

## Install and update

Once the release is available, download the Linux DS repository file and run the commands below from the download folder. [linux-ds.flatpakrepo](build-aux/linux-ds.flatpakrepo)

```sh
flatpak remote-add --user --if-not-exists linux-ds ./linux-ds.flatpakrepo
flatpak install --user linux-ds org.linuxds.Schnapsen
```

Update an installed copy through your software center or with `flatpak update org.linuxds.Schnapsen`.

## Build and run

The supported build environment is **GNOME SDK 50** with the stable Rust extension and the matching GNOME Platform runtime.

- **GNOME Builder:** Open the project folder, select `org.linuxds.Schnapsen.json` as the build configuration, then select **Build** and **Run**. Allow Builder to fetch the required Flatpak runtimes and Cargo sources on the first build.
- **Flatpak CLI:** With the SDK, runtime, Rust extension, and `flatpak-builder` available, run the following commands from the repository root.

  ```sh
  flatpak-builder .flatpak-build org.linuxds.Schnapsen.json
  flatpak-builder --run .flatpak-build org.linuxds.Schnapsen.json /app/bin/schnapsen
  ```

The Flatpak build runs the Cargo test suite through Meson. See the [development plan](PLAN.md) for the Builder-based terminal workflow and the roadmap.

## Linux DS integration build and export

On the host, use GNOME Builder, Flatpak, OSTree, and Python 3.11 or newer.
Install GNOME Platform 50, GNOME SDK 50, and the Rust SDK extension for
Freedesktop 25.08 from Flathub. Run the integration wrapper from the checkout
with a new or empty output directory.

```sh
python3 build-aux/build-flatpak.py target/linux-ds-flatpak --state-dir .flatpak-builder
```

The wrapper builds the manifest and checks the composed AppStream catalog and
PNG icons before export. It imports the resulting `x86_64/master` bundle into an
isolated OSTree repository, verifies its contents, and writes `verification.json`
next to the bundle. This produces an unsigned local integration artifact.
The signing procedure is documented in [SPEC.md](SPEC.md#flatpak-release-für-linux-ds).

## Tests

```sh
cargo fmt --check
cargo test --locked --no-default-features
cargo clippy --locked --no-default-features --all-targets -- -D warnings
```

The headless tests require Rust and Cargo. Run `cargo test --locked` in the GNOME SDK to include the graphical application's tests. The [specification](SPEC.md) documents the simulation and replay tools.

## Licensing

| Material | License |
|---|---|
| Project source code, including the card renderers, and project documentation | [GPL-3.0-or-later](LICENSE) |
| Original card faces and backs rendered by [the card designs](src/ui/design/), and the [application icon](data/org.linuxds.Schnapsen.svg) | [CC BY-SA 4.0](LICENSES/CC-BY-SA-4.0.txt) |
| [AppStream metadata](data/org.linuxds.Schnapsen.metainfo.xml.in) | [CC0 1.0](LICENSES/CC0-1.0.txt) |

Credit the original artwork to **Schnapsen contributors** and include a link to the repository and the license when reusing it. Upstream dependencies retain their own licenses and notices.
