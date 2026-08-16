pkgdatadir = get_option('prefix') / get_option('datadir') / meson.project_name()

blueprint_compiler = find_program('blueprint-compiler')

blueprints = custom_target('blueprints',
  input: files(
    'credential_preferences.blp',
    'server_preferences.blp',
    'preferences.blp',
  ),
  output: '.',
  command: [blueprint_compiler, 'batch-compile', '@OUTPUT@', '@CURRENT_SOURCE_DIR@', '@INPUT@'],
)

gnome.compile_resources('field-monitor-connection-generic-group',
  'field-monitor-connection-generic-group.gresource.xml',
  gresource_bundle: true,
  install: true,
  install_dir: pkgdatadir,
  dependencies: blueprints,
)

blueprint_format_check_connection_generic_group = run_target(
  'blueprint-format-check-connection-generic-group',
  command: [
    blueprint_compiler,
    'format', '-s4',
    meson.current_source_dir()
  ]
)
