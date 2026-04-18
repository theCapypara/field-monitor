{
  rustPlatform,
  field-monitor-devel
}:
# Builds field-monitor-devel with a custom `rdw` path.
# You need to have rdw checked out and provide an _absolute_ path in the following environment variable:
#
# NIX_RDW_PATH=/home/myuser/development/rdw
#
# Requires `--impure`.
let
  rdwSrc = /. + builtins.getEnv "NIX_RDW_PATH";
in
(field-monitor-devel.overrideAttrs (finalAttrs: previousAttrs: {
  pname = previousAttrs.pname + "-local-rdw";

  cargoDeps = (rustPlatform.importCargoLock {
    # Remove the Git repo links for the rdw dependencies also from the imported lock file.
    lockFileContents = builtins.concatStringsSep "\n" (
      builtins.filter
        (line: builtins.match ''source = "git\+https://gitlab\.gnome\.org/theCapypara/rdw\.git.*'' line == null)
        (builtins.filter builtins.isString (builtins.split "\n" (builtins.readFile "${previousAttrs.src}/Cargo.lock")))
    );
    allowBuiltinFetchGit = true;
  });

  postUnpack = (previousAttrs.postUnpack or "") + ''
    unpacked="./*-source"

    # Remove existing [patch."https://gitlab.gnome.org/theCapypara/rdw.git"] section if present
    # (assumes it's the last section in Cargo.toml)
    if grep -q '^\[patch\."https://gitlab\.gnome\.org/theCapypara/rdw\.git"\]' $unpacked/Cargo.toml; then
      sed -i '/^\[patch\."https:\/\/gitlab\.gnome\.org\/theCapypara\/rdw\.git"\]/,$d' $unpacked/Cargo.toml
    fi

    echo '[patch."https://gitlab.gnome.org/theCapypara/rdw.git"]' >> $unpacked/Cargo.toml
    echo 'rdw4 = { path = "${rdwSrc}/rdw4" }' >> $unpacked/Cargo.toml
    echo 'rdw4-spice = { path = "${rdwSrc}/rdw4-spice" }' >> $unpacked/Cargo.toml
    echo 'rdw4-vnc = { path = "${rdwSrc}/rdw4-vnc" }' >> $unpacked/Cargo.toml
    echo 'rdw4-rdp = { path = "${rdwSrc}/rdw4-rdp" }' >> $unpacked/Cargo.toml

    sed -i '/^source = "git+https:\/\/gitlab\.gnome\.org\/theCapypara\/rdw\.git/d' $unpacked/Cargo.lock
  '';
}))
