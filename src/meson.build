pkgdatadir = get_option('prefix') / get_option('datadir') / meson.project_name()
gnome = import('gnome')

blueprints = custom_target('blueprints',
  input: files(
    'gtk/help-overlay.blp',
    'widget/add_connection_dialog.blp',
    'widget/app_status.blp',
    'widget/authenticate_connection_dialog.blp',
    'widget/preferences.blp',
    'widget/quick_connect_dialog.blp',
    'widget/update_connection_dialog.blp',
    'widget/window.blp',
    'widget/connection_list/connection_stack.blp',
    'widget/connection_list/connection_list_navbar.blp',
    'widget/connection_list/info_page.blp',
    'widget/connection_list/server_group.blp',
    'widget/connection_list/server_info/icon.blp',
    'widget/connection_view/connection_tab_view.blp',
    'widget/connection_view/connection_view_navbar.blp',
    'widget/connection_view/server_screen.blp',
    'quick_connect/preferences.blp',
  ),
  output: '.',
  command: [find_program('blueprint-compiler'), 'batch-compile', '@OUTPUT@', '@CURRENT_SOURCE_DIR@', '@INPUT@'],
)

conf = configuration_data()
conf.set_quoted('APP_ID_SLASHED', get_option('app-id').replace('.', '/'))

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

gnome.compile_resources('field-monitor',
  'field-monitor.gresource.xml',
  gresource_bundle: true,
  install: true,
  install_dir: pkgdatadir,
  dependencies: blueprints,
)

cargo_bin  = find_program('cargo')
cargo_src_opt  = [ '--manifest-path', meson.project_source_root() / 'Cargo.toml' ]
cargo_opt  = cargo_src_opt + [ '--target-dir', meson.project_build_root()  / 'src' ]
cargo_env  = [ 'CARGO_HOME=' + meson.project_build_root()  / 'cargo-home' ]

if get_option('buildtype') == 'release'
  cargo_opt += [ '--release', '--features', 'prod' ]
  rust_target = 'release'
else
  cargo_opt += [ '--features', 'devel' ]
  rust_target = 'debug'
endif

cargo_build = custom_target(
  'cargo-build',
  build_by_default: true,
  build_always_stale: true,
  output: get_option('app-id'),
  console: true,
  install: true,
  install_dir: get_option('bindir'),
  command: [
    'env', cargo_env,

    cargo_bin, 'build',
    cargo_opt,
    '&&', 'cp', 'src' / rust_target / meson.project_name(), '@OUTPUT@',
  ]
)

cargo_check_clippy = run_target(
  'cargo-check-clippy',
  command: [
    'env', cargo_env,

    cargo_bin,
    'clippy',
    cargo_opt,
    '--', '-D', 'warnings'
  ]
)

cargo_check_fmt = run_target(
  'cargo-check-fmt',
  command: [
    'env', cargo_env,

    cargo_bin,
    'fmt', '--verbose',
    cargo_src_opt,
    '--', '--check'
  ]
)
