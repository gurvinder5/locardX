# Synthetic Filesystem Images

This directory holds generated synthetic filesystem image files for testing carving and recovery engines.

## Notes
- Binary disk images (`*.raw`, `*.dd`, `*.img`, `*.vhd`, `*.vhdx`) are ignored by `.gitignore` to prevent bloating the Git repository.
- Use `scripts/generate-test-media.bat` (Windows) or standard tools (`dd`, `qemu-img`) to create test media locally.
