pkgdatadir = get_option('prefix') / get_option('datadir') / meson.project_name()

blueprint_compiler = find_program('blueprint-compiler')

blueprints = custom_target('blueprints',
  input: files(
    'qemu_preferences.blp',
  ),
  output: '.',
  command: [blueprint_compiler, 'batch-compile', '@OUTPUT@', '@CURRENT_SOURCE_DIR@', '@INPUT@'],
)

gnome.compile_resources('field-monitor-connection-libvirt',
  'field-monitor-connection-libvirt.gresource.xml',
  gresource_bundle: true,
  install: true,
  install_dir: pkgdatadir,
  dependencies: blueprints,
)

blueprint_format_check_connection_libvirt = run_target(
  'blueprint-format-check-connection-libvirt',
  command: [
    blueprint_compiler,
    'format', '-s4',
    meson.current_source_dir()
  ]
)
