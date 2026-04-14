LINUX_TARGET   := x86_64-unknown-linux-gnu
WINDOWS_TARGET := x86_64-pc-windows-gnu
MAC_X86_TARGET := x86_64-apple-darwin
MAC_ARM_TARGET := aarch64-apple-darwin

LINUX_BIN   := packaging/server/jira-mcp-server-linux-x86_64
WINDOWS_BIN := packaging/server/jira-mcp-server.exe
MAC_X86_BIN := packaging/server/jira-mcp-server-macos-x86_64
MAC_ARM_BIN := packaging/server/jira-mcp-server-macos-aarch64

MCPB_OUT     := target/jira-mcp-server.mcpb
MACOS_SDK    := /home/vscode/macos-sdk
OSXCROSS_DIR := /opt/osxcross

# Platform selection — written by ./configure, defaults to all platforms
-include config.mk
BUILD_LINUX   ?= 1
BUILD_WINDOWS ?= 1
BUILD_MAC_X86 ?= 1
BUILD_MAC_ARM ?= 1

BUILD_TARGETS :=
RUST_TARGETS  :=
ifeq ($(BUILD_LINUX),1)
BUILD_TARGETS += linux
RUST_TARGETS  += $(LINUX_TARGET)
endif
ifeq ($(BUILD_WINDOWS),1)
BUILD_TARGETS += windows
RUST_TARGETS  += $(WINDOWS_TARGET)
endif
ifeq ($(BUILD_MAC_X86),1)
BUILD_TARGETS += mac-x86
RUST_TARGETS  += $(MAC_X86_TARGET)
endif
ifeq ($(BUILD_MAC_ARM),1)
BUILD_TARGETS += mac-arm
RUST_TARGETS  += $(MAC_ARM_TARGET)
endif

.PHONY: all linux windows mac-x86 mac-arm setup pack clean

all: pack

setup:
	@if [ -n "$(RUST_TARGETS)" ]; then \
		rustup target add $(RUST_TARGETS); \
	fi

linux: setup
	cargo build --release --target $(LINUX_TARGET)
	cp target/$(LINUX_TARGET)/release/jira_mcp $(LINUX_BIN)

windows: setup
	cargo build --release --target $(WINDOWS_TARGET)
	cp target/$(WINDOWS_TARGET)/release/jira_mcp.exe $(WINDOWS_BIN)

mac-x86: setup
	PATH=$(OSXCROSS_DIR)/bin:$$PATH \
	CARGO_TARGET_X86_64_APPLE_DARWIN_LINKER=o64-clang \
	CC_x86_64_apple_darwin=o64-clang \
	CXX_x86_64_apple_darwin=o64-clang++ \
	AR_x86_64_apple_darwin=x86_64-apple-darwin20.4-ar \
		cargo build --target $(MAC_X86_TARGET) --release
	cp target/$(MAC_X86_TARGET)/release/jira_mcp $(MAC_X86_BIN)

mac-arm: setup
	PATH=$(OSXCROSS_DIR)/bin:$$PATH \
	CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER=oa64-clang \
	CC_aarch64_apple_darwin=oa64-clang \
	CXX_aarch64_apple_darwin=oa64-clang++ \
	AR_aarch64_apple_darwin=aarch64-apple-darwin20.4-ar \
		cargo build --target $(MAC_ARM_TARGET) --release
	cp target/$(MAC_ARM_TARGET)/release/jira_mcp $(MAC_ARM_BIN)

pack: $(BUILD_TARGETS)
	mcpb pack packaging $(MCPB_OUT)

clean:
	cargo clean
	rm -f $(LINUX_BIN) $(WINDOWS_BIN) $(MAC_X86_BIN) $(MAC_ARM_BIN) $(MCPB_OUT)
