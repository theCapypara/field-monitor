pkgdatadir = get_option('prefix') / get_option('datadir') / meson.project_name()
gnome = import('gnome')

blueprints = custom_target('blueprints',
  input: files(
    'gtk/help-overlay.blp',
    'widget/add_connection_dialog.blp',
    'widget/authenticate_connection_dialog.blp',
    'widget/update_connection_dialog.blp',
    'widget/window.blp',
    'widget/connection_list.blp',
    'widget/connection_list/connection_entry.blp',
    'widget/connection_view.blp',
  ),
  output: '.',
  command: [find_program('blueprint-compiler'), 'batch-compile', '@OUTPUT@', '@CURRENT_SOURCE_DIR@', '@INPUT@'],
)

conf = configuration_data()
conf.set_quoted('VERSION', meson.project_version())
conf.set_quoted('APP_ID', get_option('appid'))
conf.set_quoted('APP_ID_SLASHED', get_option('appid').replace('.', '/'))
conf.set_quoted('GETTEXT_PACKAGE', get_option('appid'))
conf.set_quoted('LOCALEDIR', get_option('prefix') / get_option('localedir'))
conf.set_quoted('PKGDATADIR', pkgdatadir)

configure_file(
    input: 'field-monitor.gresource.xml.in',
    output: 'field-monitor.gresource.xml',
    configuration: conf
)

# Copy the field-monitor.gresource.xml output to the source directory.
run_command(
  'cp',
  meson.project_build_root() / 'src' / 'field-monitor.gresource.xml',
  meson.project_source_root() / 'src' / 'field-monitor.gresource.xml',
  check: true
)

configure_file(
    input: 'config.rs.in',
    output: 'config.rs',
    configuration: conf
)

# Copy the config.rs output to the source directory.
run_command(
  'cp',
  meson.project_build_root() / 'src' / 'config.rs',
  meson.project_source_root() / 'src' / 'config.rs',
  check: true
)

gnome.compile_resources('field-monitor',
  'field-monitor.gresource.xml',
  gresource_bundle: true,
  install: true,
  install_dir: pkgdatadir,
  dependencies: blueprints,
)

cargo_bin  = find_program('cargo')
cargo_opt  = [ '--manifest-path', meson.project_source_root() / 'Cargo.toml' ]
cargo_opt += [ '--target-dir', meson.project_build_root()  / 'src' ]
cargo_env  = [ 'CARGO_HOME=' + meson.project_build_root()  / 'cargo-home' ]

if get_option('buildtype') == 'release'
  cargo_opt += [ '--release' ]
  rust_target = 'release'
else
  cargo_opt += [ '--features', 'devel' ]
  rust_target = 'debug'
endif

cargo_build = custom_target(
  'cargo-build',
  build_by_default: true,
  build_always_stale: true,
  output: get_option('appid'),
  console: true,
  install: true,
  install_dir: get_option('bindir'),
  command: [
    'env', cargo_env,
    cargo_bin, 'build',
    cargo_opt, '&&', 'cp', 'src' / rust_target / meson.project_name(), '@OUTPUT@',
  ]
)
