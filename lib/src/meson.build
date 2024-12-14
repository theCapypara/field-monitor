conf = configuration_data()
conf.set_quoted('VERSION', meson.project_version())
conf.set_quoted('APP_ID', get_option('app-id'))
conf.set_quoted('GETTEXT_PACKAGE', get_option('app-id'))
conf.set_quoted('LOCALEDIR', get_option('prefix') / get_option('localedir'))
conf.set_quoted('PKGDATADIR', get_option('prefix') / get_option('datadir') / meson.project_name())
conf.set_quoted('LIBEXECDIR', get_option('prefix') / get_option('libexecdir'))

pkgdatadir = get_option('prefix') / get_option('datadir') / meson.project_name()
blueprints = custom_target('blueprints',
  input: files(
    'gtk/save_credentials_button.blp',
  ),
  output: '.',
  command: [find_program('blueprint-compiler'), 'batch-compile', '@OUTPUT@', '@CURRENT_SOURCE_DIR@', '@INPUT@'],
)

gnome.compile_resources('field-monitor-lib',
  'field-monitor-lib.gresource.xml',
  gresource_bundle: true,
  install: true,
  install_dir: pkgdatadir,
  dependencies: blueprints,
)

configure_file(
    input: 'config.rs.in',
    output: 'config.rs',
    configuration: conf
)

# Copy the config.rs output to the source directory.
run_command(
  'cp',
  meson.current_build_dir() / 'config.rs',
  meson.current_source_dir() / 'config.rs',
  check: true
)
