package memorystate

import (
	"net"
	"os"
	"path/filepath"
	"testing"
)

func TestUnixSocketServerPreservesOneInMemoryStateAcrossCommands(t *testing.T) {
	// The serve process owns the state. Each client command is deliberately a
	// fresh connection, matching the runsc exec calls that will query it before
	// and after a checkpoint restore.
	// Unix socket paths are short on macOS, so avoid t.TempDir's descriptive
	// per-test path while retaining an isolated directory and cleanup.
	directory, err := os.MkdirTemp("/tmp", "memory-state-")
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { _ = os.RemoveAll(directory) })
	socketPath := filepath.Join(directory, "state.sock")
	listener, err := net.Listen("unix", socketPath)
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() {
		if err := listener.Close(); err != nil {
			t.Errorf("close listener: %v", err)
		}
	})

	go func() {
		if err := Serve(listener); err != nil {
			t.Errorf("serve: %v", err)
		}
	}()

	initial, err := Get(socketPath)
	if err != nil {
		t.Fatal(err)
	}
	if len(initial.BootNonce) != 64 || initial.Value != nil || initial.Revision != 0 {
		t.Fatalf("unexpected initial state: %#v", initial)
	}

	mutated, err := Mutate(socketPath, "fresh")
	if err != nil {
		t.Fatal(err)
	}
	if mutated.BootNonce != initial.BootNonce || mutated.Value == nil || *mutated.Value != "fresh" || mutated.Revision != 1 {
		t.Fatalf("unexpected mutation response: %#v", mutated)
	}

	read, err := Get(socketPath)
	if err != nil {
		t.Fatal(err)
	}
	if read.BootNonce != mutated.BootNonce || read.Value == nil || mutated.Value == nil || *read.Value != *mutated.Value || read.Revision != mutated.Revision {
		t.Fatalf("read = %#v, want %#v", read, mutated)
	}
}
