# notsuperhuman

**Your inbox deserves a native app.**

notsuperhuman is an open-source desktop mail client built in Rust with [GPUI](https://www.gpui.rs). It connects to Gmail through your existing Superhuman session and to Fastmail and other JMAP providers with an API token. One native process draws the interface directly to the GPU.

Email is essential software. It should be fast, understandable, and yours to improve. We are building the mail client we want to use every day: focused on the inbox, driven by the keyboard, and open from the rendering code to the network layer.

## Download

| Platform | Get it |
| --- | --- |
| macOS, Apple Silicon | [notsuperhuman-macos-arm64.dmg](https://github.com/nothq/notsuperhuman/releases/latest/download/notsuperhuman-macos-arm64.dmg) |
| macOS, Intel | [notsuperhuman-macos-x86_64.dmg](https://github.com/nothq/notsuperhuman/releases/latest/download/notsuperhuman-macos-x86_64.dmg) |
| Linux, x86_64 | [notsuperhuman-linux-x86_64.tar.gz](https://github.com/nothq/notsuperhuman/releases/latest/download/notsuperhuman-linux-x86_64.tar.gz) |
| Windows, x86_64 | [notsuperhuman-windows-x86_64.zip](https://github.com/nothq/notsuperhuman/releases/latest/download/notsuperhuman-windows-x86_64.zip) |

Each download is the whole app: one native binary. No installer, no runtime, nothing else to fetch.

- **macOS**: open the .dmg and drag notsuperhuman into Applications. The first time you open it, go to System Settings › Privacy & Security and click **Open Anyway**.
- **Linux**: `tar -xzf notsuperhuman-linux-x86_64.tar.gz` and run `./notsuperhuman-linux-x86_64/notsuperhuman`.
- **Windows**: unzip and run `notsuperhuman.exe`.

Continue with Superhuman runs on macOS. On Linux and Windows, connect Fastmail or another JMAP account.

Every [release](https://github.com/nothq/notsuperhuman/releases) is built from source by GitHub Actions.

## Connect your inbox

For Gmail, sign in to **Superhuman Desktop**, launch notsuperhuman, and choose **Continue with Superhuman**. Approve macOS Keychain access when prompted. Your connected Gmail accounts appear in the account switcher.

notsuperhuman uses Superhuman's session API to obtain short-lived Gmail access tokens and refreshes them as needed. Session credentials stay in `~/.notsuperhuman/auth.json`, readable only by your user. Superhuman session cookies go only to Superhuman; Gmail access tokens go to Google. After connecting, Superhuman Desktop can be closed. To reconnect an expired session, sign in there again and choose **Continue with Superhuman**.

For JMAP, enter your provider's server address and API token. Fastmail accepts `api.fastmail.com`; create its token under **Settings → Privacy & Security → API tokens**.

## Built for mail

- Multiple accounts, with a searchable account switcher.
- Inbox, archive, drafts, sent mail, starred mail, labels, and search.
- Threaded conversations, rich HTML, inline images, and attachments.
- Compose, reply, forward, saved drafts, and sending.
- Archive, trash, read status, stars, and snooze.
- Local encrypted mail caches and light and dark appearances.
- Keyboard navigation: **J/K** to move, **Enter** to open, **Esc** to return, and **Ctrl+0** to switch accounts.

The inbox loads message metadata in pages. Full messages and attachments load when you open them. There is no embedded browser, JavaScript runtime, or Electron helper process.

## Memory on macOS

| Client | Processes | Median footprint |
| --- | ---: | ---: |
| **notsuperhuman** | **1** | **71.2 MiB** |
| Superhuman Desktop | 9 | 1,052.3 MiB |

Measured on 5 October 2026 with macOS `footprint`, on macOS 26.5.1, hardware model Mac17,8, with the screen unlocked and the same Gmail account loaded. Each app was raised to the foreground for three samples, ten seconds apart. notsuperhuman showed the Inbox at 2640×1720 pixels; Superhuman Desktop 1041.0.63 showed `in:inbox` results at 2992×2522 pixels. The Superhuman total includes its renderer, GPU, utility, and crash reporter processes.

The release build limits idle Gmail connections and returns freed allocator pages while idle. [Raw measurements](benchmarks/macos-2026-10-05.json) and the [measurement script](scripts/measure-memory.py) are included. To repeat the comparison, open both inboxes, leave the screen unlocked, let loading finish, and run `python3 scripts/measure-memory.py --client notsuperhuman` with notsuperhuman in front. Repeat with `--client "Superhuman Desktop"` and Superhuman in front.

## Build on macOS

Install Rust 1.95 and the Xcode command line tools, then:

```sh
git clone https://github.com/nothq/notsuperhuman
cd notsuperhuman
cargo run --release --locked
```

To package the app the way releases do, a .dmg holding `notsuperhuman.app` (application identifier `dev.nothq.notsuperhuman`):

```sh
cargo build --release --locked --target aarch64-apple-darwin
scripts/package aarch64-apple-darwin
open dist/notsuperhuman-macos-arm64.dmg
```

Run the tests with:

```sh
cargo test --release --locked --workspace
```

## Build with us

The source is the product. Bring your mail workflow, improve the HTML renderer, add an integration, or make the next interaction faster. Small, focused pull requests with a clear example are welcome.

| Directory | Responsibility |
| --- | --- |
| `crates/notsuperhuman` | Native application and window |
| `crates/mail` | Superhuman, Gmail, JMAP, mail models, and interface |
| `crates/gpui-components` | Native text input and reusable controls |
| `crates/local_cache` | Encrypted local storage |
| `crates/secret_store` | Private credential storage |
| `crates/remote-image` | Remote image loading |
| `crates/app/model` | Shared application models |

Licensed under [AGPL-3.0](LICENSE). Bundled Lato fonts include their [SIL Open Font License](crates/notsuperhuman/assets/fonts/OFL.txt).
