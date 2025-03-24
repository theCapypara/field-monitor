conf = configuration_data()
conf.set('APP_ID', get_option('app-id'))
conf.set_quoted('APP_ID_SLASHED', '/' + get_option('app-id').replace('.', '/') + '/'
)

# Field Monitor has limited support for reading Virt-Viewer / RDP files.
# This installs the MIME types in case they don't exist yet.
i18n.merge_file (
        input: 'de.capypara.FieldMonitor.mime.virt-viewer.xml.in',
       output: get_option('app-id') + '.mime.virt-viewer.xml',
       po_dir: podir,
      install: true,
    data_dirs: itsdir,
  install_dir: get_option('datadir') / 'mime' / 'packages'
)
i18n.merge_file (
        input: 'de.capypara.FieldMonitor.mime.rdp.xml.in',
       output: get_option('app-id') + '.mime.rdp.xml',
       po_dir: podir,
      install: true,
    data_dirs: itsdir,
  install_dir: get_option('datadir') / 'mime' / 'packages'
)

desktop_file = i18n.merge_file(
        input: configure_file(
            input: 'de.capypara.FieldMonitor.desktop.in.in',
            output: 'de.capypara.FieldMonitor.desktop.in',
            configuration: conf,
        ),
       output: get_option('app-id') + '.desktop',
         type: 'desktop',
       po_dir: podir,
      install: true,
  install_dir: get_option('datadir') / 'applications'
)

desktop_utils = find_program('desktop-file-validate', required: true)
test('Validate desktop file', desktop_utils, args: [desktop_file])

appstream_file = i18n.merge_file(
        input: configure_file(
            input: 'de.capypara.FieldMonitor.metainfo.xml.in.in',
            output: 'de.capypara.FieldMonitor.metainfo.xml.in',
            configuration: conf,
        ),
       output: get_option('app-id') + '.metainfo.xml',
       po_dir: podir,
      install: true,
  install_dir: get_option('datadir') / 'metainfo'
)

appstreamcli = find_program('appstreamcli', required: true)
test('Validate appstream file', appstreamcli,
     args: ['validate', '--no-net', '--explain', appstream_file])

configure_file(
  input: 'de.capypara.FieldMonitor.gschema.xml.in',
  output: get_option('app-id') + '.gschema.xml',
  configuration: conf,
  install_dir: get_option('datadir') / 'glib-2.0' / 'schemas'
)

compile_schemas = find_program('glib-compile-schemas', required: true)
test('Validate schema file',
     compile_schemas,
     args: ['--strict', '--dry-run', meson.current_build_dir()])

subdir('icons')
