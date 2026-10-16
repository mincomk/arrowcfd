{
  description = "EGui Flake";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        devShells.default = pkgs.mkShell rec {
          name = "egui";
          nativeBuildInputs = with pkgs; [
            pkg-config
            libxkbcommon
            wayland
            libGL
          ];

          buildInputs = with pkgs; [
            wayland
            pkg-config
            openssl

            cmake
            ninja
            ccache
            udev

            libclang
            linuxHeaders

            libGL
            libxkbcommon
            vulkan-loader
            glibc.dev

            stdenv.cc.cc.lib
          ];

          shellHook = ''
            export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${builtins.toString (pkgs.lib.makeLibraryPath buildInputs)}";

            export BINDGEN_EXTRA_CLANG_ARGS="\
              -isystem ${pkgs.linuxHeaders}/include \
              -isystem ${pkgs.glibc.dev}/include"
          '';
        };
      }
    );
}
