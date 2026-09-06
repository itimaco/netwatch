# NetWatch

NetWatch is a lightweight desktop network monitor for Windows, macOS, and Linux. It combines fast heartbeat checks, full network probes, OS-level network events, system proxy detection, VPN/TUN awareness, history, and quality analytics in a tray application.

## Features

- Event-driven monitoring for interface, route, proxy, and connectivity changes.
- Lightweight TCP heartbeat checks for domestic and international connectivity.
- Full probes for gateway, DNS, domestic access, international access, proxy/VPN, and custom targets.
- Network states including offline, national-only access, proxy issues, VPN failures, DNS issues, degraded quality, and instability.
- Live tray tooltip with domestic/international latency, tunnel latency, loss, and connection details.
- Custom targets supporting `tcp`, `http`, and `icmp` checks.
- Per-target enable/disable, edit, delete, domestic/international grouping, and state relevance.
- History database with event records, samples, analytics, timelines, heatmaps, and CSV export.
- Desktop notifications with quiet mode, cooldowns, and outage-only filtering.
- Native installers for Windows, macOS, and Linux through GitHub Actions.

## Supported Platforms

| Platform | Release formats |
| --- | --- |
| Windows | NSIS installer, MSI |
| macOS | DMG, app archive |
| Linux | DEB, RPM, AppImage |

## Development

### Requirements

- Node.js 22 or newer
- Rust stable
- Tauri 2 prerequisites for your operating system

For Linux, install the Tauri WebKit and GTK dependencies before building. See the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for platform-specific details.

### Install dependencies

```bash
npm ci
```

### Run in development

```bash
npm run dev
```

### Build locally

```bash
npm run build
```

The generated bundles are placed under `src-tauri/target/release/bundle/`.

## Custom Targets

Custom targets are managed from the settings screen. Each target has:

- Name
- Host or URL
- Port
- Check type: `tcp`, `http`, or `icmp`
- Domestic or international grouping
- Optional influence on the overall network state
- Enabled/disabled status

HTTP targets must use an `http://` or `https://` URL. ICMP checks fall back to TCP when appropriate.

## Releases

Releases are created automatically when a semantic version tag is pushed:

```bash
git tag -a v0.3.3 -m "Release v0.3.3"
git push origin v0.3.3
```

The workflow in `.github/workflows/release.yml` builds Linux, macOS, and Windows bundles in parallel, uploads them as workflow artifacts, and publishes a GitHub Release containing the collected installers.

## Project Structure

```text
src/                 Frontend HTML, CSS, and JavaScript
src-tauri/src/       Rust application, probes, engine, tray, and watchers
src-tauri/icons/     Application icons
.github/workflows/   Cross-platform release workflow
```

## License

No license has been declared yet.
