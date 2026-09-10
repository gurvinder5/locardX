# License Selection Guide for LocardX

> **Notice**: A formal software license has not yet been selected by the project owner. Before publishing or distributing LocardX publicly, the project maintainer must choose and apply an appropriate license.

---

## Considerations for LocardX

LocardX is a desktop application combining forensic data recovery and secure data sanitization. When selecting a license, consider the following models:

### 1. Copyleft Open Source (e.g., GNU General Public License v3.0 / AGPL v3.0)
- **Suitability**: Strongly recommended if the intention is to ensure that LocardX and all derivative forensic/sanitization enhancements remain freely accessible, verifiable, and open to the community.
- **Key Terms**: Anyone who modifies and distributes the software must also make their complete source code available under the same license terms.
- **Forensic Context**: In legal and forensic proceedings, open-source verification of algorithms (carving logic, sanitization compliance) can enhance evidentiary admissibility.

### 2. Permissive Open Source (e.g., Apache 2.0 / MIT)
- **Suitability**: Recommended if the project aims for widespread adoption and allows third parties or commercial entities to integrate LocardX components (such as the carving or sanitization crates) into proprietary solutions.
- **Key Terms**: Allows anyone to use, modify, and distribute the code for any purpose, with minimal attribution requirements and explicit patent grant protections (in Apache 2.0).

### 3. Dual Licensing / Commercial
- **Suitability**: Recommended if you wish to offer a community edition under an open-source license (such as GPLv3) while providing commercial or enterprise licenses with private customization, enterprise support, and proprietary embedding rights.

---

## Action Required

To finalize licensing:
1. Select one of the candidate licenses (or define custom licensing terms).
2. Copy the chosen license text into `LICENSE` at the repository root.
3. Remove this `LICENSE_SELECTION.md` file or archive it into project documentation.
