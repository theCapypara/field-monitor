bin_name = 'field-monitor-vte-driver-proxmox'
out_name = 'de.capypara.FieldMonitor.PtyDrv.Proxmox'
this_dir = meson.current_source_dir()
this_dir_build = meson.current_build_dir()

cargo_bin  = find_program('cargo')
cargo_opt  = [ '--manifest-path', this_dir / '..' / 'Cargo.toml' ]
cargo_opt += [ '--target-dir', this_dir_build ]
cargo_env  = [ 'CARGO_HOME=' + meson.project_build_root()  / 'cargo-home' ]

if get_option('buildtype') == 'release'
  cargo_opt += [ '--release', ]
  rust_target = 'release'
else
  rust_target = 'debug'
endif

cargo_build = custom_target(
  'cargo-build',
  build_by_default: true,
  build_always_stale: true,
  output: out_name,
  console: true,
  install: true,
  install_dir: get_option('libexecdir'),
  command: [
    'env', cargo_env,
    cargo_bin, 'build',
    cargo_opt, '&&', 'cp', this_dir_build / rust_target / bin_name, '@OUTPUT@',
  ]
)
