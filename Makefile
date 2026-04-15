.PHONY: publish

publish:
	@bash scripts/publish.sh $(filter-out $@,$(MAKECMDGOALS))

# Catch-all to allow arguments to be passed without make complaining
%:
	@:
