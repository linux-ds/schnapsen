project('schnapsen', version: '0.1.0')

app_id = 'org.linuxds.Schnapsen'
cargo = find_program('cargo')
fs = import('fs')
i18n = import('i18n')
cargo_env = {'SCHNAPSEN_LOCALEDIR': get_option('prefix') / get_option('localedir')}
cargo_config = '/app/share/schnapsen/cargo/config.toml'
cargo_flags = ['--locked']
if fs.is_file(cargo_config)
  cargo_flags += ['--offline', '--config', cargo_config]
endif

custom_target(
  'schnapsen',
  output: 'schnapsen',
  command: [
    find_program('sh'), '-ec',
    '"$@" && cp cargo-target/release/schnapsen "$0"',
    '@OUTPUT@', cargo, 'build', '--release',
    '--manifest-path', meson.project_source_root() / 'Cargo.toml',
    '--target-dir', 'cargo-target',
  ] + cargo_flags,
  build_always_stale: true,
  env: cargo_env,
  install: true,
  install_dir: get_option('bindir'),
)

test(
  'cargo test',
  cargo,
  args: ['test', '--release', '--manifest-path', meson.project_source_root() / 'Cargo.toml',
         '--target-dir', meson.project_build_root() / 'cargo-target'] + cargo_flags,
  env: cargo_env,
)

subdir('po')

i18n.merge_file(
  input: 'data/' + app_id + '.desktop.in',
  output: app_id + '.desktop',
  type: 'desktop',
  po_dir: 'po',
  install: true,
  install_dir: get_option('datadir') / 'applications',
)
i18n.merge_file(
  input: 'data/' + app_id + '.metainfo.xml.in',
  output: app_id + '.metainfo.xml',
  po_dir: 'po',
  install: true,
  install_dir: get_option('datadir') / 'metainfo',
)
install_data('data/' + app_id + '.svg',
  install_dir: get_option('datadir') / 'icons/hicolor/scalable/apps')

install_data(
  'data/' + app_id + '.gschema.xml',
  install_dir: get_option('datadir') / 'glib-2.0/schemas',
)
gnome = import('gnome')
gnome.post_install(glib_compile_schemas: true)
