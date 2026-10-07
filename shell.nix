{ pkgs ? import <nixpkgs> {} }:
with pkgs;
mkShell {
  packages = [
    babeltrace2
    fontconfig
    graphviz
    gtk3
    hicolor-icon-theme
    librsvg
    pkg-config
    python3Packages.pygobject3
    (python3Packages.xdot.overrideAttrs (old: {
      version = "1.6wsh";
      src = fetchFromGitHub {
        owner = "jrfonseca";
        repo = "xdot.py";
        hash = "sha256-nZjH+uZFMw9iPPnyPEqciSOABby4+QI5f8ROOqsOak4=";
        rev = "d2066a4bba06a66ae1c764a864e1fb9ffe76e953";
        # date = "2026-03-19T08:59:49Z";
      };
    }))
    rustPlatform.bindgenHook
    sqlite
  ];
  shellHook = ''
    export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${pkgs.lib.makeLibraryPath [
      sqlite
      fontconfig
      freetype
      babeltrace2
    ]}"
  '';
}
