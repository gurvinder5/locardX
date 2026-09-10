# LocardX Test Fixtures

This directory contains test fixtures and generators for validating carving and sanitization engines.

## Directory Structure
- `files/`: Known-good files (JPEGs, PDFs, ZIPs, SQLite databases) used to test carving accuracy.
- `filesystems/`: Minimal filesystem image headers and metadata blocks (FAT32, NTFS, ext4).
- `synthetic_media/`: Generated raw test images with known byte sequences.

## Safety Invariant
Never store or commit real forensic evidence or proprietary data in this directory. All fixtures must be synthetic or publicly available test vectors.
