pkgdatadir = get_option('prefix') / get_option('datadir') / meson.project_name()

blueprint_compiler = find_program('blueprint-compiler')

blueprints = custom_target('blueprints',
  input: files(
    'credential_preferences.blp',
    'preferences.blp',
  ),
  output: '.',
  command: [blueprint_compiler, 'batch-compile', '@OUTPUT@', '@CURRENT_SOURCE_DIR@', '@INPUT@'],
)

gnome.compile_resources('field-monitor-connection-proxmox',
  'field-monitor-connection-proxmox.gresource.xml',
  gresource_bundle: true,
  install: true,
  install_dir: pkgdatadir,
  dependencies: blueprints,
)

blueprint_format_check_connection_proxmox = run_target(
  'blueprint-format-check-connection-proxmox',
  command: [
    blueprint_compiler,
    'format', '-s4',
    meson.current_source_dir()
  ]
)
