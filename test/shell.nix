{
  sources ? import ./nix/sources.nix,
  distro ? "rolling",
}:
let
  pkgs = import sources.nix-ros-overlay { overlays = [] ; config = {}; };
  ros = pkgs.rosPackages.${distro};
in
with pkgs; mkShell {
  name = "r2ta-tests-${distro}";
  packages = [
    bats
    (ros.buildEnv {
      paths = with ros; [
        ros-core
        ros2trace
        demo-nodes-cpp
      ];
    })
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
