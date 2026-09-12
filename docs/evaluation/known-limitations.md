# LocardX Known Technical & Operational Limitations

## 1. Purpose & Forensic Transparency

In adherence to digital forensics standards (ISO/IEC 27037 and NIST SP 800-88), software tools must transparently articulate their known physical, logical, and environmental boundaries. This document outlines the documented technical limitations of the LocardX Forensic Workstation v0.1.0 release.

---

## 2. Storage Hardware & Sanitization Limitations

### 2.1 Solid-State Drive (SSD) & Flash Memory Remanence (FTL Wear-Leveling)
- **Technical Context**: Modern flash storage devices (SATA SSDs, NVMe SSDs, USB thumb drives, SD cards) implement an internal Flash Translation Layer (FTL). The FTL dynamically maps logical block addresses (LBAs) to physical NAND flash pages to optimize performance and distribute wear leveling.
- **Limitation**: When software issues standard LBA overwrite patterns (such as NIST 800-88 Clear single-pass zeros), the FTL remaps written LBAs to available physical blocks. Retired, worn, or over-provisioned physical flash blocks are hidden from the host operating system and cannot be overwritten via user-space software LBA writes alone.
- **Forensic Mitigation**: On solid-state media requiring high-assurance sanitization, hardware-level cryptographically validated cryptographic erase (NVMe Format / Sanitize Crypto Scramble) or physical media degaussing/shredding must be employed.

### 2.2 Reallocated Bad Sectors
- **Technical Context**: Magnetic hard drives and flash controllers automatically detect damaged sectors during runtime and remap them to spare sectors via internal G-Lists (Grown Defect Lists).
- **Limitation**: Host operating systems cannot access or overwrite remapped bad sectors through standard raw block I/O (`\\.\PhysicalDriveN` or `/dev/sdX`). Remanent magnetic or electronic charge within these sectors may theoretically remain recoverable in specialized cleanroom hardware recovery environments.

### 2.3 Real Hardware Erasure Default Simulation Mode
- **Technical Context**: The core architecture includes `RealHardwareExecutionGate`.
- **Limitation**: In release v0.1.0, real hardware raw disk overwriting is permanently disabled by default. All drive sanitization workflows execute in verified **Simulation Mode**. This protects incident responder lab workstations from accidental device destruction while proving algorithm compliance and generating formal certificates.

---

## 3. Forensic Acquisition & File Recovery Limitations

### 3.1 Advanced Apple APFS Filesystem Containers
- **Technical Context**: Apple's APFS filesystem utilizes copy-on-write (CoW), dynamic space sharing, and complex B-tree checkpoint maps.
- **Limitation**: Raw file carving (extracting JPEGs, PNGs, PDFs, SQLite databases by header/footer signatures) functions with 100% precision on raw APFS image containers. However, logical filesystem directory-tree reconstruction for encrypted APFS volumes without recovery keys is not supported in v0.1.0.

### 3.2 Proprietary Forensic Containers (Expert Witness `.E01` / `.L01`)
- **Limitation**: LocardX natively creates and processes raw bitstream disk images (`.raw`, `.dd`, `.img`). Support for reading proprietary segmented EnCase `.E01` containers with zlib compression is scheduled for the v0.2.0 release roadmap.

### 3.3 Severely Fragmented Non-Contiguous File Carving
- **Limitation**: The file carving engine includes fragment reconstruction logic based on sequential cluster entropy and header/footer alignment. Files split across non-contiguous clusters interspersed with unrelated data may be flagged as `PartiallyValid` with lower confidence scores rather than completely reconstructed.

---

## 4. Workstation Architecture & Operational Scope

### 4.1 Single-Seat Desktop Architecture
- **Limitation**: LocardX v0.1.0 is engineered as a standalone desktop forensic workstation. It is not designed as a multi-tenant client-server daemon or cloud-hosted web portal.
- **Operational Recommendation**: Each forensic workstation runs its own local embedded SQLite database and local audit hash chain.

### 4.2 OS Privilege Requirements for Physical Storage Discovery
- **Limitation**: Direct enumeration and bitstream acquisition of raw physical hardware drives (`\\.\PhysicalDriveN` on Windows, `/dev/sdX` on Linux) requires Administrator or `root` execution privilege. When run as an unprivileged user, physical drive lists will be restricted by the OS kernel.
