project('field-monitor', 'rust',
          version: '47.0',
    meson_version: '>= 1.1.0',
  default_options: [ 'warning_level=2', 'werror=false', ],
)

i18n = import('i18n')
gnome = import('gnome')



subdir('data')
subdir('lib/src')
subdir('connection/generic-group/src')
subdir('connection/libvirt/src')
subdir('vte-pty-driver/libvirt/src')
subdir('src')
subdir('po')

if get_option('buildtype') != 'release'
  subdir('connection/debug/src')
endif

gnome.post_install(
     glib_compile_schemas: true,
    gtk_update_icon_cache: true,
  update_desktop_database: true,
)
