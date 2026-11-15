{
  description = "VeeMeeter - native PipeWire audio mixer and router for Linux (Rust)";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAll = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      # `veemeeter` : compilé depuis les sources.
      # `veemeeter-bin` : binaire de la dernière release, construit par la CI
      # (nix/bin.json, écrit par la CI) et seulement adapté à NixOS : rien n'est
      # compilé. C'est le paquet par défaut (et celui des modules) quand il existe.
      packages = forAll (pkgs:
        let
          veemeeter = pkgs.callPackage ./nix/package.nix { };
          hasBin = builtins.pathExists ./nix/bin.json && pkgs.stdenv.hostPlatform.system == "x86_64-linux";
          bin = pkgs.callPackage ./nix/package-bin.nix { };
        in
        { inherit veemeeter; default = if hasBin then bin else veemeeter; }
        // pkgs.lib.optionalAttrs hasBin { veemeeter-bin = bin; });

      apps = forAll (pkgs: {
        default = {
          type = "app";
          program = "${self.packages.${pkgs.stdenv.hostPlatform.system}.default}/bin/veemeeter";
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
