/**
 * Validation for the `cargoRunner.releaseRepo` setting.
 *
 * Free of `vscode` imports so it can be exercised under `node --test`.
 */

/**
 * `owner/repo`, rejecting `.` and `..` as either segment.
 *
 * The value is interpolated into release URLs. URL normalization means a `..`
 * segment could only ever reach another path on the same host — `/`, `@`, `:`
 * and `%` are all outside the character class, so no authority or scheme is
 * expressible — but there is no reason to accept it.
 */
export function isValidRepoSlug(value: string): boolean {
  const parts = value.split("/");
  if (parts.length !== 2) {
    return false;
  }
  return parts.every(
    (p) => /^[A-Za-z0-9._-]+$/.test(p) && p !== "." && p !== "..",
  );
}
