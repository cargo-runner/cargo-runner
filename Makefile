.PHONY: publish vscode vscode-package

publish:
	@bash scripts/publish.sh $(filter-out $@,$(MAKECMDGOALS))

# Build the VS Code extension (TypeScript → out/extension.js)
vscode:
	cd extensions/vscode && npm install && npm run build

# Package a .vsix (requires vsce)
vscode-package: vscode
	cd extensions/vscode && npm run package

# Catch-all to allow arguments to be passed without make complaining
%:
	@:
