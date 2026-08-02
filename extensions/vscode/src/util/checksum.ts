/**
 * SHA256SUMS parsing for release-artifact verification.
 *
 * Split out from the fetching so the matching logic — which decides whether a
 * downloaded binary is executed — can be exercised under `node --test`.
 */

/**
 * Find the expected digest for `assetName` in a `sha256sum`-format manifest.
 *
 * Lines look like `<64-hex>  <basename>`; `*` marks a binary-mode entry and is
 * ignored. Returns `undefined` when the asset is absent, which callers must
 * treat as a failure rather than a pass.
 */
export function digestForAsset(
  manifest: string,
  assetName: string,
): string | undefined {
  for (const line of manifest.split(/\r?\n/)) {
    const m = line.trim().match(/^([0-9a-fA-F]{64})\s+\*?(.+)$/);
    if (!m) {
      continue;
    }
    // Compare by basename: the manifest is generated next to the archives, but
    // a path-shaped entry should not be able to match a different asset.
    const entry = m[2].trim().split(/[/\\]/).pop();
    if (entry === assetName) {
      return m[1].toLowerCase();
    }
  }
  return undefined;
}
