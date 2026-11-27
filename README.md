# Schnapsen

Schnapsen is a native GNOME implementation of the Austrian two-player card game. It uses Rust, GTK 4, and libadwaita. It is maintained by the [Linux DS Team](https://linux-ds.org/). Source code and issues are hosted on [GitHub](https://github.com/linux-ds/schnapsen).

**Status (0.5.0):** Playable prototype in active development. The game includes soft and sharp Schnapsen with [documented house rules](RULE_AUDIT.md), three computer difficulties, automatic save and resume, and German and English localization. It supports mouse, keyboard, and touch input. Double German is the default card face. French faces and card backs can be selected independently.

## Build and run

The supported build environment is **GNOME SDK 51** with the stable Rust extension and the matching GNOME Platform runtime.

- **GNOME Builder:** Open the project folder, select `org.linuxds.Schnapsen.json` as the build configuration, then select **Build** and **Run**. Allow Builder to fetch the required Flatpak runtimes and Cargo sources on the first build.
- **Flatpak CLI:** With the SDK, runtime, Rust extension, and `flatpak-builder` available, run the following commands from the repository root.

  ```sh
  flatpak-builder .flatpak-build org.linuxds.Schnapsen.json
  flatpak-builder --run .flatpak-build org.linuxds.Schnapsen.json /app/bin/schnapsen
  ```

The Flatpak build runs the Cargo test suite through Meson. See the [development plan](PLAN.md) for the Builder-based terminal workflow and the roadmap.

## Linux DS Flatpak releases and updates

The release ref is `app/org.linuxds.Schnapsen/x86_64/master`, following
Monthly's `x86_64/master` convention. App updates come from
`https://update.linux-ds.org/flatpak/`. Flathub supplies the GNOME 51 runtime.

From the host, build the app with the manifest above, then export the finished
build to a **local staging copy of the existing Linux DS repository**. On the
authorized release machine, use the same signing setup as Monthly. The public
repository key fingerprint is `E4C9EF78D422A21FA20DB7A7BBFF7B34B6DF591B`.

```sh
flatpak build-export --arch=x86_64 --no-update-summary \
    --gpg-sign=E4C9EF78D422A21FA20DB7A7BBFF7B34B6DF591B \
    /path/to/staging-repo .flatpak-build master
flatpak build-update-repo --generate-static-deltas \
    --gpg-sign=E4C9EF78D422A21FA20DB7A7BBFF7B34B6DF591B /path/to/staging-repo
sh build-aux/build-flatpak-bundle.sh /path/to/staging-repo \
    org.linuxds.Schnapsen-x86_64-master.flatpak
```

The bundle embeds the app repository URL, its public key, and the Flathub
runtime source. For a metadata-only local check, export to a scratch repository
without signing. Distribute bundles only from a signed release export.

To enable updates, publish the new ref, its signed commit, and the refreshed
AppStream and signed summary through the existing Linux DS repository deployment.
Keep the existing refs, including Monthly. Verify the published ref with
`flatpak remote-info linux-ds org.linuxds.Schnapsen//master`. A standalone
bundle download alone does not publish the ref.

### Switch an older bundle with the same app ID to Linux DS

After the release is available, quit Schnapsen and back up
`~/.var/app/org.linuxds.Schnapsen/`. Use the same Flatpak installation scope as
the current bundle. For a user installation, run these commands from this
checkout. For a system installation, replace `--user` with `--system`.

```sh
flatpak remote-add --user --if-not-exists linux-ds build-aux/linux-ds.flatpakrepo
flatpak remote-info --user linux-ds org.linuxds.Schnapsen//master
flatpak install --user --reinstall linux-ds org.linuxds.Schnapsen//master
flatpak info --user --show-origin org.linuxds.Schnapsen//master
```

The last command should show `linux-ds`. Reinstalling this same app ID preserves
the app data directory, including saves and settings. Leave that directory in
place and avoid `flatpak uninstall --delete-data`. Future updates use
`flatpak update org.linuxds.Schnapsen` in the same installation scope.

## Data from an earlier Flatpak installation

The Linux DS app ID gives the Flatpak its own data directory under
`~/.var/app/org.linuxds.Schnapsen/`. The previous installation keeps its files.
For a manual transfer, quit both versions and create the new destination directories as needed. Copy the former sandbox's
`data/schnapsen/` directory into the new sandbox's `data/` directory if the
destination is empty. To transfer settings, copy the old app's
`config/glib-2.0/settings/keyfile` to the same relative location in the new sandbox
and change its Schnapsen section header to `[org/linuxds/Schnapsen]`. Preserve any
settings or save files already created in the new sandbox. For a native build,
the XDG save path stays the same and the GSettings schema path changes.

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
