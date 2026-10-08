package memorystate

import (
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net"
	"os"
)

// Listen creates the Unix socket through which short-lived commands reach the
// long-lived serve process. A stale socket can only be left by a previous
// server, so removing any other file type would hide a configuration mistake.
func Listen(socketPath string) (net.Listener, error) {
	if info, err := os.Lstat(socketPath); err == nil {
		if info.Mode()&os.ModeSocket == 0 {
			return nil, fmt.Errorf("refusing to replace non-socket %q", socketPath)
		}
		if err := os.Remove(socketPath); err != nil {
			return nil, err
		}
	} else if !errors.Is(err, os.ErrNotExist) {
		return nil, err
	}

	return net.Listen("unix", socketPath)
}

// Serve keeps one randomly initialized state store alive while accepting
// independent client commands. The initial OCI process calls this function and
// must remain alive; callers use Get and Mutate from later runsc exec processes.
func Serve(listener net.Listener) error {
	store, err := newStateStore()
	if err != nil {
		return err
	}

	for {
		connection, err := listener.Accept()
		if err != nil {
			if errors.Is(err, net.ErrClosed) {
				return nil
			}
			return err
		}
		go serveConnection(connection, store)
	}
}

// Get reads the state owned by the serving process through a one-request
// connection. It does not create a store, so a successful result is evidence
// that the long-lived process was reachable.
func Get(socketPath string) (State, error) {
	return call(socketPath, commandRequest{Command: "get"})
}

// Mutate replaces the value and advances the serving process's revision.
func Mutate(socketPath, value string) (State, error) {
	return call(socketPath, commandRequest{Command: "mutate", Value: &value})
}

type commandRequest struct {
	Command string  `json:"command"`
	Value   *string `json:"value,omitempty"`
}

type commandResponse struct {
	State *State `json:"state,omitempty"`
	Error string `json:"error,omitempty"`
}

func serveConnection(connection net.Conn, store *stateStore) {
	defer connection.Close()

	request, err := decodeRequest(connection)
	if err != nil {
		writeResponse(connection, commandResponse{Error: err.Error()})
		return
	}

	var state State
	switch request.Command {
	case "get":
		if request.Value != nil {
			writeResponse(connection, commandResponse{Error: "get does not accept a value"})
			return
		}
		state = store.snapshot()
	case "mutate":
		if request.Value == nil {
			writeResponse(connection, commandResponse{Error: "mutate requires a value"})
			return
		}
		state = store.mutate(*request.Value)
	default:
		writeResponse(connection, commandResponse{Error: "unknown command"})
		return
	}

	writeResponse(connection, commandResponse{State: &state})
}

func call(socketPath string, request commandRequest) (State, error) {
	connection, err := net.Dial("unix", socketPath)
	if err != nil {
		return State{}, err
	}
	defer connection.Close()

	if err := json.NewEncoder(connection).Encode(request); err != nil {
		return State{}, err
	}

	var response commandResponse
	if err := json.NewDecoder(connection).Decode(&response); err != nil {
		return State{}, err
	}
	if response.Error != "" {
		return State{}, errors.New(response.Error)
	}
	if response.State == nil {
		return State{}, errors.New("server response omitted state")
	}
	return *response.State, nil
}

func decodeRequest(reader io.Reader) (commandRequest, error) {
	// Request and response share one full-duplex connection. Waiting for EOF
	// here would deadlock: the client waits for the response before it closes.
	decoder := json.NewDecoder(reader)
	decoder.DisallowUnknownFields()
	var request commandRequest
	if err := decoder.Decode(&request); err != nil {
		return commandRequest{}, fmt.Errorf("decode request: %w", err)
	}
	return request, nil
}

func writeResponse(writer io.Writer, response commandResponse) {
	_ = json.NewEncoder(writer).Encode(response)
}
