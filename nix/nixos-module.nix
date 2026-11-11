# Module NixOS : `programs.veemeeter.enable = true;`
# Installe VeeMeeter et son service systemd *utilisateur* (lancé avec PipeWire
# dans chaque session).
self:
{ config, lib, pkgs, ... }:

let
  cfg = config.programs.veemeeter;
in
{
  options.programs.veemeeter = {
    enable = lib.mkEnableOption "VeeMeeter, a native PipeWire audio mixer and router";

    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${pkgs.stdenv.hostPlatform.system}.veemeeter;
      defaultText = lib.literalExpression "inputs.veemeeter.packages.\${system}.veemeeter";
      description = "The VeeMeeter package to use.";
    };

    autostart = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = ''
        Start the VeeMeeter daemon with PipeWire in every user session, so
        virtual devices and routes exist before the window is opened.
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    assertions = [{
      assertion = config.services.pipewire.enable;
      message = "programs.veemeeter requires services.pipewire.enable = true.";
    }];

    environment.systemPackages = [ cfg.package ];
    systemd.packages = [ cfg.package ];
    systemd.user.services.veemeeter.wantedBy = lib.mkIf cfg.autostart [ "pipewire.service" ];
  };
}
