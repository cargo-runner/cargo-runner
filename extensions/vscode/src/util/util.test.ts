import assert from "node:assert/strict";
import { test } from "node:test";

import { digestForAsset } from "./checksum";
import { isDebugCommand, isLongRunning } from "./commands";
import { escapeMarkdown, stripControlChars } from "./markdown";
import { isValidRepoSlug } from "./repo";
import { isNewerSemver, tagToSemver } from "./semver";
import { lookupHelper, rustcTarget } from "./target";

// ── repo slug ────────────────────────────────────────────────────────────────
// This setting selects where a binary that gets chmod +x'd and executed is
// downloaded from.

test("valid owner/repo slugs are accepted", () => {
  for (const v of ["cargo-runner/cargo-runner", "a/b", "Owner.Name/repo_1"]) {
    assert.equal(isValidRepoSlug(v), true, v);
  }
});

test("slugs with traversal segments or wrong shape are rejected", () => {
  for (const v of ["../..", "./.", "a/..", "../b", "a/b/c", "", "x/", "/y"]) {
    assert.equal(isValidRepoSlug(v), false, JSON.stringify(v));
  }
});

// ── checksum manifest ────────────────────────────────────────────────────────
// A miss here means an unverified binary runs, so absence must never look like
// a match.

const DIGEST_A = "a".repeat(64);
const DIGEST_B = "b".repeat(64);
const MANIFEST = [
  `${DIGEST_A}  cargo-runner-cli-x86_64-unknown-linux-gnu-v2.1.4.tar.gz`,
  `${DIGEST_B}  cargo-runner-cli-aarch64-apple-darwin-v2.1.4.tar.gz`,
].join("\n");

test("the digest for the requested asset is found", () => {
  assert.equal(
    digestForAsset(MANIFEST, "cargo-runner-cli-x86_64-unknown-linux-gnu-v2.1.4.tar.gz"),
    DIGEST_A,
  );
});

test("a missing asset yields undefined rather than another asset's digest", () => {
  assert.equal(digestForAsset(MANIFEST, "not-in-manifest.tar.gz"), undefined);
  assert.equal(digestForAsset("", "anything.tar.gz"), undefined);
});

test("binary-mode markers and CRLF manifests parse", () => {
  const crlf = `${DIGEST_A} *only.tar.gz\r\n`;
  assert.equal(digestForAsset(crlf, "only.tar.gz"), DIGEST_A);
});

test("a path-shaped manifest entry matches on basename only", () => {
  const nested = `${DIGEST_A}  ./dist/only.tar.gz`;
  assert.equal(digestForAsset(nested, "only.tar.gz"), DIGEST_A);
  assert.equal(digestForAsset(nested, "dist/only.tar.gz"), undefined);
});

test("malformed lines are skipped, not treated as matches", () => {
  assert.equal(digestForAsset("not a digest  only.tar.gz", "only.tar.gz"), undefined);
  assert.equal(digestForAsset("abc  only.tar.gz", "only.tar.gz"), undefined);
});

// ── semver ───────────────────────────────────────────────────────────────────

test("tags reduce to a semver", () => {
  assert.equal(tagToSemver("cargo-runner-cli-v1.6.2"), "1.6.2");
  assert.equal(tagToSemver("v2.1.4"), "2.1.4");
  assert.equal(tagToSemver("2.1.4-beta.1"), "2.1.4-beta.1");
  assert.equal(tagToSemver("no-version-here"), null);
});

test("semver ordering compares numerically, not lexically", () => {
  assert.equal(isNewerSemver("1.10.0", "1.9.0"), true, "10 > 9 numerically");
  assert.equal(isNewerSemver("2.0.0", "1.999.999"), true);
  assert.equal(isNewerSemver("1.0.0", "1.0.0"), false);
  assert.equal(isNewerSemver("1.0.0", "1.0.1"), false);
});

// ── target / lookup helper ───────────────────────────────────────────────────

test("host targets map to release asset triples", () => {
  assert.equal(rustcTarget("darwin", "arm64"), "aarch64-apple-darwin");
  assert.equal(rustcTarget("linux", "x64"), "x86_64-unknown-linux-gnu");
  assert.equal(rustcTarget("win32", "x64"), "x86_64-pc-windows-msvc");
  assert.throws(() => rustcTarget("sunos" as NodeJS.Platform, "sparc"));
});

test("the Windows lookup helper is absolute, the POSIX one is not", () => {
  // A bare name on Windows resolves against the current directory first, so a
  // repository shipping where.exe would supply the helper itself.
  const win = lookupHelper("win32", "C:\\Windows");
  assert.equal(win, "C:\\Windows\\System32\\where.exe");
  assert.ok(win.includes("System32"));

  // POSIX PATH resolution never consults the cwd, and `which` is not at a
  // fixed location across distributions.
  assert.equal(lookupHelper("linux"), "which");
  assert.equal(lookupHelper("darwin"), "which");
});

// ── markdown / clipboard ─────────────────────────────────────────────────────

test("markdown control characters are escaped", () => {
  assert.equal(escapeMarkdown("a`b"), "a\\`b");
  // `!` is escaped too, which is what stops an image from rendering — the
  // remote-fetch-on-hover case this guards against.
  assert.equal(
    escapeMarkdown("![x](http://evil/i.png)"),
    "\\!\\[x\\]\\(http://evil/i\\.png\\)",
  );
  assert.equal(escapeMarkdown("plain"), "plain");
});

test("clipboard text cannot carry a newline that would self-execute on paste", () => {
  assert.equal(stripControlChars("cargo test\nrm -rf /"), "cargo test rm -rf /");
  assert.equal(stripControlChars("cargo test\r\nid"), "cargo test  id");
  assert.equal(stripControlChars("  cargo test  "), "cargo test");
});

// ── command classification ───────────────────────────────────────────────────

test("debug lenses are matched by command id, not title", () => {
  assert.equal(isDebugCommand("rust-analyzer.debugSingle"), true);
  assert.equal(isDebugCommand("rust-analyzer.debug"), true);
  // Another extension's lens whose title merely contains "debug".
  assert.equal(isDebugCommand("someOther.extension.debugThing"), false);
  assert.equal(isDebugCommand(undefined), false);
});

test("long-running commands are recognised", () => {
  assert.equal(isLongRunning("dx serve --port 8080"), true);
  assert.equal(isLongRunning("cargo leptos watch"), true);
  assert.equal(isLongRunning("cargo test --lib"), false);
});
