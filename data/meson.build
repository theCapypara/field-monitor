conf = configuration_data()
conf.set_quoted('APP_ID', get_option('app-id'))

desktop_file = i18n.merge_file(
        input: configure_file(
            input: 'de.capypara.FieldMonitor.desktop.in.in',
            output: 'de.capypara.FieldMonitor.desktop.in',
            configuration: conf,
        ),
       output: get_option('app-id') + '.desktop',
         type: 'desktop',
       po_dir: '../po',
      install: true,
  install_dir: get_option('datadir') / 'applications'
)

desktop_utils = find_program('desktop-file-validate', required: false)
if desktop_utils.found()
  test('Validate desktop file', desktop_utils, args: [desktop_file])
endif

appstream_file = i18n.merge_file(
        input: configure_file(
            input: 'de.capypara.FieldMonitor.metainfo.xml.in.in',
            output: 'de.capypara.FieldMonitor.metainfo.xml.in',
            configuration: conf,
        ),
       output: get_option('app-id') + '.xml',
       po_dir: '../po',
      install: true,
  install_dir: get_option('datadir') / 'metainfo'
)

appstreamcli = find_program('appstreamcli', required: false, disabler: true)
test('Validate appstream file', appstreamcli,
     args: ['validate', '--no-net', '--explain', appstream_file])

install_data('de.capypara.FieldMonitor.gschema.xml',
  install_dir: get_option('datadir') / 'glib-2.0' / 'schemas',
  rename: get_option('app-id') + '.gschema.xml'
)

compile_schemas = find_program('glib-compile-schemas', required: false, disabler: true)
test('Validate schema file',
     compile_schemas,
     args: ['--strict', '--dry-run', meson.current_source_dir()])

subdir('icons')
