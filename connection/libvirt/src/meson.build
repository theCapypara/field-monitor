pkgdatadir = get_option('prefix') / get_option('datadir') / meson.project_name()
blueprints = custom_target('blueprints',
  input: files(
    'qemu_preferences.blp',
  ),
  output: '.',
  command: [find_program('blueprint-compiler'), 'batch-compile', '@OUTPUT@', '@CURRENT_SOURCE_DIR@', '@INPUT@'],
)

gnome.compile_resources('field-monitor-connection-libvirt',
  'field-monitor-connection-libvirt.gresource.xml',
  gresource_bundle: true,
  install: true,
  install_dir: pkgdatadir,
  dependencies: blueprints,
)
