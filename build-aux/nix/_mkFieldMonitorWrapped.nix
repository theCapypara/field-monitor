{ writeShellApplication }:
fm:
# This wrapper is needed since FieldMonitor needs it's own sister binaries in PATH.
writeShellApplication {
  name = "field-monitor-path-wrapper";

  runtimeInputs = [ fm ];

  text = ''
    exec ${fm.meta.mainProgram} "$@"
  '';
}
