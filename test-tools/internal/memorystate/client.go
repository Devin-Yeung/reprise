package memorystate

import (
	"bytes"
	"encoding/json"
	"fmt"
	"net/http"
	"os"
	"os/exec"
	"strings"
	"syscall"
	"time"
)

const requestTimeout = 5 * time.Second

// Start launches a detached server and returns after its listener is ready.
func Start() (state State, err error) {
	executable, err := os.Executable()
	if err != nil {
		return state, err
	}
	reader, writer, err := os.Pipe()
	if err != nil {
		return state, err
	}
	defer reader.Close()
	defer writer.Close()

	child := exec.Command(executable, "serve")
	child.Env = withEnvironment(os.Environ(), readinessEnv, "1")
	child.ExtraFiles = []*os.File{writer}
	child.SysProcAttr = &syscall.SysProcAttr{Setsid: true}
	// Nil stdio becomes /dev/null; the child never retains Execution output.
	if err = child.Start(); err != nil {
		return state, err
	}
	defer func() {
		if err != nil {
			_ = child.Process.Kill()
			_ = child.Wait()
		}
	}()
	_ = writer.Close()
	if err = reader.SetReadDeadline(time.Now().Add(15 * time.Second)); err != nil {
		return state, err
	}
	if err = json.NewDecoder(reader).Decode(&state); err != nil {
		return state, fmt.Errorf("server readiness: %w", err)
	}
	return state, child.Process.Release()
}

// GetState reads the current state from the local workload server.
func GetState() (State, error) {
	return request(http.MethodGet, "/state", nil)
}

// Mutate sets the current value and returns the resulting state.
func Mutate(value string) (State, error) {
	return request(http.MethodPost, "/mutate", struct {
		Value string `json:"value"`
	}{Value: value})
}

func request(method, path string, body any) (state State, err error) {
	var requestBody *bytes.Reader
	if body != nil {
		encoded, encodeErr := json.Marshal(body)
		if encodeErr != nil {
			return state, encodeErr
		}
		requestBody = bytes.NewReader(encoded)
	} else {
		requestBody = bytes.NewReader(nil)
	}

	req, err := http.NewRequest(method, "http://"+address+path, requestBody)
	if err != nil {
		return state, err
	}
	if body != nil {
		req.Header.Set("Content-Type", "application/json")
	}

	client := &http.Client{Timeout: requestTimeout, Transport: &http.Transport{}}
	response, err := client.Do(req)
	if err != nil {
		return state, err
	}
	defer response.Body.Close()
	if response.StatusCode != http.StatusOK {
		return state, fmt.Errorf("HTTP request failed: %s", response.Status)
	}
	if err := json.NewDecoder(response.Body).Decode(&state); err != nil {
		return state, err
	}
	return state, nil
}

func withEnvironment(environment []string, key, value string) []string {
	prefix := key + "="
	result := make([]string, 0, len(environment)+1)
	for _, entry := range environment {
		if !strings.HasPrefix(entry, prefix) {
			result = append(result, entry)
		}
	}
	return append(result, prefix+value)
}
