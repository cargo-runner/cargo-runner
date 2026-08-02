/**
 * Version comparison for the CLI update check.
 *
 * Free of `vscode` imports so it can be exercised under `node --test`.
 */

/** `cargo-runner-cli-v1.6.2` or `v1.6.2` → `1.6.2` */
export function tagToSemver(tag: string): string | null {
  const m = tag.match(/(\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?)/);
  return m ? m[1] : null;
}

/** True if `a` is a higher semver than `b` (numeric major.minor.patch only). */
export function isNewerSemver(a: string, b: string): boolean {
  const pa = parseSemver(a);
  const pb = parseSemver(b);
  if (!pa || !pb) {
    return a !== b && a > b;
  }
  for (let i = 0; i < 3; i++) {
    if (pa[i] > pb[i]) {
      return true;
    }
    if (pa[i] < pb[i]) {
      return false;
    }
  }
  return false;
}

export function parseSemver(v: string): [number, number, number] | null {
  const m = v.match(/^(\d+)\.(\d+)\.(\d+)/);
  if (!m) {
    return null;
  }
  return [Number(m[1]), Number(m[2]), Number(m[3])];
}
