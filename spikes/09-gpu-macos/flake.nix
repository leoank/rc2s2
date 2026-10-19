{
  description = "EXP-0009 spike: read GPU PerformanceStatistics through IOKit from Rust";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { nixpkgs, ... }:
    let
      forSystems = f: nixpkgs.lib.genAttrs [ "aarch64-darwin" "x86_64-darwin" ]
        (system: f nixpkgs.legacyPackages.${system});
    in {
      devShells = forSystems (pkgs: {
        default = pkgs.mkShell {
          packages = [ pkgs.rustc pkgs.cargo pkgs.apple-sdk ];
        };
      });
    };
}
