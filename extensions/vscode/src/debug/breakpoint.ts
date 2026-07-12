import * as vscode from "vscode";

/**
 * Find the tightest function/method document symbol containing `position`.
 */
export async function findSymbolAt(
  document: vscode.TextDocument,
  position: vscode.Position,
): Promise<vscode.DocumentSymbol | null> {
  const symbols = await vscode.commands.executeCommand<vscode.DocumentSymbol[]>(
    "vscode.executeDocumentSymbolProvider",
    document.uri,
  );
  if (!symbols?.length) {
    return null;
  }

  const inRange = (pos: vscode.Position, symbol: vscode.DocumentSymbol) =>
    pos.isAfterOrEqual(symbol.range.start) && pos.isBeforeOrEqual(symbol.range.end);

  const walk = (items: vscode.DocumentSymbol[]): vscode.DocumentSymbol | null => {
    let found: vscode.DocumentSymbol | null = null;
    for (const symbol of items) {
      if (!inRange(position, symbol)) {
        continue;
      }
      if (
        symbol.kind === vscode.SymbolKind.Function ||
        symbol.kind === vscode.SymbolKind.Method
      ) {
        return symbol;
      }
      const child = walk(symbol.children);
      if (
        child &&
        (child.kind === vscode.SymbolKind.Function ||
          child.kind === vscode.SymbolKind.Method)
      ) {
        return child;
      }
      found = child || symbol;
    }
    return found;
  };

  return walk(symbols);
}

export function breakpointsInSymbol(
  symbol: vscode.DocumentSymbol,
  document: vscode.TextDocument,
): vscode.SourceBreakpoint[] {
  return vscode.debug.breakpoints.filter((bp): bp is vscode.SourceBreakpoint => {
    if (!(bp instanceof vscode.SourceBreakpoint)) {
      return false;
    }
    if (bp.location.uri.toString() !== document.uri.toString()) {
      return false;
    }
    const line = bp.location.range.start.line;
    return line >= symbol.range.start.line && line <= symbol.range.end.line;
  });
}

/**
 * If breakpoints exist in the function at the cursor, execute rust-analyzer's
 * Debug CodeLens. Returns true when debug was started.
 */
export async function tryDebugAtCursor(
  document: vscode.TextDocument,
  position: vscode.Position,
  output: vscode.OutputChannel,
): Promise<boolean> {
  const config = vscode.workspace.getConfiguration("cargoRunner");
  if (!config.get<boolean>("enableBreakpointDetection", true)) {
    return false;
  }

  const symbol = await findSymbolAt(document, position);
  if (
    !symbol ||
    (symbol.kind !== vscode.SymbolKind.Function &&
      symbol.kind !== vscode.SymbolKind.Method)
  ) {
    output.appendLine("[debug] No function/method at cursor");
    return false;
  }

  const bps = breakpointsInSymbol(symbol, document);
  output.appendLine(
    `[debug] Symbol ${symbol.name}: ${bps.length} breakpoint(s) in range`,
  );
  if (bps.length === 0) {
    return false;
  }

  const lenses =
    (await vscode.commands.executeCommand<vscode.CodeLens[]>(
      "vscode.executeCodeLensProvider",
      document.uri,
    )) || [];

  const debugLens = lenses.find(
    (lens) =>
      lens.command?.title?.toLowerCase().includes("debug") &&
      lens.range.start.line >= symbol.range.start.line - 2 &&
      lens.range.start.line <= symbol.range.end.line,
  );

  if (debugLens?.command) {
    output.appendLine(
      `[debug] Executing ${debugLens.command.command} for ${symbol.name}`,
    );
    await vscode.commands.executeCommand(
      debugLens.command.command,
      ...(debugLens.command.arguments || []),
    );
    vscode.window.showInformationMessage(
      `Debug: ${bps.length} breakpoint(s) in ${symbol.name}`,
    );
    return true;
  }

  // Fallback: known rust-analyzer command ids
  const commands = await vscode.commands.getCommands(true);
  for (const cmd of [
    "rust-analyzer.debug",
    "rust-analyzer.debugSingle",
    "rust-analyzer.runSingle",
  ]) {
    if (commands.includes(cmd)) {
      try {
        await vscode.commands.executeCommand(cmd);
        output.appendLine(`[debug] Executed fallback ${cmd}`);
        return true;
      } catch {
        // try next
      }
    }
  }

  output.appendLine("[debug] No Debug CodeLens; falling back to run");
  return false;
}

export async function showDebugInfo(
  document: vscode.TextDocument,
  position: vscode.Position,
): Promise<void> {
  const symbol = await findSymbolAt(document, position);
  if (!symbol) {
    vscode.window.showInformationMessage("No symbol at cursor");
    return;
  }
  const bps = breakpointsInSymbol(symbol, document);
  const lenses =
    (await vscode.commands.executeCommand<vscode.CodeLens[]>(
      "vscode.executeCodeLensProvider",
      document.uri,
    )) || [];
  const hasDebug = lenses.some((l) =>
    l.command?.title?.toLowerCase().includes("debug"),
  );
  vscode.window.showInformationMessage(
    [
      `Symbol: ${symbol.name} (${vscode.SymbolKind[symbol.kind]})`,
      `Breakpoints in symbol: ${bps.length}`,
      `Debug CodeLens available: ${hasDebug ? "yes" : "no"}`,
      `Cmd+R would: ${bps.length > 0 && hasDebug ? "debug" : "run"}`,
    ].join("\n"),
    { modal: true },
  );
}
