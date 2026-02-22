# WSL2 Development Setup

## Required System Packages

```bash
sudo apt-get update
sudo apt-get install -y \
    libxkbcommon-x11-0 \
    mesa-vulkan-drivers
```

### What these do

- **libxkbcommon-x11-0** — X11 keyboard handling library, required by winit (Bevy's window backend)
- **mesa-vulkan-drivers** — Software Vulkan renderer (lavapipe), needed because WSL2 doesn't expose host GPU by default

## GPU Passthrough (Optional)

If you have an NVIDIA GPU on the Windows side, you can skip `mesa-vulkan-drivers` and use hardware-accelerated rendering instead:

1. Install the latest [NVIDIA drivers for Windows](https://www.nvidia.com/download/index.aspx) (version 510.06+)
2. The WSL2 kernel (5.10.43+) automatically exposes `/dev/dxg` for GPU access
3. No Linux-side driver install needed — Windows handles it

To verify GPU passthrough is working:

```bash
sudo apt-get install -y vulkaninfo
vulkaninfo --summary
```

You should see your NVIDIA GPU listed. If not, fall back to `mesa-vulkan-drivers`.
