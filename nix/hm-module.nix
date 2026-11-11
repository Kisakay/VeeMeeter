# Module home-manager : `services.veemeeter.enable = true;`
self:
{ config, lib, pkgs, ... }:

let
  cfg = config.services.veemeeter;
in
{
  options.services.veemeeter = {
    enable = lib.mkEnableOption "VeeMeeter, a native PipeWire audio mixer and router";

    package = lib.mkOption {
      type = lib.types.package;
      default = self.packages.${pkgs.stdenv.hostPlatform.system}.veemeeter;
      defaultText = lib.literalExpression "inputs.veemeeter.packages.\${system}.veemeeter";
      description = "The VeeMeeter package to use.";
    };
  };

  config = lib.mkIf cfg.enable {
    home.packages = [ cfg.package ];

    systemd.user.services.veemeeter = {
      Unit = {
        Description = "VeeMeeter audio mixer and router";
        After = [ "pipewire.service" "wireplumber.service" ];
        Requires = [ "pipewire.service" ];
        PartOf = [ "pipewire.service" ];
      };
      Service = {
        ExecStart = "${lib.getExe cfg.package} daemon";
        Restart = "on-failure";
        RestartSec = 2;
        NoNewPrivileges = true;
        PrivateNetwork = true;
        RestrictAddressFamilies = "AF_UNIX";
      };
      Install.WantedBy = [ "pipewire.service" ];
    };
  };
}
