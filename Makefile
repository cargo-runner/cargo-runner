.PHONY: publish release-cli release-vscode release-major release vscode vscode-package

# Legacy alias → new release script (patch / CLI-only)
publish:
	@bash scripts/release.sh cli $(filter-out $@,$(MAKECMDGOALS))

# Prefer these:
#   make release-cli              # patch (CLI/core only)
#   make release-vscode           # minor (product / VS Code)
#   make release-vscode MARKETPLACE=1
#   make release VERSION=1.7.0
release-cli:
	@bash scripts/release.sh cli $(if $(DRY_RUN),--dry-run,) $(if $(NO_CRATES),--no-crates,) $(if $(MARKETPLACE),--marketplace,)

release-vscode:
	@bash scripts/release.sh vscode $(if $(DRY_RUN),--dry-run,) $(if $(NO_CRATES),--no-crates,) $(if $(MARKETPLACE),--marketplace,)

release-major:
	@bash scripts/release.sh major $(if $(DRY_RUN),--dry-run,) $(if $(NO_CRATES),--no-crates,) $(if $(MARKETPLACE),--marketplace,)

release:
	@bash scripts/release.sh $(VERSION) $(if $(DRY_RUN),--dry-run,) $(if $(NO_CRATES),--no-crates,) $(if $(MARKETPLACE),--marketplace,)

# Build the VS Code extension (TypeScript → out/extension.js)
vscode:
	cd extensions/vscode && npm install && npm run build

# Package a .vsix (requires vsce)
vscode-package: vscode
	cd extensions/vscode && npm run package

# Catch-all to allow arguments to be passed without make complaining
%:
	@:
