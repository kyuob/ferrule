# Changelog

## 0.1.0 — 2026-11-11 (pre-release)

[Public release and downloads](https://github.com/kyuob/ferrule/releases/tag/v0.1.0).

- Rust CLI creates Flutter/Rust iOS and Android projects with FRB 2.13.0/Cargokit.
- Optional modules: foreground BLE scanner/GATT inspector, Rust SQLite notes,
  experimental platform-held-key private-note encryption and bounded Rust HTTPS GET.
- Embedded module contracts, dependency resolution, parsed configuration merges,
  read-only add previews and toolchain doctor.
- Managed-file interruption journal and explicit recovery preserve later user edits.
- Whole-directory no-replace creation preserves competing destinations and avoids
  incomplete final folders during process interruption.
- Installation/FAQ/recovery guides and native executable/checksum packaging with
  isolated install/run/removal checks. Three-OS CI packages are configured.

Known limits: Ferrule-generated projects only; manual customized-source migrations;
no automatic module upgrade/removal; SQLite data is plaintext; no peripheral/Classic
Bluetooth, background BLE, WebSocket/QUIC, NVR or RTSP. Packages are unsigned and
native-host-specific. See [status](docs/status.md) and [release procedure](docs/releases.md).
