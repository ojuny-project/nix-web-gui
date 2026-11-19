# Deployment

The server binary is `nix-web-gui`, built from
`/home/den/repos/ojuny-future-repos/nix-web-gui/server/src/main.rs`. It has
three subcommands: `serve`, `popout`, and `worker`. Only `serve` and `popout`
bind a listener. `worker` is the stdio process the parent spawns for a signed-in
account.

## Running the service

```sh
nix-web-gui serve \
  --listen 127.0.0.1:8787 \
  --static-dir "$(dirname "$(command -v nix-web-gui)")/../share/nix-web-gui/dist" \
  --theme-file /etc/nix-web-gui/theme \
  --pam-service nix-web-gui \
  --flake /home/den/repos/nixos-configs/dendrite \
  --session-ttl-secs 43200 \
  --max-sessions 16 \
  --allowed-group wheel
```

| Flag | Default | Meaning |
|---|---|---|
| `--listen` | `127.0.0.1:8787` | Listener address |
| `--static-dir` | `NIX_WEB_GUI_STATIC_DIR` or `<manifest>/../frontend/dist` | Built frontend directory |
| `--theme-file` | `NIX_WEB_GUI_THEME_FILE` or `/etc/nix-web-gui/theme` | Base16 scheme name file |
| `--flake` | none | Initial flake offered to a browser |
| `--page` | none | Regex for the first menu selected after open |
| `--pam-service` | `nix-web-gui` | PAM service name |
| `--allowed-group` | none | Repeatable allowlist group |
| `--dev-auth` | false | In-process engine and a shared password |
| `--dev-password` | none | Password for `--dev-auth` |
| `--session-ttl-secs` | `43200` | Browser session lifetime |
| `--max-sessions` | `16` | Maximum live sessions; the oldest is dropped first |
| `--secure-cookie` | false | Always set `Secure` on the session cookie |

A five-minute task purges expired sessions and engine state or workers whose
session no longer exists.

`popout` starts a loopback server with the given `--flake`, `--page`, and
`--port`, opens `xdg-open` unless `NIX_WEB_GUI_NO_OPEN` is set, and still
requires PAM login. It is a local launcher, not a second authentication mode.

Without `--static-dir` the server serves a placeholder page that says the
frontend is not built.

## Engine dependency

`nix-settings-gui` is the engine crate and DSL. The flake pins it to a GitHub
revision:

```nix
nix-settings-gui.url = "github:ojuny-project/nix-settings-gui/3f3a980d5186321ef4c01c8e064a17a0b5b10b5d";
```

`flake.lock` locks that input to the NSG revision
`3f3a980d5186321ef4c01c8e064a17a0b5b10b5d`. A fresh clone builds without a
local checkout.
`github:ojuny-project/nix-settings-gui`, before the tarball can build anywhere
else.

## NixOS module

`/home/den/repos/ojuny-future-repos/nix-web-gui/modules/nixos.nix` provides
`services.nix-web-gui`.

```nix
services.nix-web-gui = {
  enable = true;
  listenAddress = "127.0.0.1";
  port = 8787;
  allowedGroups = [ "wheel" ];
  flakePath = "/home/den/repos/nixos-configs/dendrite";
  startupPage = "Theme";
};
```

| Option | Type | Default | Meaning |
|---|---|---|---|
| `services.nix-web-gui.enable` | bool | false | Enable the systemd service |
| `services.nix-web-gui.package` | package | built from `withOjunyWidgets` | Binary and static assets |
| `services.nix-web-gui.withOjunyWidgets` | bool | true | Select `packages.default` or `packages.nix-web-gui-minimal` |
| `services.nix-web-gui.listenAddress` | string | `127.0.0.1` | Listener address |
| `services.nix-web-gui.port` | port | `8787` | Listener port |
| `services.nix-web-gui.openFirewall` | bool | false | Open `port` when the address is not `127.0.0.1` |
| `services.nix-web-gui.allowRemote` | bool | false | Permit a non-loopback address |
| `services.nix-web-gui.allowedGroups` | list of string | `[ "wheel" ]` | Groups allowed to sign in |
| `services.nix-web-gui.pamService` | string | `nix-web-gui` | PAM service name |
| `services.nix-web-gui.flakePath` | null or string | null | Passed as `--flake` |
| `services.nix-web-gui.startupPage` | null or string | null | Regex for the first menu |
| `services.nix-web-gui.theme` | string | `config.ojuny.style.palette` or `everforest-dark-hard` | Base16 scheme name |
| `services.nix-web-gui.themeFile` | path | `/etc/nix-web-gui/theme` | File read by `--theme-file` |
| `services.nix-web-gui.sessionTtlSeconds` | positive int | `43200` | Passed as `--session-ttl-secs` |
| `services.nix-web-gui.maxSessions` | positive int | `16` | Passed as `--max-sessions` |
| `services.nix-web-gui.secureCookie` | bool | false | Passed as `--secure-cookie` |

The module:

- declares `security.pam.services.system-auth` and includes it from
  `security.pam.services.nix-web-gui` for auth, account, password, and session;
- writes `services.nix-web-gui.theme` and a newline to
  `/etc/nix-web-gui/theme`;
- starts `nix-web-gui serve` as `root` with `NoNewPrivileges = false`,
  `Restart = on-failure`, and `RestartSec = 2s`;
- passes `--flake`, `--session-ttl-secs`, `--max-sessions`, one
  `--allowed-group` per entry, and `--secure-cookie` only when enabled;
- adds `services.nix-web-gui.port` to
  `networking.firewall.allowedTCPPorts` only when `openFirewall` is true and
  the listen address is not `127.0.0.1`.

`services.nix-web-gui.startupPage` is declared, and
`/home/den/repos/ojuny-future-repos/nix-web-gui/modules/nixos.nix` computes a
`--page` argument for it, but the `ExecStart` line in that file does not
interpolate the argument. The module currently starts the service without
`--page`, so the option has no effect. Pass `--page` manually or fix the
`ExecStart` template.

The root parent needs `NoNewPrivileges = false` because it calls `setgroups`,
`setgid`, and `setuid` before executing a worker, and because the Secure Boot
setup, rebuild, and reboot actions run in the parent. The worker does not run as
root. See `/home/den/repos/ojuny-future-repos/nix-web-gui/docs/architecture.md`.

## PAM service

Authentication runs in the root parent through
`/home/den/repos/ojuny-future-repos/nix-web-gui/server/src/auth/pam.rs`. It runs
`pam_authenticate` and then `pam_acct_mgmt` against
`services.nix-web-gui.pamService`, default `nix-web-gui`. The module declares
`security.pam.services.system-auth` and writes
`security.pam.services.nix-web-gui.text` with only these lines:

```text
auth include system-auth
account include system-auth
password include system-auth
session include system-auth
```

NixOS has no built-in `system-auth`; the module's declaration creates it from
the default NixOS rules. The web service has no module text of its own and
inherits every module added to that stack. Out of the box the stack is the
NixOS default: `pam_unix` auth, account, password, and session, which includes
account expiry. The VM test asserts a successful login, a rejected wrong
password, the group allowlist, cross-user denial, and a rejected login for an
account whose `expires` date has passed.

MFA, faillock, LDAP, and Kerberos are not provisioned or tested by this
repository. An operator must add them to `system-auth`; the web service picks
them up through the includes:

- MFA: set the per-service options on `security.pam.services.system-auth`, such
  as `googleAuthenticator.enable`, `duoSecurity.enable`, `u2f.enable`,
  `yubicoAuth`, or `otpwAuth`, and enroll the tokens or keys for each account.
- Faillock: set `security.pam.services.system-auth.logFailures = true;` and tune
  the lockout thresholds in `/etc/security/faillock.conf`. The server's own
  five-failure throttle is separate and still applies.
- LDAP: set `users.ldap.enable = true;` and `users.ldap.loginPam = true;`, plus
  the LDAP server and base settings. `pam_ldap` then joins the `system-auth`
  stack.
- Kerberos: set `security.pam.krb5.enable = true;` and configure
  `security.krb5`. `pam_krb5` then joins the `system-auth` stack.

If `services.nix-web-gui.pamService` is changed, define that PAM service
yourself and include `system-auth` from it the same way. The account phase
follows a successful authentication.

## Allowlist groups

`services.nix-web-gui.allowedGroups` is enforced after PAM authentication. The
value is passed to the server as one `--allowed-group` flag per entry. The
server looks the account up with `uzers::get_user_by_name` and accepts the login
when one of the account's groups matches an allowed name.

- The default is `[ "wheel" ]`.
- An empty list allows every device account that PAM accepts.
- The module assertion requires a non-empty list before a non-loopback address
  is accepted.

## `allowRemote` and the listener

The listener is loopback-only by default. Setting
`services.nix-web-gui.listenAddress` to anything other than `127.0.0.1` or
`::1` requires:

```nix
services.nix-web-gui = {
  listenAddress = "0.0.0.0";
  allowRemote = true;
  allowedGroups = [ "wheel" ];
};
```

Without both, evaluation fails with:

```text
nix-web-gui: set allowRemote and a non-empty allowedGroups list before binding a non-loopback address.
```

`allowRemote` is a configuration gate, not a hardening feature. The open items
in `/home/den/repos/ojuny-future-repos/nix-web-gui/TODO.md` still apply.

## Reverse proxy

Terminate TLS at the proxy and forward to the loopback listener. The server
speaks plain HTTP. It reads `X-Forwarded-Proto` to set the `Secure` cookie flag,
and it adds the Content Security Policy and the standard security headers to
every response.

```nginx
server {
  listen 443 ssl;
  server_name settings.example.lan;

  ssl_certificate     /etc/ssl/settings.example.lan.pem;
  ssl_certificate_key /etc/ssl/settings.example.lan.key;

  location / {
    proxy_pass http://127.0.0.1:8787;
    proxy_http_version 1.1;
    proxy_set_header Host $host;
    proxy_set_header X-Forwarded-Proto $scheme;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;

    # /api/events is a long-lived SSE stream.
    proxy_buffering off;
    proxy_read_timeout 3600s;
  }
}
```

The `Host` header must be preserved because `POST /api/login`, `POST /api/logout`,
and `POST /api/rpc` reject an `Origin` that does not match it. Requests without
an `Origin` header are allowed and remain CSRF-gated where applicable.

The SSE stream at `/api/events` sends one `theme` event on connect, a
`theme.changed` event on a palette change, and a `heartbeat` event every 15
seconds. It also carries `progress` events for scan, save, apply, update-check,
and rebuild jobs. Buffering proxies delay the palette update but do not break
it.

## TLS termination

- There is no built-in TLS listener and no certificate management. See
  `TODO.md`.
- Keep the app listener on `127.0.0.1` and expose only the proxy.
- The session cookie is HttpOnly and SameSite=Strict. It also gets `Secure` when
  the proxy sends `X-Forwarded-Proto: https` or the service uses
  `--secure-cookie`.
- Add HSTS at the proxy. The app sets CSP, `X-Content-Type-Options`,
  `X-Frame-Options`, `Referrer-Policy`, `Permissions-Policy`, and
  `Cross-Origin-Opener-Policy` itself.

## Firewall

| Setup | Firewall action |
|---|---|
| Loopback plus proxy on the same host | No port to open; leave `openFirewall` false |
| Proxy on another host | `allowRemote = true`, non-empty `allowedGroups`, and restrict the port to the proxy address |
| Direct browser access on the LAN | Not recommended; requires `allowRemote`, a group list, and a TLS terminator in front |

`openFirewall` treats every address other than `127.0.0.1` as remote, so an
`::1` listener with `openFirewall = true` also opens the port even though the
listener is loopback. Opening the port manually bypasses that check, so keep the
assertion in place.

## Remote access warnings

Exposing the service beyond loopback puts a root parent that performs PAM
authentication, runs the Secure Boot setup actions, and spawns per-user workers
on the network. Before doing it:

- There is no built-in TLS listener. The service is designed for loopback use; a proxy is the operator's responsibility.
- PAM inherits the generated `system-auth` stack, including account expiry.
  MFA, faillock, LDAP, and Kerberos apply only when the operator configures them
  on `security.pam.services.system-auth`; the VM test does not cover them.
- Secure Boot setup, rebuild, and reboot require the device password again and
  verify it through PAM before the root service account runs the fixed action.
- Apply and rebuild for ordinary settings run as the signed-in account and
  depend on that account's sudo configuration.
- The NixOS VM auth test covers cross-user isolation: bob can log in through the
  allowed group but cannot read alice's `0700` home. Carol is rejected by the
  allowlist, and an account with a past `expires` date is rejected by PAM.
- `nix flake check` runs the browser smoke test, the VM auth test, and the
  minimal-renderer check.

Implemented protections include login throttling, the session cap and cleanup,
the conditional `Secure` cookie, the `Origin` check, the CSP and security
headers, and tracing audit records. See
`/home/den/repos/ojuny-future-repos/nix-web-gui/TODO.md` and
`/home/den/repos/ojuny-future-repos/nix-web-gui/docs/security.md`.

For a single remote user, prefer an SSH tunnel or a WireGuard address and keep
the listener on loopback.

## Development auth

```sh
nix-web-gui serve \
  --dev-auth --dev-password secret \
  --static-dir frontend/packages/app/dist \
  --theme-file /tmp/nix-web-gui-theme
```

`--dev-auth` has two effects:

- `POST /api/login` compares the supplied password with `--dev-password` and
  skips PAM and the allowlist.
- The engine runs in process under the server account instead of a per-user
  worker.

The in-process engine is created in
`/home/den/repos/ojuny-future-repos/nix-web-gui/server/src/main.rs` and selected
by `EngineMode::InProcess` in
`/home/den/repos/ojuny-future-repos/nix-web-gui/server/src/http.rs`.

Why it is loopback only:

- Every browser session runs in the server account's process, so scans, writes,
  git, and rebuild use that account's file permissions and sudo rights.
- The per-user worker boundary, and therefore the operating-system check
  between users, is absent.
- There is no group allowlist.

The default `--listen` is `127.0.0.1:8787`. Nothing in
`/home/den/repos/ojuny-future-repos/nix-web-gui/server/src/config.rs` rejects a
non-loopback address when `--dev-auth` is set, so keeping it on loopback is a
policy decision, not an enforced check. The NixOS module never passes the dev
flags.
