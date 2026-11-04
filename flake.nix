{
  description = "A very basic flake";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-26.05";
    crate2nix.url = "github:nix-community/crate2nix";
    crate2nix.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { self, nixpkgs, crate2nix }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
      cargoNix = crate2nix.tools.${system}.appliedCargoNix {
        name = "Ros2TraceAnalyzer";
        src = ./.;
      };
    in {
      packages.${system}.default = cargoNix.workspaceMembers.Ros2TraceAnalyzer.build.override {
        crateOverrides = pkgs.defaultCrateOverrides // {
          bt2-sys = attrs: {
            nativeBuildInputs = [ pkgs.rustPlatform.bindgenHook ];
            buildInputs = [ pkgs.babeltrace2 ];
          };
          Ros2TraceAnalyzer = attrs: {
            buildInputs = [ pkgs.babeltrace2 ];
          };
          yeslogic-fontconfig-sys = attrs: {
            nativeBuildInputs = [ pkgs.pkg-config ];
            buildInputs = [ pkgs.fontconfig ];
          };
        };
      };

      devShells.${system}.default = import ./shell.nix { inherit pkgs; };
    };
}
