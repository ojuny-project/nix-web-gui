{
  description = "nix-web-gui browser fixture";

  outputs = { self, nixpkgs }: {
    nixosConfigurations.demo = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [ ./modules/system.nix ];
    };
  };
}
