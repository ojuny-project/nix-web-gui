{
  description = "nix-web-gui limitation fixture";

  outputs = { self, nixpkgs }: {
    nixosConfigurations.demo = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        ./modules/mkmerge.nix
        ./modules/map.nix
        ./modules/mkif.nix
      ];
    };
  };
}
