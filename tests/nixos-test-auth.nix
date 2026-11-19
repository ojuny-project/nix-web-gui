{ self, pkgs, system }:
pkgs.testers.nixosTest {
  name = "nix-web-gui-auth";

  nodes.machine = { ... }: {
    imports = [ self.nixosModules.default ];

    system.stateVersion = "26.05";

    services.nix-web-gui = {
      enable = true;
      package = self.packages.${system}.default;
      allowedGroups = [ "wheel" "weblogin" ];
    };

    users.groups.weblogin = { };

    users.users.alice = {
      isNormalUser = true;
      extraGroups = [ "wheel" ];
      homeMode = "0700";
      password = "secret";
    };

    users.users.bob = {
      isNormalUser = true;
      extraGroups = [ "weblogin" ];
      password = "secret";
    };

    users.users.carol = {
      isNormalUser = true;
      password = "secret";
    };

    environment.systemPackages = [ pkgs.curl pkgs.jq ];
  };

  testScript = ''
    import json

    machine.wait_for_unit("nix-web-gui.service")
    machine.wait_for_open_port(8787)

    machine.succeed("curl -fsS http://127.0.0.1:8787/api/health")

    machine.succeed("mkdir -p /home/alice/nwg-flake/modules")
    machine.succeed("""cat > /home/alice/nwg-flake/flake.nix <<'EOF'
    { outputs = { self, nixpkgs }: { }; }
    EOF
    """)
    machine.succeed("""cat > /home/alice/nwg-flake/modules/system.nix <<'EOF'
    /* @menu displayname = "Test"; */
    { ... }: {
      /* @option widget = "text"; label = "Hostname"; */
      networking.hostName = "vm";
    }
    EOF
    """)
    machine.succeed("chown -R alice:users /home/alice/nwg-flake")

    csrf = machine.succeed(
        "curl -fsS -c /tmp/alice.cookies "
        "-X POST http://127.0.0.1:8787/api/login "
        "-H 'content-type: application/json' "
        "-d '{\"username\":\"alice\",\"password\":\"secret\"}' "
        "| jq -r '.csrf'"
    ).strip()

    machine.succeed(
        f"curl -fsS -b /tmp/alice.cookies "
        "-X POST http://127.0.0.1:8787/api/rpc "
        "-H 'content-type: application/json' "
        f"-H 'x-csrf-token: {csrf}' "
        "-d '{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"echo\",\"params\":{\"ok\":true}}' "
        "| jq -e '.result.ok'"
    )

    opened = machine.succeed(
        f"curl -fsS -b /tmp/alice.cookies "
        "-X POST http://127.0.0.1:8787/api/rpc "
        "-H 'content-type: application/json' "
        f"-H 'x-csrf-token: {csrf}' "
        "-d '{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"session.open\",\"params\":{\"root\":\"/home/alice/nwg-flake\"}}'"
    )
    machine.log(opened)
    assert json.loads(opened)["result"]["menus"] >= 1

    staged = machine.succeed(
        f"curl -fsS -b /tmp/alice.cookies "
        "-X POST http://127.0.0.1:8787/api/rpc "
        "-H 'content-type: application/json' "
        f"-H 'x-csrf-token: {csrf}' "
        "-d '{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"control.setTyped\",\"params\":{\"path\":\"networking.hostName\",\"widget\":\"text\",\"value\":\"vm2\"}}'"
    )
    machine.log(staged)
    assert json.loads(staged)["result"]["staged"]

    applied = machine.succeed(
        f"curl -fsS -b /tmp/alice.cookies "
        "-X POST http://127.0.0.1:8787/api/rpc "
        "-H 'content-type: application/json' "
        f"-H 'x-csrf-token: {csrf}' "
        "-d '{\"jsonrpc\":\"2.0\",\"id\":4,\"method\":\"edits.apply\",\"params\":{}}'"
    )
    machine.log(applied)
    assert json.loads(applied)["result"]["files"] == 1

    machine.succeed("grep -q vm2 /home/alice/nwg-flake/modules/system.nix")

    machine.succeed(
        f"curl -fsS -b /tmp/alice.cookies "
        "-X POST http://127.0.0.1:8787/api/rpc "
        "-H 'content-type: application/json' "
        f"-H 'x-csrf-token: {csrf}' "
        "-d '{\"jsonrpc\":\"2.0\",\"id\":6,\"method\":\"control.set\",\"params\":{\"path\":\"networking.hostName\",\"value\":\"\\\"unterminated\"}}' "
        "| jq -e '.result.staged'"
    )
    broken = machine.succeed(
        f"curl -fsS -b /tmp/alice.cookies "
        "-X POST http://127.0.0.1:8787/api/rpc "
        "-H 'content-type: application/json' "
        f"-H 'x-csrf-token: {csrf}' "
        "-d '{\"jsonrpc\":\"2.0\",\"id\":7,\"method\":\"edits.apply\",\"params\":{}}'"
    )
    machine.log(broken)
    assert "error" in json.loads(broken)
    machine.succeed("grep -q vm2 /home/alice/nwg-flake/modules/system.nix")

    machine.fail(
        "curl -fsS -X POST http://127.0.0.1:8787/api/login "
        "-H 'content-type: application/json' "
        "-d '{\"username\":\"alice\",\"password\":\"wrong\"}'"
    )

    bob_csrf = machine.succeed(
        "curl -fsS -c /tmp/bob.cookies "
        "-X POST http://127.0.0.1:8787/api/login "
        "-H 'content-type: application/json' "
        "-d '{\"username\":\"bob\",\"password\":\"secret\"}' "
        "| jq -r '.csrf'"
    ).strip()

    bob_opened = machine.succeed(
        f"curl -fsS -b /tmp/bob.cookies "
        "-X POST http://127.0.0.1:8787/api/rpc "
        "-H 'content-type: application/json' "
        f"-H 'x-csrf-token: {bob_csrf}' "
        "-d '{\"jsonrpc\":\"2.0\",\"id\":5,\"method\":\"session.open\",\"params\":{\"root\":\"/home/alice/nwg-flake\"}}'"
    )
    machine.log(bob_opened)
    assert "error" in json.loads(bob_opened)

    machine.fail(
        "curl -fsS -X POST http://127.0.0.1:8787/api/login "
        "-H 'content-type: application/json' "
        "-d '{\"username\":\"carol\",\"password\":\"secret\"}'"
    )
  '';
}
