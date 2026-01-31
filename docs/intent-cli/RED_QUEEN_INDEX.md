# Red Queen Testing - Complete Index

This directory contains comprehensive security and resilience testing results for Intent CLI.

## 📋 Quick Navigation

### For Developers
- **Start here:** `.red-queen-findings.md` - Quick reference for the 3 critical bugs
- **Test now:** `./REPRODUCE_CRITICAL_FINDINGS.sh` - Verify vulnerabilities

### For Security Review
- **Executive summary:** `STATE_CORRUPTION_SUMMARY.md` - 1-page overview
- **Full report:** `RED_QUEEN_STATE_CORRUPTION_REPORT.md` - Complete technical details

### For Testing
- **Test suite:** `red-queen-state-corruption.sh` - Automated attack scenarios
- **Test logs:** `/tmp/intent-state-corruption-*.log` - Raw test output

---

## 🔴 Critical Findings (Must Fix Before Production)

### 1. Symlink Attack - Arbitrary File Read
- **Severity:** CRITICAL (P0)
- **CWE:** CWE-59
- **Issues:** intent-cli-83rb, intent-cli-x22j, intent-cli-k5qu
- **Impact:** Can read `/etc/passwd`, SSH keys, config files, etc.

### 2. Silent Data Loss on File Corruption
- **Severity:** CRITICAL (P0)
- **CWE:** CWE-755
- **Issue:** intent-cli-4c5t
- **Impact:** 381 sessions hidden, appears as empty list (success response)

### 3. Export Succeeds with Corrupted Data
- **Severity:** HIGH (P2)
- **CWE:** CWE-703
- **Issue:** intent-cli-pn1w
- **Impact:** Generates invalid specs without warning

---

## 📊 Test Results Summary

**Test Date:** 2026-01-30
**Test Duration:** ~4 minutes
**Test Coverage:** 15 attack scenarios

### Results
- **Critical Vulnerabilities:** 3
- **Graceful Handling:** 12
- **Pass Rate:** 80%

### Attack Dimensions Tested
- ✅ JSONL corruption (8 scenarios)
- ✅ Filesystem chaos (3 scenarios)
- ✅ Concurrent access (1 scenario)
- ⚠️ SQLite corruption (1 scenario - N/A, not actively used)

---

## 📁 File Guide

### Documentation
| File | Purpose | Audience |
|------|---------|----------|
| `.red-queen-findings.md` | Quick reference with code snippets | Developers |
| `STATE_CORRUPTION_SUMMARY.md` | Executive summary | Management, Security |
| `RED_QUEEN_STATE_CORRUPTION_REPORT.md` | Full technical report | Security, QA |
| `RED_QUEEN_INDEX.md` | This file | Everyone |

### Scripts
| File | Purpose | Usage |
|------|---------|-------|
| `REPRODUCE_CRITICAL_FINDINGS.sh` | Reproduce 3 critical bugs | `./REPRODUCE_CRITICAL_FINDINGS.sh` |
| `red-queen-state-corruption.sh` | Full test suite (15 tests) | `./red-queen-state-corruption.sh` |

### Logs
| File | Content |
|------|---------|
| `/tmp/intent-state-corruption-*.log` | Raw test execution output |
| `/tmp/test-*.sh` | Individual test artifacts |

---

## 🚀 Quick Start

### Verify Vulnerabilities Exist
```bash
./REPRODUCE_CRITICAL_FINDINGS.sh
```

**Expected output before fixes:**
```
✗ VULNERABLE: CLI followed symlink and read /etc/passwd
✗ VULNERABLE: Silent data loss (381 sessions hidden)
✗ VULNERABLE: Export succeeded with corrupted data
```

### After Implementing Fixes
```bash
./REPRODUCE_CRITICAL_FINDINGS.sh
```

**Expected output after fixes:**
```
✓ FIXED: CLI rejected symlink
✓ FIXED: Corruption detected and reported
✓ FIXED: Export failed with error for corrupted data
```

### Run Full Test Suite
```bash
./red-queen-state-corruption.sh 2>&1 | tee test-run-$(date +%s).log
```

---

## 🔧 Remediation Guide

### Phase 1: Critical Security (Week 1)
1. **Symlink Detection**
   - File: `src/intent/interview_storage.gleam`
   - Add `simplifile.is_symlink()` check before all file reads
   - Return error for symlinks

2. **Corruption Detection**
   - File: `src/intent/interview_storage.gleam:513-558`
   - Change silent skip to logged errors
   - Return error if >0 unparseable lines found

3. **Export Validation**
   - Add session validation before export
   - Fail with exit code 4 if data incomplete

### Phase 2: Robustness (Week 2)
4. Implement atomic writes (temp file + rename)
5. Add comprehensive error logging
6. Add file integrity checks (checksums)

### Phase 3: Testing (Week 3)
7. Unit tests for all corruption scenarios
8. Integration tests for concurrent access
9. Fuzzing for malformed inputs

---

## 📈 Red Queen Metrics

### This Dimension: state-corruption
- **Survivors:** 3 vulnerabilities filed
- **Discards:** 8 graceful behaviors recorded
- **Generations:** 1-9 through 1-24
- **Risk Level:** HIGH

### Future Dimensions
Potential next tests:
- `filesystem-limits` - Max file size, path length, inode exhaustion
- `memory-exhaustion` - Large files, memory leaks, OOM
- `injection-attacks` - Command injection, path traversal
- `race-conditions` - Advanced concurrent scenarios
- `dos-attacks` - Resource exhaustion, infinite loops

---

## 🎯 Success Criteria

Before marking issues as resolved:

1. ✅ Run `./REPRODUCE_CRITICAL_FINDINGS.sh` → All show `✓ FIXED`
2. ✅ Run `./red-queen-state-corruption.sh` → No new CRITICAL findings
3. ✅ Verify Beads issues closed: `bd show intent-cli-83rb intent-cli-4c5t intent-cli-pn1w`
4. ✅ Add regression tests to test suite
5. ✅ Update documentation with security considerations

---

## 📞 Support

**Issues Filed:**
- View all: `bd list | grep "Red Queen"`
- Critical only: `bd list --priority=0 | grep state-corruption`
- Show specific: `bd show intent-cli-83rb`

**Test Logs:**
```bash
ls -lt /tmp/intent-state-corruption-*.log | head -1
```

**Questions:**
See full technical details in `RED_QUEEN_STATE_CORRUPTION_REPORT.md`

---

## 🎓 Red Queen Philosophy

> "Break everything gracefully, or expose what breaks catastrophically."

The Red Queen protocol tests software resilience by:
1. **Survivors** - Vulnerabilities found → Filed as priority issues
2. **Discards** - Graceful handling → Recorded, no action needed
3. **Iterate** - Continue testing until no new survivors

**Goal:** Find vulnerabilities before attackers do, verify graceful degradation.

---

## 📜 Test History

| Date | Dimension | Survivors | Discards | Status |
|------|-----------|-----------|----------|--------|
| 2026-01-30 | state-corruption | 3 | 8 | ✅ Complete |
| TBD | filesystem-limits | - | - | ⏸ Pending |
| TBD | memory-exhaustion | - | - | ⏸ Pending |

---

**Last Updated:** 2026-01-30T02:18:00-06:00
**Next Review:** After P0 issues resolved
**Version:** 1.0
