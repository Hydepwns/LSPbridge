# Makefile for LSP Bridge Neovim plugin

PLUGIN_NAME = lsp-bridge.nvim
INSTALL_DIR = ~/.local/share/nvim/site/pack/lsp-bridge/start/$(PLUGIN_NAME)

.PHONY: install uninstall dev test

install:
	@echo "Installing LSP Bridge for Neovim..."
	@mkdir -p $(INSTALL_DIR)
	@cp -r lua $(INSTALL_DIR)/
	@cp -r plugin $(INSTALL_DIR)/
	@cp README.md $(INSTALL_DIR)/
	@echo "Installation complete!"
	@echo "Add 'require(\"lsp-bridge\").setup()' to your init.lua"

uninstall:
	@echo "Uninstalling LSP Bridge for Neovim..."
	@rm -rf $(INSTALL_DIR)
	@echo "Uninstallation complete!"

dev:
	@echo "Creating symlink for development..."
	@mkdir -p $(dir $(INSTALL_DIR))
	@ln -sf $(PWD) $(INSTALL_DIR)
	@echo "Development symlink created!"

test:
	@echo "Running tests..."
	@nvim --headless -c "PlenaryBustedDirectory tests/ {minimal_init = 'tests/minimal_init.vim'}" -c "qa!"