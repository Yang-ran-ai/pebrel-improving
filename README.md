# Pebrel Improving

This repository is the build and customization control plane for the Windows x64 test package.

## Windows x64 build

The workflow `.github/workflows/build-windows-x64.yml` builds both:

- `Pebrel-v2.5.0-windows-x64.zip` (portable)
- `Pebrel-v2.5.0-windows-x64-setup.exe` (installer)

The build runs on a GitHub-hosted Windows runner. No Rust, Cargo, Visual Studio Build Tools, or Inno Setup installation is required on the tester's PC.

The current build is pinned to source snapshot `Yang-ran-ran/pebrel-custom@8b8a4e50d4630a2a83b2e968250bac46c22fd263`. Repository-owned changes are kept in `patches/` and applied before compiling. This pin makes the build reproducible; source migration into this repository is not yet complete.

## Current customization overlay

- The existing AI answer reader already supports rich Markdown and a native/plain-source presentation.
- The compact mode control is relabeled **Markdown / Native** in English and Simplified Chinese.
- The source snapshot already contains the DeepSeek provider, Flash / Pro model choices, and peak/off-peak pricing guidance. Prices and schedule should be checked against the [official DeepSeek pricing page](https://api-docs.deepseek.com/zh-cn/quick_start/pricing/) because they can change.

## Downloading the test package

Open **Actions → Build Windows x64 test packages** and select **Run workflow**. When the run completes, download the `pebrel-windows-x64` artifact. The artifact contains both the portable ZIP and installer EXE and is retained for 30 days.

The package is not code-signed. Windows SmartScreen may display a warning on first launch.
