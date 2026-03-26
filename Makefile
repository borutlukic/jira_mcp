LINUX_TARGET  := x86_64-unknown-linux-gnu
WINDOWS_TARGET := x86_64-pc-windows-gnu

LINUX_BIN  := packaging/server/jira-mcp-server-linux-x86_64
WINDOWS_BIN := packaging/server/jira-mcp-server.exe

MCPB_OUT  := target/jira-mcp-server.mcpb

.PHONY: all linux windows setup pack clean

all: linux windows pack

setup:
	rustup target add $(LINUX_TARGET) $(WINDOWS_TARGET)

linux: setup
	cargo build --release --target $(LINUX_TARGET)
	cp target/$(LINUX_TARGET)/release/jira_mcp $(LINUX_BIN)

windows: setup
	cargo build --release --target $(WINDOWS_TARGET)
	cp target/$(WINDOWS_TARGET)/release/jira_mcp.exe $(WINDOWS_BIN)

pack: linux windows
	mcpb pack packaging $(MCPB_OUT)

clean:
	cargo clean
	rm -f $(LINUX_BIN) $(WINDOWS_BIN) $(MCPB_OUT)
