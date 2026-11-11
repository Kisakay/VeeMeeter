{
  description = "VeeMeeter - native PipeWire audio mixer and router for Linux (Rust)";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAll = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      packages = forAll (pkgs: rec {
        veemeeter = pkgs.callPackage ./nix/package.nix { };
        default = veemeeter;
      });

      apps = forAll (pkgs: {
        default = {
          type = "app";
          program = "${self.packages.${pkgs.stdenv.hostPlatform.system}.veemeeter}/bin/veemeeter";
        };
      });

      overlays.default = final: _prev: {
        veemeeter = final.callPackage ./nix/package.nix { };
      };

      nixosModules.default = import ./nix/nixos-module.nix self;
      homeManagerModules.default = import ./nix/hm-module.nix self;

      devShells = forAll (pkgs: {
        default = pkgs.mkShell {
          inputsFrom = [ self.packages.${pkgs.stdenv.hostPlatform.system}.veemeeter ];
          packages = with pkgs; [ cargo rustc rustfmt clippy rust-analyzer ];
          LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (import ./nix/runtime-libs.nix pkgs);
        };
      });
    };
}
