pkgdatadir = get_option('prefix') / get_option('datadir') / meson.project_name()
blueprints = custom_target('blueprints',
  input: files(
    'credential_preferences.blp',
    'preferences.blp',
  ),
  output: '.',
  command: [find_program('blueprint-compiler'), 'batch-compile', '@OUTPUT@', '@CURRENT_SOURCE_DIR@', '@INPUT@'],
)

gnome.compile_resources('field-monitor-connection-proxmox',
  'field-monitor-connection-proxmox.gresource.xml',
  gresource_bundle: true,
  install: true,
  install_dir: pkgdatadir,
  dependencies: blueprints,
)
