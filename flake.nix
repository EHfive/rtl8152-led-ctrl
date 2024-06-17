{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, naersk, ... }:
    let
      forAllSystems = nixpkgs.lib.genAttrs [ "x86_64-linux" "x86_64-darwin" "i686-linux" "aarch64-linux" "aarch64-darwin" ];

      package = { pkgs, lib, stdenv }:
        let
          hostCargoEnvVarTarget = stdenv.hostPlatform.rust.cargoEnvVarTarget;
          hostCargoEnvVarTargetLower = lib.toLower hostCargoEnvVarTarget;
          hostCC = pkgs.rust.envVars.linkerForHost;
        in
        (pkgs.callPackage naersk { }).buildPackage {
          src = ./.;

          nativeBuildInputs = with pkgs; [
            pkg-config
          ];
          buildInputs = with pkgs; [
            libusb1
          ];
          strictDeps = true;

          "CC_${hostCargoEnvVarTargetLower}" = hostCC;
          "CARGO_TARGET_${hostCargoEnvVarTarget}_LINKER" = hostCC;
        };

      overlay = final: prev: {
        rtl8125-led-ctrl = prev.callPackage package { };
      };

      defaultModule = {
        imports = [
          # TODO: add udev rules to setup LED configuration on USB device plug-in
          {
            nixpkgs.overlays = [ overlay ];
          }
        ];
      };
    in
    {
      overlays.default = overlay;
      modules.default = defaultModule;

      packages = forAllSystems (system:
        let
          pkgs = import nixpkgs {
            inherit system;
          };
        in
        {
          default = pkgs.callPackage package { };
          default-aarch64 = pkgs.pkgsCross.aarch64-multiplatform.callPackage package { };
        }
      );

      legacyPackages = forAllSystems (system:
        import nixpkgs {
          inherit system;
          overlays = [ overlay ];
          crossOverlays = [ overlay ];
        }
      );
    };
}
